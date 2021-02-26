use gpu::GPU_SCREENWIN_SCALE;
use winit::{
	event::{
		Event,
		WindowEvent
	},
	dpi::PhysicalSize,
	self,
	event_loop::{
		EventLoop,
		ControlFlow,
		EventLoopProxy
	},
	window::{
		self,
		Window,
		WindowBuilder
	}
};

use crate::{application_core::ApplicationCore, fm_mio::FmMemoryIO, fm_interrupt_bus::FmInterruptBus, gpu};
use rv_vsys::CpuWakeupHandle;

use std::{
	thread,
	sync::mpsc,
	sync::mpsc::{channel, TryRecvError, Sender, Receiver},
};

pub struct ApplicationGUI {
	inbox: Sender<ApplicationGuiControlMessage>,
	outbox: Receiver<ApplicationGuiEventMessage>,
	loop_proxy: EventLoopProxy<()>,
}

enum ApplicationGuiControlMessage {
	SetMIO(FmMemoryIO),
	SetFramebufferAddress(u32)
}

enum ApplicationGuiEventMessage {
	Close,
	
}

impl ApplicationGUI {
	pub fn run() {
		let (gui_outbox, logic_inbox) = mpsc::channel();
		let (logic_outbox, gui_inbox) = mpsc::channel();
		let event_loop = EventLoop::new();
		let window = WindowBuilder::new()
			.with_title("FunRisc Virtual Console")
			.with_inner_size(PhysicalSize::new(gpu::GPU_OUTPUT_W * gpu::GPU_SCREENWIN_SCALE, gpu::GPU_OUTPUT_H * gpu::GPU_SCREENWIN_SCALE))
			.with_resizable(false)
			.with_visible(true)
			.build(&event_loop).unwrap();
		let cpu_wakeup = CpuWakeupHandle::new();
		let mut mio = FmMemoryIO::new();
		let logic_mio = mio.clone();
		let mut interrupt_bus = FmInterruptBus::new();
		let logic_interrupt_bus = interrupt_bus.clone();
		let mut gpu = futures::executor::block_on(gpu::Gpu::new(&window, &mut mio, &mut interrupt_bus, cpu_wakeup.clone()));
		let application_gui = ApplicationGUI {
			inbox: logic_outbox,
			outbox: logic_inbox,
			loop_proxy: event_loop.create_proxy(),
		};
		let logic_thread = thread::spawn(move || {
			let mut app_core = ApplicationCore::new(application_gui, logic_mio, logic_interrupt_bus, cpu_wakeup);
			app_core.run();
		});
		event_loop.run(move |event, _, control_flow| {
			let mut mem: Option<FmMemoryIO> = None;
			let mut fb_addr: Option<u32> = None;
			match event {
				Event::MainEventsCleared => {
					// redraw
					window.request_redraw();
				},
				Event::RedrawRequested(_) => {
					gpu.render();
				}
				Event::WindowEvent{event: WindowEvent::CloseRequested, ..} => {
					*control_flow = ControlFlow::Exit;
				},
				Event::UserEvent(()) => {
					while match gui_inbox.try_recv() {
						Ok(control_message) => {
							match control_message {
								ApplicationGuiControlMessage::SetMIO(mio) => {
									mem = Some(mio);
								},
								ApplicationGuiControlMessage::SetFramebufferAddress(address) => {
									fb_addr = Some(address);
								}
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
				},
				_ => {
					*control_flow = ControlFlow::Poll;
				}
			}
		});
	}
	
	pub fn set_mio(&mut self, mio: FmMemoryIO) {
		self.inbox.send(ApplicationGuiControlMessage::SetMIO(mio)).unwrap();
	}
}
