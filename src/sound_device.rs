use cpal::{self, SupportedStreamConfigRange, traits::{DeviceTrait, HostTrait, StreamTrait}};
use rv_vsys::{CpuWakeupHandle, MemIO, MemReadResult, MemWriteResult};
use core::f32;
use std::{mem::{self}, sync::{Arc, atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering}}};
use std::fmt::{self, Debug, Formatter};
use parking_lot::{Mutex};
use atom::{Atom, IntoRawPtr, FromRawPtr};

use crate::{fm_interrupt_bus::FmInterruptBus, fm_mio::FmMemoryIO};

struct AudioFrame {
	pub data: Box<[i16]>
}

impl IntoRawPtr for AudioFrame {
	fn into_raw(self) -> *mut () {
		let Self{mut data} = self;
		let ptr = data.as_mut_ptr() as *mut ();
		std::mem::forget(data);
		ptr
	}
}

impl FromRawPtr for AudioFrame {
	unsafe fn from_raw(ptr: *mut ()) -> Self {
		let slice_ptr = core::ptr::slice_from_raw_parts_mut(ptr as *mut i16, FRAME_SIZE * CHANNEL_COUNT);
		AudioFrame {
			data: Box::from_raw(slice_ptr)
		}
	}
}

#[allow(dead_code)]
pub struct SoundDevice {
	enabled: Arc<AtomicBool>,
	interrupt_enabled: Arc<AtomicBool>,
	stream: cpal::Stream,
	io_framebuffer: Arc<Atom<AudioFrame>>,
	write_framebuffer: Arc<Mutex<Option<AudioFrame>>>,
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

const FRAME_SIZE: usize = 256;
const CHANNEL_COUNT: usize = 2;
const SAMPLE_RATE: u32 = 44100;

fn sample_rate_score(c: &cpal::SupportedStreamConfigRange) -> i32 {
	if c.min_sample_rate().0 <= SAMPLE_RATE && c.max_sample_rate().0 >= SAMPLE_RATE {
		2
	} else if c.min_sample_rate().0 <= (SAMPLE_RATE * 2) && c.max_sample_rate().0 >= (SAMPLE_RATE * 2) {
		1
	} else {
		0
	}
}

fn buffer_size_score(c: &cpal::SupportedStreamConfigRange) -> i32 {
	match c.buffer_size() {
		cpal::SupportedBufferSize::Range {min, max} => {
			if FRAME_SIZE as u32 >= *min && FRAME_SIZE as u32 <= *max {
				1
			} else {
				i32::MIN
			}
		}
		cpal::SupportedBufferSize::Unknown => i32::MIN
	}
}

fn format_score(c: &cpal::SupportedStreamConfigRange) -> u32 {
	match c.sample_format() {
		cpal::SampleFormat::I16 => 2,
		cpal::SampleFormat::U16 => 1,
		cpal::SampleFormat::F32 => 0,
	}
}

fn channel_score(c: &cpal::SupportedStreamConfigRange) -> i32 {
	if c.channels() >= 2 {
		1
	} else {
		0
	}
}

fn b_is_better_sound_config(a: &Option<cpal::SupportedStreamConfigRange>, b: &cpal::SupportedStreamConfigRange) -> bool {
	match a {
		None => true,
		Some(a) => {
			if channel_score(b) > channel_score(a) {
				true
			} else if channel_score(a) > channel_score(b) {
				false
			} else {
				if sample_rate_score(b) > sample_rate_score(a) {
					true
				} else if sample_rate_score(b) < sample_rate_score(a) {
					false
				} else {
					if buffer_size_score(b) > buffer_size_score(a) {
						true
					} else if buffer_size_score(a) > buffer_size_score(b) {
						false
					} else {
						format_score(b) > format_score(a)
					}
				}
			}
		}
	}
}

fn pick_sound_format(device: &cpal::Device) -> Option<(cpal::StreamConfig, cpal::SampleFormat, bool)> {
	match device.supported_output_configs() {
		Ok(configs) => {
			let mut best_config = None;
			for config in configs {
				let new_best_config = Some(if b_is_better_sound_config(&best_config, &config) {
					config
				} else {
					best_config.clone().unwrap()
				});
				best_config = new_best_config;
			}
			match best_config {
				Some(best_config) => {
					let (sample_rate, double_rate) = if best_config.min_sample_rate().0 > SAMPLE_RATE {
						(cpal::SampleRate(SAMPLE_RATE * 2), true)
					} else {
						(cpal::SampleRate(SAMPLE_RATE), false)
					};
					Some(
						(
							cpal::StreamConfig {
								channels: CHANNEL_COUNT as cpal::ChannelCount,
								sample_rate: sample_rate,
								buffer_size: cpal::BufferSize::Fixed(if double_rate {FRAME_SIZE * 2} else {FRAME_SIZE} as cpal::FrameCount),
							},
							best_config.sample_format(),
							double_rate
						)
					)
				},
				None => None
			}
		},
		_ => None
	}
}

trait FromNativeSample {
	fn from_native_sample(v: i16) -> Self;
	const SAMPLE_ZERO: Self;
}

impl FromNativeSample for i16 {
	fn from_native_sample(v: i16) -> Self {
		v
	}
	const SAMPLE_ZERO: Self = 0;
}

impl FromNativeSample for u16 {
	fn from_native_sample(v: i16) -> Self {
		((v as i32) + 32768) as u16
	}
	const SAMPLE_ZERO: Self = 32768;
}

impl FromNativeSample for f32 {
	fn from_native_sample(v: i16) -> Self {
		if v >= 0 {
			f32::from(v) / (f32::from(i16::MAX))
		} else {
			f32::from(v) / -(f32::from(i16::MIN))
		}
	}
	const SAMPLE_ZERO: Self = 0.0;
}

fn sound_callback<T: FromNativeSample> (data: &mut [T], cb_enabled: &AtomicBool, cb_framebuffer: &mut Option<AudioFrame>, cb_io_framebuffer: &Atom<AudioFrame>, cb_audio_frame: &AtomicUsize, cb_interrupt_enabled: &AtomicBool, cpu_wake_handle: &mut CpuWakeupHandle, double_rate: bool) {
	for i in 0 .. data.len() {
		data[i] = T::SAMPLE_ZERO;
	}
	if cb_enabled.load(Ordering::SeqCst) {
		let mut cb_fb_swap = None;
		std::mem::swap(&mut cb_fb_swap, cb_framebuffer);
		let cb_fb_swap = cb_fb_swap.unwrap();
		let mut cb_fb_swap = Some(cb_io_framebuffer.swap(cb_fb_swap, Ordering::SeqCst).unwrap());
		std::mem::swap(&mut cb_fb_swap, cb_framebuffer);
		let framebuffer = cb_framebuffer.as_ref().unwrap();
		{
			let frame_data = &framebuffer.data;
			if double_rate {
				for i in 0 .. FRAME_SIZE {
					for c in 0 .. CHANNEL_COUNT {
						data[i * 2 * CHANNEL_COUNT + c] = T::from_native_sample(frame_data[i * CHANNEL_COUNT + c]);
						data[i * 2 * CHANNEL_COUNT + c + CHANNEL_COUNT] = T::from_native_sample(frame_data[i * CHANNEL_COUNT + c]);
					}
				}
			} else {
				for i in 0 .. FRAME_SIZE {
					for c in 0 .. CHANNEL_COUNT {
						data[i * CHANNEL_COUNT + c] = T::from_native_sample(frame_data[i * CHANNEL_COUNT + c]);
					}
				}
			}
		}
		cb_audio_frame.fetch_add(1, Ordering::SeqCst);
		if cb_interrupt_enabled.load(Ordering::SeqCst) {
			cpu_wake_handle.cpu_wake();
		}
	} else {
		if double_rate {
			for i in 0 .. FRAME_SIZE * CHANNEL_COUNT * 2 {
				data[i] = T::SAMPLE_ZERO;
			}
		} else {
			for i in 0 .. FRAME_SIZE * CHANNEL_COUNT {
				data[i] = T::SAMPLE_ZERO;
			}
		}
	}
}

fn sound_error_callback() {
	println!("Sound ERROR!");
}

impl SoundDevice {
	fn new_framebuffer() -> AudioFrame {
		AudioFrame {
			data: vec![0i16; FRAME_SIZE * CHANNEL_COUNT].into_boxed_slice()
		}
	}
	
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
		let io_framebuffer = Arc::new(Atom::new(Self::new_framebuffer()));
		let write_framebuffer = Arc::new(Mutex::new(Some(Self::new_framebuffer())));
		let audio_frame = Arc::new(AtomicUsize::new(0));
		let enabled = Arc::new(AtomicBool::new(false));
		let interrupt_enabled = Arc::new(AtomicBool::new(false));
		let cb_io_framebuffer = io_framebuffer.clone();
		let mut cb_framebuffer = Some(Self::new_framebuffer());
		let cb_audio_frame = audio_frame.clone();
		let cb_enabled = enabled.clone();
		let cb_interrupt_enabled = interrupt_enabled.clone();
		let (config, sample_format, double_rate) = match pick_sound_format(&device) {
			Some(format) => format,
			None => {
				return Result::Err("Unable to find suitable output format for sound device!".to_string());
			}
		};
		println!("cpal config: {:?}, format: {:?}, double_rate: {}", config, sample_format, double_rate);
		let stream = match sample_format {
			cpal::SampleFormat::I16 => {
				device.build_output_stream(&config, move |data: &mut [i16], _| {
					sound_callback(data, &cb_enabled, &mut cb_framebuffer, &cb_io_framebuffer, &cb_audio_frame, &cb_interrupt_enabled, &mut cpu_wake_handle, double_rate);
				}, |_| {
					sound_error_callback();
				}).unwrap()
			},
			cpal::SampleFormat::U16 => {
				device.build_output_stream(&config, move |data: &mut [u16], _| {
					sound_callback(data, &cb_enabled, &mut cb_framebuffer, &cb_io_framebuffer, &cb_audio_frame, &cb_interrupt_enabled, &mut cpu_wake_handle, double_rate);
				}, |_| {
					sound_error_callback();
				}).unwrap()
			},
			cpal::SampleFormat::F32 => {
				device.build_output_stream(&config, move |data: &mut [f32], _| {
					sound_callback(data, &cb_enabled, &mut cb_framebuffer, &cb_io_framebuffer, &cb_audio_frame, &cb_interrupt_enabled, &mut cpu_wake_handle, double_rate);
				}, |_| {
					sound_error_callback();
				}).unwrap()
			}
		};
		stream.play().unwrap();
		let int_audio_frame = audio_frame.clone();
		let int_interrupt_enabled = interrupt_enabled.clone();
		interrupt_bus.set_sound_interrupt(SoundInterruptOutput::new(int_audio_frame, int_interrupt_enabled));
		Ok(SoundDevice {
			enabled,
			interrupt_enabled,
			stream,
			io_framebuffer,
			write_framebuffer,
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
				let mut write_fb = self.write_framebuffer.lock();
				let write_fb_data = &mut (*write_fb).as_mut().unwrap().data;
				let addr = self.frame_ptr.load(Ordering::SeqCst);
				for i in 0 .. (FRAME_SIZE * CHANNEL_COUNT) as u32 {
					match mio.read_16(addr + i * 2) {
						MemReadResult::Ok(value) => {
							(write_fb_data)[i as usize] = value as i16;
						},
						_ => return MemWriteResult::PeripheralError
					};
				}
				mio.access_break();
				let mut wfb_swap = None;
				mem::swap(&mut wfb_swap, &mut *write_fb);
				*write_fb = Some(self.io_framebuffer.swap(wfb_swap.unwrap(), Ordering::SeqCst).unwrap());
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
