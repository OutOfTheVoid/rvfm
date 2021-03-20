use cpal::{self, traits::{DeviceTrait, HostTrait, StreamTrait}};
use rv_vsys::{CpuWakeupHandle, MemIO, MemReadResult, MemWriteResult};
use core::f32;
use std::{sync::{Arc, Barrier, atomic::{AtomicBool, AtomicU32, Ordering}}, usize};
use std::fmt::{self, Debug, Formatter};
use parking_lot::{Mutex};
use ringbuf;

use crate::{fm_interrupt_bus::FmInterruptBus, fm_mio::FmMemoryIO};

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

const SAMPLE_RATE: u32 = 48000;
const CHANNEL_COUNT: u32 = 2;
const TARGET_FRAME_LENGTH: u32 = 128;
const ELEMENTS_PER_FRAME: u32 = TARGET_FRAME_LENGTH * CHANNEL_COUNT;
const SOUND_FIFO_LENGTH: u32 = ELEMENTS_PER_FRAME * 5;
const FIFO_FILL_THRESHOLD: u32 = ELEMENTS_PER_FRAME * 3;

struct SoundCallbackData {
	ring_buffer: ringbuf::Consumer<i16>,
	enabled: Arc<AtomicBool>,
	fifo_int_enabled: Arc<AtomicBool>,
	fifo_int_state: Arc<AtomicBool>,
	cpu_wakeup: CpuWakeupHandle,
	double_rate: bool,
	odd_sample: bool,
	double_rate_sample: i16,
}

impl SoundCallbackData {
	pub fn fill_buffer<T: FromNativeSample>(&mut self, buffer: &mut [T]) {
		if self.enabled.load(Ordering::SeqCst) {
			if self.double_rate {
				let fill_size = (self.ring_buffer.len() * 2).min(buffer.len());
				let mut odd_sample = self.odd_sample;
				for i in 0 .. fill_size {
					if self.odd_sample {
						buffer[i] = T::from_native_sample(self.double_rate_sample);
					} else {
						self.double_rate_sample = self.ring_buffer.pop().unwrap();
						buffer[i] = T::from_native_sample(self.double_rate_sample);
					}
					odd_sample = ! odd_sample;
				}
				self.odd_sample = odd_sample;
				for i in fill_size .. buffer.len() {
					buffer[i] = T::SAMPLE_ZERO;
				}
			} else {
				let fill_size = self.ring_buffer.len().min(buffer.len());
				for i in 0 .. fill_size {
					buffer[i] = T::from_native_sample(self.ring_buffer.pop().unwrap());
				}
				for i in fill_size .. buffer.len() {
					buffer[i] = T::SAMPLE_ZERO;
				}
			}
			if (self.ring_buffer.len() as u32) < FIFO_FILL_THRESHOLD {
				self.fifo_int_state.store(true, Ordering::SeqCst);
				if self.fifo_int_enabled.load(Ordering::SeqCst) {
					self.cpu_wakeup.cpu_wake();
				}
			}
		} else {
			for i in 0 .. buffer.len() {
				buffer[i] = T::SAMPLE_ZERO;
			}
		}
	}
}

#[derive(Debug)]
pub struct SoundInterruptOutput {
	fifo_int_state: Arc<AtomicBool>,
}

impl SoundInterruptOutput {
	pub fn get_fifo_int_state(&self) -> bool {
		self.fifo_int_state.load(Ordering::SeqCst)
	}
	
	pub fn clear_fifo_int_state(&self) {
		self.fifo_int_state.store(false, Ordering::SeqCst);
	}
}

const SOUND_OUTPUT_REG_ENABLE: u32 = 0;
const SOUND_OUTPUT_REG_FIFO_LENGTH: u32 = 4;
const SOUND_OUTPUT_REG_ENABLE_FIFO_INT: u32 = 8;
const SOUND_OUTPUT_REG_FILL_SOUND_PTR: u32 = 12;
const SOUND_OUTPUT_REG_FILL_TRIGGER: u32 = 16;
const SOUND_OUTPUT_REG_FILL_COUNT: u32 = 20;

pub struct SoundOutPeripheral {
	ring_buffer: Arc::<Mutex<ringbuf::Producer<i16>>>,
	enabled: Arc<AtomicBool>,
	fifo_int_enabled: Arc<AtomicBool>,
	source_ptr: Arc<AtomicU32>,
	_stream: cpal::Stream,
	last_fill_count: Arc<AtomicU32>
}

impl Debug for SoundOutPeripheral {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("SoundOutPeripheral")
    }
}

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
			if TARGET_FRAME_LENGTH as u32 >= *min && TARGET_FRAME_LENGTH as u32 <= *max {
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

fn pick_sound_format(configs: cpal::SupportedOutputConfigs) -> Option<(cpal::StreamConfig, cpal::SampleFormat, bool)> {
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
						buffer_size: cpal::BufferSize::Fixed(if double_rate {TARGET_FRAME_LENGTH * 2} else {TARGET_FRAME_LENGTH} as cpal::FrameCount),
					},
					best_config.sample_format(),
					double_rate
				)
			)
		},
		None => None
	}
}

impl SoundOutPeripheral {
	pub fn new(cpu_wakeup: CpuWakeupHandle, interrupt_bus: &mut FmInterruptBus, mio: &mut FmMemoryIO, with_host_name: Option<String>, with_device_name: Option<String>) -> Result<(), String> {
		let ring_buffer = ringbuf::RingBuffer::new(SOUND_FIFO_LENGTH as usize);
		let (ring_buff_in, ring_buff_out) = ring_buffer.split();
		let enabled = Arc::new(AtomicBool::new(false));
		let stream_started = Arc::new(Barrier::new(2));
		let fifo_int_enabled = Arc::new(AtomicBool::new(false));
		let fifo_int_state = Arc::new(AtomicBool::new(false));
		let host = if let Some(with_host_name) = with_host_name {
			let hosts = cpal::available_hosts();
			let mut host_index: i32 = -1;
			for i in 0 .. hosts.len() {
				if hosts[i].name().to_lowercase() == with_host_name.to_lowercase() {
					host_index = i as i32;
					break;
				}
			}
			if host_index == -1 {
				return Err(format!("Audio host \"{}\" not found", with_host_name));
			}
			if let Ok(host) = cpal::host_from_id(hosts[host_index as usize]) {
				host
			} else {
				return Err(format!("Could not use audio host \"{}\"", with_host_name));
			}
		} else {
			cpal::default_host()
		};
		let device = if let Some(with_device_name) = with_device_name {
			let with_device_name = with_device_name.to_lowercase();
			if let Ok(devices) = host.output_devices() {
				let mut found_device = None;
				for device in devices {
					if let Ok(device_name) = device.name() {
						if device_name.to_lowercase() == with_device_name {
							found_device = Some(device);
							break;
						}
					}
				}
				if let Some(device) = found_device {
					device
				} else {
					return Err(format!("Could not find audio device \"{}\"", with_device_name));
				}
			} else {
				return Err("Failed to enumerate audio devices".to_string());
			}
		} else {
			if let Some(device) = host.default_output_device() {
				device
			} else {
				return Err("No audio output device available".to_string());
			}
		};
		let output_configs = if let Ok(output_configs) = device.supported_output_configs() {
			output_configs
		} else {
			return Err("Failed to enumerate supported output configurations of audio device".to_string());
		};
		let (stream_config, sample_format, double_rate) = if let Some(format_info) = pick_sound_format(output_configs) {
			format_info
		} else {
			return Err("No adequate sound output format found for sound device".to_string());
		};
		let mut barrier_waited = false;
		let callback_barrier = stream_started.clone();
		let mut callback_data = SoundCallbackData {
			ring_buffer: ring_buff_out,
			enabled: enabled.clone(),
			fifo_int_enabled: fifo_int_enabled.clone(),
			fifo_int_state: fifo_int_state.clone(),
			cpu_wakeup,
			double_rate,
			double_rate_sample: 0,
			odd_sample: false
		};
		let stream_build_result = match sample_format {
			cpal::SampleFormat::I16 => {
				device.build_output_stream(&stream_config, move |buffer: &mut [i16], _| {
					if ! barrier_waited {
						callback_barrier.wait();
						barrier_waited = true;
					}
					callback_data.fill_buffer(buffer);
				}, |_| {
					panic!("Audio output stream encountered an error");
				})
			},
			cpal::SampleFormat::U16 => {
				device.build_output_stream(&stream_config, move |buffer: &mut [u16], _| {
					if ! barrier_waited {
						callback_barrier.wait();
						barrier_waited = true;
					}
					callback_data.fill_buffer(buffer);
				}, |_| {
					panic!("Audio output stream encountered an error");
				})
			},
			cpal::SampleFormat::F32 => {
				device.build_output_stream(&stream_config, move |buffer: &mut [f32], _| {
					if ! barrier_waited {
						callback_barrier.wait();
						barrier_waited = true;
					}
					callback_data.fill_buffer(buffer);
				}, |_| {
					panic!("Audio output stream encountered an error");
				})
			}
		};
		let stream = match stream_build_result {
			Ok(stream) => stream,
			Err(_) => {
				return Err("Failed to open sound output stream".to_string());
			}
		};
		if let Err(_) = stream.play() {
			return Err("Failed to play sound output stream".to_string());
		}
		stream_started.wait();
		mio.set_sound_out(SoundOutPeripheral {
			ring_buffer: Arc::new(Mutex::new(ring_buff_in)),
			enabled,
			source_ptr: Arc::new(AtomicU32::new(0)),
			_stream: stream,
			fifo_int_enabled,
			last_fill_count: Arc::new(AtomicU32::new(0))
		});
		interrupt_bus.set_sound_interrupt(SoundInterruptOutput {
			fifo_int_state
		});
		Ok(())
	}
	
	pub fn write_32(&self, mio: &mut FmMemoryIO, offset: u32, value: u32) -> MemWriteResult {
		match offset {
			SOUND_OUTPUT_REG_ENABLE => {
				self.enabled.store(value != 0, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			SOUND_OUTPUT_REG_FIFO_LENGTH => MemWriteResult::ErrReadOnly,
			SOUND_OUTPUT_REG_ENABLE_FIFO_INT => {
				self.fifo_int_enabled.store(value != 0, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			SOUND_OUTPUT_REG_FILL_SOUND_PTR => {
				self.source_ptr.store(value, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			SOUND_OUTPUT_REG_FILL_TRIGGER => {
				let fill_start_addr: u32 = self.source_ptr.load(Ordering::SeqCst);
				let mut ring_buffer = self.ring_buffer.lock();
				let fill_size = value.min(ring_buffer.remaining() as u32);
				for i in 0 .. fill_size {
					if let MemReadResult::Ok(value) = mio.read_16(fill_start_addr + i * 2) {
						ring_buffer.push(value as i16).unwrap();
					} else {
						return MemWriteResult::PeripheralError;
					}
				}
				self.last_fill_count.store(fill_size, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			_ => MemWriteResult::PeripheralError
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		match offset {
			SOUND_OUTPUT_REG_ENABLE => MemReadResult::Ok(if self.enabled.load(Ordering::SeqCst) { 1 } else { 0 }),
			SOUND_OUTPUT_REG_FIFO_LENGTH => {
				let ring_buffer = self.ring_buffer.lock();
				MemReadResult::Ok(ring_buffer.len() as u32)
			}
			SOUND_OUTPUT_REG_ENABLE_FIFO_INT => MemReadResult::Ok(if self.fifo_int_enabled.load(Ordering::SeqCst) { 1 } else { 0 }),
			SOUND_OUTPUT_REG_FILL_SOUND_PTR => MemReadResult::Ok(self.source_ptr.load(Ordering::SeqCst)),
			SOUND_OUTPUT_REG_FILL_TRIGGER => MemReadResult::Ok(0),
			SOUND_OUTPUT_REG_FILL_COUNT => MemReadResult::Ok(self.last_fill_count.load(Ordering::SeqCst)),
			_ => MemReadResult::PeripheralError
		}
	}
}
