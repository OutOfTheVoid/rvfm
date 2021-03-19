use winit::{self, dpi::PhysicalSize, event::{Event, WindowEvent}, event_loop::{
		EventLoop,
		ControlFlow,
		EventLoopProxy
	}, window::{
		WindowBuilder
	}};
	
use crate::{application_core::ApplicationCore, fm_interrupt_bus::FmInterruptBus, fm_mio::FmMemoryIO, gpu, sound_device::SoundDevice};
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
	pub fn run() {
		let (_gui_outbox, logic_inbox) = mpsc::channel();
		let (logic_outbox, gui_inbox) = mpsc::channel();
		let event_loop = EventLoop::new();
		let window = WindowBuilder::new()
			.with_title("FunRisc Virtual Console")
			.with_inner_size(PhysicalSize::new(gpu::GPU_OUTPUT_W * gpu::GPU_SCREENWIN_SCALE, gpu::GPU_OUTPUT_H * gpu::GPU_SCREENWIN_SCALE))
			.with_resizable(false)
			.with_visible(true)
			.build(&event_loop).unwrap();
		let cpu0_wakeup = CpuWakeupHandle::new();
		let cpu1_wakeup = CpuWakeupHandle::new();
		let mut interrupt_bus = FmInterruptBus::new();
		let logic_interrupt_bus = interrupt_bus.clone();
		let mut mio = FmMemoryIO::new(interrupt_bus.clone());
		let logic_mio = mio.clone();
		let (gpu, mut gpu_event_sink) = futures::executor::block_on(gpu::Gpu::new(&window, &mut mio, &mut interrupt_bus, cpu0_wakeup.clone()));
		let _application_gui = ApplicationGUI {
			inbox: logic_outbox,
			outbox: logic_inbox,
			loop_proxy: event_loop.create_proxy(),
		};
		gpu.run();
		let _logic_thread = thread::spawn(move || {
			// start sound device from non-main thread to support winit/windows
			let sound_device = SoundDevice::new(None, cpu1_wakeup.clone(), &mut interrupt_bus).unwrap();
			mio.set_sound_device(sound_device);
			let app_core = ApplicationCore::new(logic_mio, logic_interrupt_bus, cpu0_wakeup, cpu1_wakeup);
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
				_ => {
					*control_flow = ControlFlow::Poll;
				}
			}
		});
	}
}
