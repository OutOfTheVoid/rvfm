use cpal::{self, traits::{DeviceTrait, HostTrait, StreamTrait}};
use rv_vsys::{CpuWakeupHandle, MemIO, MemReadResult, MemWriteResult};
use core::f32;
use std::{f32::consts::PI, sync::{Arc, atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering}}};
use std::fmt::{self, Debug, Formatter};
use parking_lot::{Mutex};

use crate::{fm_interrupt_bus::FmInterruptBus, fm_mio::FmMemoryIO};

#[allow(dead_code)]
pub struct SoundDevice {
	enabled: Arc<AtomicBool>,
	interrupt_enabled: Arc<AtomicBool>,
	stream: cpal::Stream,
	frame_buffer: Arc<Mutex<Box<[i16]>>>,
	frame_count: Arc<AtomicUsize>,
	frame_ptr: Arc<AtomicU32>,
}

impl Debug for SoundDevice {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("SoundDevice").finish()
	}
}

#[derive(Clone, Debug)]
pub struct SoundInterruptOutput {
	audio_frame: Arc<AtomicUsize>,
	cpu_frame: Arc<AtomicUsize>,
	audio_interrupt_state: Arc<AtomicBool>,
	audio_interrupt_enable: Arc<AtomicBool>
}

const SOUND_OFFSET_ENABLE: u32 = 0;
const SOUND_OFFSET_FRAME: u32 = 4;
const SOUND_OFFSET_INTERRUPT_ENABLE: u32 = 8;
const SOUND_OFFSET_FRAME_PTR: u32 = 12;
const SOUND_OFFSET_COPY_BUFF: u32 = 16;

impl SoundInterruptOutput {
	pub fn new(audio_frame: Arc<AtomicUsize>, audio_interrupt_enable: Arc<AtomicBool>) -> Self {
		Self {
			cpu_frame: Arc::new(AtomicUsize::new(0)),
			audio_frame,
			audio_interrupt_state: Arc::new(AtomicBool::new(false)),
			audio_interrupt_enable
		}
	}
	
	pub fn poll_audio_interrupt(&mut self) -> bool {
		if ! self.audio_interrupt_enable.load(Ordering::SeqCst) {
			self.audio_interrupt_state.store(false, Ordering::SeqCst);
			return false
		}
		let gpu_frame_num = self.audio_frame.load(Ordering::SeqCst);
		let active = self.cpu_frame.load(Ordering::SeqCst) < gpu_frame_num;
		self.cpu_frame.store(gpu_frame_num, Ordering::SeqCst);
		self.audio_interrupt_state.fetch_or(active, Ordering::SeqCst) || active
	}
	
	pub fn clear_audio_interrupt(&mut self) {
		self.audio_interrupt_state.store(false, Ordering::SeqCst);
	}
	
	pub fn get_audio_interrupt_state(&mut self) -> bool {
		self.audio_interrupt_state.load(Ordering::SeqCst)
	}
}

const FRAME_SIZE: usize = 128;
const CHANNEL_COUNT: usize = 2;

impl SoundDevice {
	pub fn new(device: Option<String>, mut cpu_wake_handle: CpuWakeupHandle, interrupt_bus: &mut FmInterruptBus) -> Result<Self, String> {
		let host = cpal::default_host();
		let device = match device {
			Some(device_name) => {
				let mut device_list = match host.output_devices() {
					Ok(device_list) => device_list,
					Err(error) => {
						return Err(format!("could not to get audio device list: {}", error).to_string())
					}
				};
				match device_list.find_map(|device| {
					if device.name().unwrap() == device_name {
						Some(device)
					} else {
						None
					}
				}) {
					Some(device) => device,
					None => return Err(format!("could not find audio device with name {}", device_name).to_string())
				}
			},
			None => {
				match host.default_output_device() {
					Some(device) => device,
					None => return Err("could not get default audio device".to_string())
				}
			}
		};
		let frame_buffer = Arc::new(Mutex::new(vec![0i16; FRAME_SIZE * CHANNEL_COUNT].into_boxed_slice()));
		let audio_frame = Arc::new(AtomicUsize::new(0));
		let enabled = Arc::new(AtomicBool::new(false));
		let interrupt_enabled = Arc::new(AtomicBool::new(false));
		let cb_frame_buffer = frame_buffer.clone();
		let cb_audio_frame = audio_frame.clone();
		let cb_enabled = enabled.clone();
		let cb_interrupt_enabled = interrupt_enabled.clone();
		let stream = device.build_output_stream(&cpal::StreamConfig {
			channels: 2,
			sample_rate: cpal::SampleRate(44100),
			buffer_size: cpal::BufferSize::Fixed(FRAME_SIZE as u32),
		}, move |data: &mut [f32], _| {
			let frame_buffer = cb_frame_buffer.lock();
			if cb_enabled.load(Ordering::SeqCst) {
				for i in 0 .. FRAME_SIZE * CHANNEL_COUNT {
					data[i] = f32::from((*frame_buffer)[i]) / 32768.0;
				}
				cb_audio_frame.fetch_add(1, Ordering::SeqCst);
				if cb_interrupt_enabled.load(Ordering::SeqCst) {
					cpu_wake_handle.cpu_wake();
				}
			} else {
				for i in 0 .. FRAME_SIZE * CHANNEL_COUNT {
					data[i] = 0.0;
				}
			}
		}, |_| {
		}).unwrap();
		stream.play().unwrap();
		std::thread::sleep(std::time::Duration::from_secs(2));
		let int_audio_frame = audio_frame.clone();
		let int_interrupt_enabled = interrupt_enabled.clone();
		interrupt_bus.set_sound_interrupt(SoundInterruptOutput::new(int_audio_frame, int_interrupt_enabled));
		Ok(SoundDevice {
			enabled,
			interrupt_enabled,
			stream,
			frame_buffer,
			frame_count: audio_frame,
			frame_ptr: Arc::new(AtomicU32::new(0)),
		})
	}
	
	pub fn write_32(&self, mio: &mut FmMemoryIO, offset: u32, value: u32) -> MemWriteResult {
		match offset {
			SOUND_OFFSET_ENABLE => {
				self.enabled.store(value != 0, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			SOUND_OFFSET_FRAME => {
				MemWriteResult::Ok
			},
			SOUND_OFFSET_INTERRUPT_ENABLE => {
				self.interrupt_enabled.store(value != 0, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			SOUND_OFFSET_FRAME_PTR => {
				self.frame_ptr.store(value, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			SOUND_OFFSET_COPY_BUFF => {
				let mut buffer = self.frame_buffer.lock();
				let addr = self.frame_ptr.load(Ordering::SeqCst);
				for i in 0 .. (FRAME_SIZE * CHANNEL_COUNT) as u32 {
					match mio.read_16(addr + i * 2) {
						MemReadResult::Ok(value) => {
							(*buffer)[i as usize] = value as i16;
						},
						_ => return MemWriteResult::PeripheralError
					};
				}
				MemWriteResult::Ok
			}
			_ => MemWriteResult::PeripheralError
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		match offset {
			SOUND_OFFSET_ENABLE => MemReadResult::Ok(self.enabled.load(Ordering::SeqCst) as u32),
			SOUND_OFFSET_FRAME => MemReadResult::Ok(self.frame_count.load(Ordering::SeqCst) as u32),
			SOUND_OFFSET_INTERRUPT_ENABLE => MemReadResult::Ok(if self.interrupt_enabled.load(Ordering::SeqCst) { 1 } else { 0 }),
			SOUND_OFFSET_FRAME_PTR => MemReadResult::Ok(self.frame_ptr.load(Ordering::SeqCst)),
			SOUND_OFFSET_COPY_BUFF => MemReadResult::Ok(0),
			_ => MemReadResult::PeripheralError
		}
	}
}
