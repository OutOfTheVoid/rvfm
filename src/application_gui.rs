use winit::{self, dpi::PhysicalSize, event::{Event, WindowEvent, VirtualKeyCode}, event_loop::{
		EventLoop,
		ControlFlow,
		EventLoopProxy
	}, window::{
		WindowBuilder
	}};
	
use crate::{application_core::ApplicationCore, fm_interrupt_bus::FmInterruptBus, fm_mio::FmMemoryIO, gpu, input::{InputEventSink, InputPeripheral}, sound_out::SoundOutPeripheral};
use rv_vsys::CpuWakeupHandle;

use std::{sync::mpsc, sync::mpsc::{TryRecvError, Sender, Receiver}, thread};

#[allow(dead_code)]
pub struct ApplicationGUI {
	inbox: Sender<ApplicationGuiControlMessage>,
	outbox: Receiver<ApplicationGuiEventMessage>,
	loop_proxy: EventLoopProxy<()>,
}

#[allow(dead_code)]
enum ApplicationGuiControlMessage {
	SetMIO(FmMemoryIO),
	SetFramebufferAddress(u32)
}

#[allow(dead_code)]
enum ApplicationGuiEventMessage {
	Close,
}

impl ApplicationGUI {
	pub fn run(screen_scale: u32) {
		let (_gui_outbox, logic_inbox) = mpsc::channel();
		let (logic_outbox, gui_inbox) = mpsc::channel();
		let event_loop = EventLoop::new();
		let window = WindowBuilder::new()
			.with_title("FunRisc Virtual Console")
			.with_inner_size(PhysicalSize::new(gpu::GPU_OUTPUT_W * screen_scale, gpu::GPU_OUTPUT_H * screen_scale))
			.with_resizable(false)
			.with_visible(true)
			.build(&event_loop).unwrap();
		let cpu0_wakeup = CpuWakeupHandle::new();
		let cpu1_wakeup = CpuWakeupHandle::new();
		let mut interrupt_bus = FmInterruptBus::new();
		let logic_interrupt_bus = interrupt_bus.clone();
		let mut mio = FmMemoryIO::new(interrupt_bus.clone());
		let mut input_sink = InputPeripheral::new(&mut mio);
		let logic_mio = mio.clone();
		let (gpu, mut gpu_event_sink, gpu_reset_handle) = futures::executor::block_on(gpu::Gpu::new(&window, &mut mio, &mut interrupt_bus, cpu0_wakeup.clone(), screen_scale));
		let _application_gui = ApplicationGUI {
			inbox: logic_outbox,
			outbox: logic_inbox,
			loop_proxy: event_loop.create_proxy(),
		};
		gpu.run();
		let _logic_thread = thread::spawn(move || {
			// start sound device from non-main thread to support winit/windows
			SoundOutPeripheral::new(cpu1_wakeup.clone(), &mut interrupt_bus, &mut mio, None, None).unwrap();
			let app_core = ApplicationCore::new(logic_mio, logic_interrupt_bus, cpu0_wakeup, cpu1_wakeup, gpu_reset_handle);
			app_core.run();
		});
		event_loop.run(move |event, _, control_flow| {
			match event {
				Event::MainEventsCleared => {
					// redraw
					window.request_redraw();
					*control_flow = ControlFlow::Poll;
				},
				Event::RedrawRequested(_) => {
					gpu_event_sink.render_event();
					*control_flow = ControlFlow::Poll;
				}
				Event::WindowEvent{event: WindowEvent::CloseRequested, ..} => {
					*control_flow = ControlFlow::Exit;
				},
				Event::WindowEvent{event: WindowEvent::KeyboardInput{input, ..}, ..} => {
					if let Some(vkey) = input.virtual_keycode {
						let down = match input.state {
							winit::event::ElementState::Pressed => true,
							winit::event::ElementState::Released => false
						};
						input_sink.vkey_event(vkey, down);
					}
				},
				Event::WindowEvent{event: WindowEvent::CursorMoved{position, ..}, ..} => {
					let x = (position.x as u32) / screen_scale;
					let y = (position.y as u32) / screen_scale;
					input_sink.mouse_move_event(x, y);
				},
				Event::WindowEvent{event: WindowEvent::CursorEntered{..}, ..} => {
					
				},
				Event::WindowEvent{event: WindowEvent::CursorLeft{..}, ..} => {
					
				},
				Event::WindowEvent{event: WindowEvent::MouseInput{button, state, ..}, ..} => {
					let down = match state {
						winit::event::ElementState::Pressed => true,
						winit::event::ElementState::Released => false
					};
					input_sink.mouse_button_event(button, down);
				},
				Event::UserEvent(()) => {
					while match gui_inbox.try_recv() {
						Ok(control_message) => {
							match control_message {
								_ => {}
							}
							true
						},
						Err(TryRecvError::Disconnected) => {
							*control_flow = ControlFlow::Exit;
							false
						},
						Err(TryRecvError::Empty) => {
							false
						}
						
					}{}
					*control_flow = ControlFlow::Poll;
				},
				_ => {}
			}
		});
	}
}
