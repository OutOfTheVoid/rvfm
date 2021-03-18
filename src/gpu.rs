use std::{borrow::BorrowMut, sync::{Arc, atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering}, mpsc}};
use std::thread;

use parking_lot::Mutex;
use wgpu;
use winit::window::Window;

use crate::{fm_mio::FmMemoryIO, raw_fb_renderer::RawFBRenderer, fm_interrupt_bus::FmInterruptBus, fb_present_renderer::FramebufferPresentRenderer};
use rv_vsys::{CpuWakeupHandle, MemIO, MemWriteResult};

pub struct Gpu {
	device: Arc<wgpu::Device>,
	queue: Arc<wgpu::Queue>,
	present_chain: GpuPresentChain,
	current_present_fb: Option<wgpu::Texture>,
	mio: FmMemoryIO,
	mode: Mode,
	cmd_queue: mpsc::Receiver<Command>,
	raw_fb_renderer: Option<RawFBRenderer>,
	raw_fb_base_addr: Arc<AtomicU32>,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
	Disabled,
	RawFBDisplay,
}

pub enum Command {
	SetMode(Mode),
	PresentMMFB,
	SetMMFBBase(u32)
}

pub const GPU_OUTPUT_W: u32 = 256;
pub const GPU_OUTPUT_H: u32 = 192;
pub const GPU_OUTPUT_FB_SIZE: u32 = GPU_OUTPUT_W * GPU_OUTPUT_H * 4;
pub const GPU_SCREENWIN_SCALE: u32 = 4;

pub struct GpuWindowEventSink {
	last_present_tex: Option<wgpu::Texture>,
	swap_chain: wgpu::SwapChain,
	device: Arc<wgpu::Device>,
	queue: Arc<wgpu::Queue>,
	present_chain: GpuPresentChain,
	present_renderer: FramebufferPresentRenderer,
	present_counter: Arc<AtomicUsize>,
	cpu_wakeup: CpuWakeupHandle,
	sync_interrupt_enable: Arc<AtomicBool>
}

const DEFAULT_CLEAR_COLOR: wgpu::Color = wgpu::Color {r: 0.0, g: 0.0, b: 0.1, a: 1.0};

impl GpuWindowEventSink {
	pub fn render_event(&mut self) {
		self.present_counter.fetch_add(1, Ordering::SeqCst);
		if self.sync_interrupt_enable.load(Ordering::SeqCst) {
			self.cpu_wakeup.cpu_wake();
		}
		let mut last_swap = None;
		std::mem::swap(&mut self.last_present_tex, &mut last_swap);
		match self.present_chain.present_swap(last_swap) {
			Some(texture) => {
				let mut command_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
					label: Some("GpuWindowEventSink::render_event()")
				});
				let framebuffer = self.swap_chain.get_current_frame().unwrap().output;
				self.present_renderer.render(&*self.device, &mut command_encoder, &framebuffer.view, &texture);
				self.queue.submit(Some(command_encoder.finish()));
				self.last_present_tex = Some(texture);
			},
			None => {
				println!("none");
				let mut command_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
					label: Some("GpuWindowEventSink::render_event()")
				});
				let framebuffer = self.swap_chain.get_current_frame().unwrap().output;
				{
					let mut _render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
						color_attachments: &[
							wgpu::RenderPassColorAttachmentDescriptor {
								attachment: &framebuffer.view,
								resolve_target: None,
								ops: wgpu::Operations {
									load: wgpu::LoadOp::Clear(DEFAULT_CLEAR_COLOR),
									store: true
								}
							}
						],
						depth_stencil_attachment: None,
					});
				}
				self.queue.submit(Some(command_encoder.finish()));
			}
		}
	}
}

enum GpuPresentState {
	None,
	Presenting(wgpu::Texture),
	Free(wgpu::Texture),
}

#[derive(Clone)]
pub struct GpuPresentChain {
	texture_counter: Arc<AtomicUsize>,
	chain: Arc<Mutex<GpuPresentState>>
}

impl GpuPresentChain {
	pub fn new() -> Self {
		Self {
			texture_counter: Arc::new(AtomicUsize::new(0)),
			chain: Arc::new(Mutex::new(GpuPresentState::None))
		}
	}
	
	pub fn present_swap(&mut self, texture: Option<wgpu::Texture>) -> Option<wgpu::Texture> {
		let mut lock_gaurd = self.chain.lock();
		let mut swap_state = match texture {
			Some(tex) => {
				match &*lock_gaurd {
					GpuPresentState::Free(_) => {
						return Some(tex);
					},
					GpuPresentState::None => {
						return Some(tex);
					},
					_ => {}
				}
				GpuPresentState::Free(tex)
			},
			None => GpuPresentState::None
		};
		std::mem::swap(&mut swap_state, &mut *lock_gaurd);
		match swap_state {
			GpuPresentState::None => None,
			GpuPresentState::Presenting(texture) => Some(texture),
			GpuPresentState::Free(texture) => {
				*lock_gaurd = GpuPresentState::Free(texture);
				None
			},
		}
	}
	
	pub fn gpu_swap(&mut self, texture: Option<wgpu::Texture>, device: &wgpu::Device) -> wgpu::Texture{
		let swap_result = {
			let mut lock_gaurd = self.chain.lock();
			let mut swap_state = match texture {
				Some(texture) => GpuPresentState::Presenting(texture),
				None => GpuPresentState::None,
			};
			let chain = lock_gaurd.borrow_mut();
			std::mem::swap(&mut swap_state, chain);
			match swap_state {
				GpuPresentState::None => None,
				GpuPresentState::Presenting(texture) => Some(texture),
				GpuPresentState::Free(texture) => Some(texture)
			}
		};
		match swap_result {
			Some(texture) => texture,
			None => self.make_swap_texture(device)
		}
	}
	
	fn make_swap_texture(&mut self, device: &wgpu::Device) -> wgpu::Texture {
		let id = self.texture_counter.fetch_add(1, Ordering::SeqCst);
		device.create_texture(&wgpu::TextureDescriptor {
			label: Some(format!("Gpu swap texture {}", id).as_str()),
			size: wgpu::Extent3d {
				width: GPU_OUTPUT_W,
				height: GPU_OUTPUT_H,
				depth: 1
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: wgpu::TextureFormat::Rgba8UnormSrgb,
			usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::OUTPUT_ATTACHMENT,
		})
	}
}

impl Gpu {
	pub async fn new(window: &Window, mio: &mut FmMemoryIO, int_bus: &mut FmInterruptBus, cpu_wakeup: CpuWakeupHandle) -> (Self, GpuWindowEventSink) {
		let (cmd_queue_tx, cmd_queue_rx) = mpsc::channel();
		let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
		let surface = unsafe {
			instance.create_surface(window)
		};
		let adapter = instance.request_adapter(
			&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::Default,
				compatible_surface: Some(&surface)
			}
		).await.unwrap();
		let (device, queue) = adapter.request_device(
			&wgpu::DeviceDescriptor {
				features: wgpu::Features::empty(),
				limits: wgpu::Limits::default(),
				shader_validation: true,
			},
			None,
		).await.unwrap();
		let device = Arc::new(device);
		let queue = Arc::new(queue);
		let swap_desc = wgpu::SwapChainDescriptor {
			usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: GPU_OUTPUT_W * GPU_SCREENWIN_SCALE,
            height: GPU_OUTPUT_H * GPU_SCREENWIN_SCALE,
            present_mode: wgpu::PresentMode::Fifo,
		};
		let swap_chain = device.create_swap_chain(&surface, &swap_desc);
		let sync_interrupt_enable = Arc::new(AtomicBool::new(false));
		mio.set_gpu_interface(GpuPeripheralInterface::new(cmd_queue_tx, sync_interrupt_enable.clone()));
		let present_counter = Arc::new(AtomicUsize::new(1));
		let interrupt_output = GpuInterruptOutput::new(
			present_counter.clone(), 
			sync_interrupt_enable.clone()
		);
		let present_renderer = FramebufferPresentRenderer::new(&*device, &swap_desc).unwrap();
		let present_chain = GpuPresentChain::new();
		int_bus.set_gpu_interrupts(interrupt_output);
		let raw_fb_base_addr = Arc::new(AtomicU32::new(0x0200_0000));
		(Gpu {
			device: device.clone(),
			queue: queue.clone(),
			present_chain: present_chain.clone(),
			current_present_fb: None,
			mio: mio.clone(),
			mode: Mode::Disabled,
			cmd_queue: cmd_queue_rx,
			raw_fb_renderer: None,
			raw_fb_base_addr,
		},
		GpuWindowEventSink {
			last_present_tex: None,
			device: device,
			queue: queue,
			swap_chain: swap_chain,
			present_chain: present_chain,
			present_renderer: present_renderer,
			present_counter: present_counter,
			cpu_wakeup: cpu_wakeup,
			sync_interrupt_enable: sync_interrupt_enable
		})
	}
	
	pub fn run(mut self) {
		thread::spawn(move || {
			self.run_thread();
		});
	}
	
	pub fn run_thread(&mut self) {
		self.swap_fb();
		self.clear_display();
		loop {
			match self.cmd_queue.recv().unwrap() {
				Command::SetMode(mode) => {
					self.set_mode(mode);
				},
				Command::PresentMMFB => {
					self.present_mmfb();
				},
				Command::SetMMFBBase(base_address) => {
					self.raw_fb_base_addr.store(base_address, Ordering::SeqCst);
				}
			}
		}
	}
	
	fn swap_fb (&mut self) {
		let mut fb_current = None;
		std::mem::swap(&mut fb_current, &mut self.current_present_fb);
		let mut fb_current = Some(self.present_chain.gpu_swap(fb_current, &*self.device));
		std::mem::swap(&mut fb_current, &mut self.current_present_fb);
	}
	
	fn set_mode(&mut self, mode: Mode) {
		if self.mode != mode {
			match self.mode {
				Mode::Disabled => {},
				Mode::RawFBDisplay => {
					self.raw_fb_renderer = None;
				},
			}
			self.mode = mode;
			match mode {
				Mode::Disabled => {
					self.clear_display();
					self.swap_fb();
				}
				Mode::RawFBDisplay => {
					self.raw_fb_renderer = Some(RawFBRenderer::new(&self.device, self.raw_fb_base_addr.clone()).unwrap());
				}
			}
		}
	}
	
	fn clear_display(&mut self) {
		let framebuffer = self.current_present_fb.as_mut().unwrap();
		let fb_view = framebuffer.create_view(&wgpu::TextureViewDescriptor {
			label: Some("fb draw view"),
			dimension: Some(wgpu::TextureViewDimension::D2),
			format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
			aspect: wgpu::TextureAspect::All,
			base_mip_level: 0,
			level_count: None,
			base_array_layer: 0,
			array_layer_count: None,
		});
		let mut command_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
			label: Some("Gpu::clear_display()")
		});
		{
			let mut _render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				color_attachments: &[
					wgpu::RenderPassColorAttachmentDescriptor {
						attachment: &fb_view,
						resolve_target: None,
						ops: wgpu::Operations {
							load: wgpu::LoadOp::Clear(DEFAULT_CLEAR_COLOR),
							store: true
						}
					}
				],
				depth_stencil_attachment: None,
			});
		}
		self.queue.submit(Some(command_encoder.finish()));
		self.swap_fb();
	}
	
	fn present_mmfb(&mut self) {
		let framebuffer = self.current_present_fb.as_mut().unwrap();
		let fb_view = framebuffer.create_view(&wgpu::TextureViewDescriptor {
			label: Some("fb draw view"),
			dimension: Some(wgpu::TextureViewDimension::D2),
			format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
			aspect: wgpu::TextureAspect::All,
			base_mip_level: 0,
			level_count: None,
			base_array_layer: 0,
			array_layer_count: None,
		});
		let mut command_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
			label: Some("Gpu::present_mmfb")
		});
		match &mut self.raw_fb_renderer {
			Some(renderer) => {
				renderer.render(&mut self.mio, &self.queue, &mut command_encoder, &fb_view);
				self.mio.access_break();
			},
			None => {}
		}
		self.queue.submit(Some(command_encoder.finish()));
		self.swap_fb();
	}
}

#[derive(Clone, Debug)]
pub struct GpuPeripheralInterface {
	cmd_queue: mpsc::Sender<Command>,
	sync_interrupt_enable: Arc<AtomicBool>
}

pub const GPU_REGISTER_MODE: u32 = 0;
pub const GPU_MODE_VALUE_DISABLED: u32 = 0;
pub const GPU_MODE_VALUE_RAW_FB: u32 = 1;

pub const GPU_REGISTER_PRESENT_MMFB: u32 = 4;

pub const GPU_REGISTER_SYNC_INT_ENABLE: u32 = 8;

pub const GPU_REGISTER_MMFB_BASE: u32 = 12;

impl GpuPeripheralInterface {
	pub fn new(cmd_queue: mpsc::Sender<Command>, sync_interrupt_enable: Arc<AtomicBool>) -> Self {
		Self {
			cmd_queue,
			sync_interrupt_enable
		}
	}
	
	pub fn write_u32(&mut self, offset: u32, value: u32) -> MemWriteResult {
		match offset {
			GPU_REGISTER_MODE => {
				match value {
					GPU_MODE_VALUE_DISABLED => {
						self.cmd_queue.send(Command::SetMode(Mode::Disabled)).unwrap();
						MemWriteResult::Ok
					},
					GPU_MODE_VALUE_RAW_FB => {
						self.cmd_queue.send(Command::SetMode(Mode::RawFBDisplay)).unwrap();
						MemWriteResult::Ok
					},
					_ => MemWriteResult::PeripheralError
				}
			},
			GPU_REGISTER_PRESENT_MMFB => {
				self.cmd_queue.send(Command::PresentMMFB).unwrap();
				MemWriteResult::Ok
			},
			GPU_REGISTER_SYNC_INT_ENABLE => {
				self.sync_interrupt_enable.store(value != 0, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			GPU_REGISTER_MMFB_BASE => {
				if value & 0x03 != 0 {
					MemWriteResult::PeripheralError
				} else {
					self.cmd_queue.send(Command::SetMMFBBase(value)).unwrap();
					MemWriteResult::Ok
				}
			},
			_ => MemWriteResult::ErrUnmapped
		}
	}
}

#[derive(Debug, Clone)]
pub struct GpuInterruptOutput {
	cpu_frame: Arc<AtomicUsize>,
	gpu_frame: Arc<AtomicUsize>,
	sync_interrupt_state: Arc<AtomicBool>,
	sync_interrupt_enable: Arc<AtomicBool>
}

impl GpuInterruptOutput {
	pub fn new(gpu_frame: Arc<AtomicUsize>, sync_interrupt_enable: Arc<AtomicBool>) -> Self {
		Self {
			cpu_frame: Arc::new(AtomicUsize::new(0)),
			gpu_frame,
			sync_interrupt_state: Arc::new(AtomicBool::new(false)),
			sync_interrupt_enable
		}
	}
	
	pub fn poll_sync_interrupt(&mut self) -> bool {
		if ! self.sync_interrupt_enable.load(Ordering::SeqCst) {
			self.sync_interrupt_state.store(false, Ordering::SeqCst);
			return false
		}
		let gpu_frame_num = self.gpu_frame.load(Ordering::SeqCst);
		let active = self.cpu_frame.load(Ordering::SeqCst) < gpu_frame_num;
		self.cpu_frame.store(gpu_frame_num, Ordering::SeqCst);
		self.sync_interrupt_state.fetch_or(active, Ordering::SeqCst) || active
	}
	
	pub fn clear_sync_interrupt(&mut self) {
		self.sync_interrupt_state.store(false, Ordering::SeqCst);
	}
	
	pub fn get_sync_interrupt_state(&mut self) -> bool {
		self.sync_interrupt_state.load(Ordering::SeqCst)
	}
}
