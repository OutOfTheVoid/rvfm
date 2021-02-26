use std::sync::{mpsc, Arc, Mutex, atomic::{AtomicUsize, Ordering}};

use once_cell::sync::OnceCell;

use winit::window::Window;

use crate::{fm_mio::FmMemoryIO, raw_fb_renderer::RawFBRenderer, fm_interrupt_bus::FmInterruptBus};
use rv_vsys::{CpuWakeupHandle, MemIO, MemWriteResult};

#[allow(non_camel_case_types)]
enum TextureFormat {
	/*
	// 4-bit uint pairs
	U4_Vec2,
	U4_Vec4,
	*/
	// 8-bit uint
	U8,
	U8_Vec2,
	U8_Vec3,
	U8_Vec4,
	
	/*
	// 16-bit uint
	U16,
	U16_Vec2,
	U16_Vec3,
	U16_Vec4,
	
	// 32-bit uint
	U32,
	U32_Vec2,
	U32_Vec3,
	U32_Vec4,
	*/
	
	// 32-bit int
	I32,
	I32_Vec2,
	I32_Vec3,
	I32_Vec4,
	
	/*
	// 16-bit fixed 8.8
	Fx16,
	Fx16_Vec2,
	Fx16_Vec3,
	Fx16_Vec4,
	*/
	
	// 32-bit float
	Fp32,
	Fp32_Vec2,
	Fp32_Vec3,
	Fp32_Vec4,
}

pub struct Gpu {
	instance: wgpu::Instance,
	surface: wgpu::Surface,
	adapter: wgpu::Adapter,
	device: wgpu::Device,
	queue: wgpu::Queue,
	swap_chain_desc: wgpu::SwapChainDescriptor,
	swap_chain: wgpu::SwapChain,
	mio: FmMemoryIO,
	mode: Mode,
	next_mode: Mode,
	present_counter: Arc<AtomicUsize>,
	cmd_queue: mpsc::Receiver<Command>,
	raw_fb_renderer: Option<RawFBRenderer>,
	cpu_wakeup: CpuWakeupHandle,
}

enum GpuTextureLayout {
	Tex1D(u32),
	Tex2D(u32, u32),
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
	Disabled,
	RawFBDisplay,
}

pub enum Command {
	SetMode(Mode),
	SetupRamBuffer{buffer_id: u32, address: u32},
	SetupRamBufferTextureView{buffer_id: u32, format: TextureFormat, layout: GpuTextureLayout}
}

pub const GPU_OUTPUT_W: u32 = 256;
pub const GPU_OUTPUT_H: u32 = 192;
pub const GPU_OUTPUT_FB_SIZE: u32 = GPU_OUTPUT_W * GPU_OUTPUT_H * 4;
pub const GPU_SCREENWIN_SCALE: u32 = 4;

impl Gpu {
	pub async fn new(window: &Window, mio: &mut FmMemoryIO, int_bus: &mut FmInterruptBus, cpu_wakeup: CpuWakeupHandle) -> Self {
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
		let swap_desc = wgpu::SwapChainDescriptor {
			usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: GPU_OUTPUT_W * GPU_SCREENWIN_SCALE,
            height: GPU_OUTPUT_H * GPU_SCREENWIN_SCALE,
            present_mode: wgpu::PresentMode::Fifo,
		};
		let swap_chain = device.create_swap_chain(&surface, &swap_desc);
		mio.set_gpu_interface(GpuPeripheralInterface::new(cmd_queue_tx));
		let present_counter = Arc::new(AtomicUsize::new(1));
		int_bus.set_gpu_interrupts(GpuInterruptOutput::new(present_counter.clone()));
		Gpu {
			instance: instance,
			surface: surface,
			adapter: adapter,
			device: device,
			queue: queue,
			swap_chain_desc: swap_desc,
			swap_chain: swap_chain,
			mio: mio.clone(),
			mode: Mode::Disabled,
			present_counter: present_counter,
			next_mode: Mode::Disabled,
			cmd_queue: cmd_queue_rx,
			raw_fb_renderer: None,
			cpu_wakeup: cpu_wakeup
		}
	}
	
	fn kill_last_mode(&mut self) {
		match self.mode {
			Mode::Disabled => {},
			Mode::RawFBDisplay => {
				self.raw_fb_renderer = None;
			}
		}
	}
	
	fn recv_commands(&mut self) {
		while match self.cmd_queue.try_recv() {
			Ok(command) => {
				match command {
					Command::SetMode(mode) => {
						self.next_mode = mode;
						true
					},
					Command::SetupRamBuffer{..} => {
						println!("Unimplemented GPU command: SetupRamBuffer");
						true
					},
					Command::SetupRamBufferTextureView{..} => {
						println!("Unimplemented GPU command: SetupRamBufferTextureView");
						true
					}
				}
			},
			_ => false,
		}{}
	}
	
	pub fn render(&mut self) {
		self.recv_commands();
		let framebuffer = self.swap_chain.get_current_frame().unwrap().output;
		let mut command_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
			label: Some("RVFM GPU Swap Encoder Descriptor")
		});
		if self.next_mode != self.mode {
			self.kill_last_mode();
		}
		self.mode = self.next_mode;
		self.present_counter.fetch_add(1, Ordering::SeqCst);
		match self.mode {
			Mode::Disabled => {
				let mut _render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
					color_attachments: &[
						wgpu::RenderPassColorAttachmentDescriptor {
							attachment: &framebuffer.view,
							resolve_target: None,
							ops: wgpu::Operations {
								load: wgpu::LoadOp::Clear(wgpu::Color {
									r: 0.0, g: 0.0, b: 0.1, a: 1.0
								}),
								store: true
							}
						}
					],
					depth_stencil_attachment: None,
				});
			},
			Mode::RawFBDisplay => {
				if let None = &self.raw_fb_renderer {
					self.raw_fb_renderer = Some(RawFBRenderer::new(&self.device, &self.swap_chain_desc).unwrap());
				}
				match &mut self.raw_fb_renderer {
					Some(renderer) => renderer.render(&mut self.mio, &self.queue, &mut command_encoder, &framebuffer),
					None => {}
				}
				self.cpu_wakeup.cpu_wake();
			}
		}
		self.mio.access_break();
		self.queue.submit(Some(command_encoder.finish()));
	}
}

pub const GPU_REGISTER_MODE: u32 = 0;
pub const GPU_MODE_VALUE_DISABLED: u32 = 0;
pub const GPU_MODE_VALUE_RAW_FB: u32 = 1;

#[derive(Clone)]
pub struct GpuPeripheralInterface {
	cmd_queue: mpsc::Sender<Command>
}

impl GpuPeripheralInterface {
	pub fn new(cmd_queue: mpsc::Sender<Command>) -> Self {
		Self {
			cmd_queue
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
					_ => MemWriteResult::ErrUnmapped
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
}

impl GpuInterruptOutput {
	pub fn new(gpu_frame: Arc<AtomicUsize>) -> Self {
		Self {
			cpu_frame: Arc::new(AtomicUsize::new(0)),
			gpu_frame
		}
	}
	
	pub fn poll_sync_interrupt(&mut self) -> bool {
		let gpu_frame_num = self.gpu_frame.load(Ordering::SeqCst);
		let active = self.cpu_frame.load(Ordering::SeqCst) < gpu_frame_num;
		self.cpu_frame.store(gpu_frame_num, Ordering::SeqCst);
		active
	}
}
