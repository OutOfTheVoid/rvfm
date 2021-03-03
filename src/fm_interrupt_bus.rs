use rv_vsys::{InterruptBus, MemReadResult, MemWriteResult};
use std::sync::Arc;
use crate::gpu::GpuInterruptOutput;
use crate::sound_device::SoundInterruptOutput;
use once_cell::sync::OnceCell;

const OFFSET_VSYNC_INTERRUPT: u32 = 0;
const OFFSET_SOUND_INTERRUPT: u32 = 4;

#[derive(Clone)]
pub struct FmInterruptBus {
	gpu_interrupts: Arc<OnceCell<GpuInterruptOutput>>,
	sound_interrupt: Arc<OnceCell<SoundInterruptOutput>>,
}

impl FmInterruptBus {
	pub fn new() -> Self {
		Self {
			gpu_interrupts: Arc::new(OnceCell::default()),
			sound_interrupt: Arc::new(OnceCell::default()),
		}
	}
	
	pub fn set_gpu_interrupts(&mut self, gpu_int_out: GpuInterruptOutput) {
		self.gpu_interrupts.set(gpu_int_out).unwrap();
	}
	
	pub fn set_sound_interrupt(&mut self, sound_int_out: SoundInterruptOutput) {
		self.sound_interrupt.set(sound_int_out).unwrap();
	}
	
	pub fn write_32(&mut self, offset: u32, val: u32) -> MemWriteResult {
		match offset {
			OFFSET_VSYNC_INTERRUPT => {
				if val == 0 {
					self.gpu_interrupts.get().unwrap().clone().clear_sync_interrupt();
				}
				MemWriteResult::Ok
			},
			OFFSET_SOUND_INTERRUPT => {
				if val == 0 {
					self.sound_interrupt.get().unwrap().clone().clear_audio_interrupt();
				}
				MemWriteResult::Ok
			},
			_ => MemWriteResult::PeripheralError
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		match offset {
			OFFSET_VSYNC_INTERRUPT => {
				MemReadResult::Ok(
					if self.gpu_interrupts.get().unwrap().clone().get_sync_interrupt_state() {
						1
					 } else {
						 0
					 }
				)
			},
			OFFSET_SOUND_INTERRUPT => {
				MemReadResult::Ok(
					if self.sound_interrupt.get().unwrap().clone().get_audio_interrupt_state() {
						1
					 } else {
						 0
					 }
				)
			}
			_ => MemReadResult::PeripheralError
		}
	}
}

unsafe impl Send for FmInterruptBus {}
unsafe impl Sync for FmInterruptBus {}

impl InterruptBus for FmInterruptBus {
	fn poll_interrupts(&mut self, hart_id: u32) -> bool {
		match hart_id {
			0 => {
				self.gpu_interrupts.get().unwrap().clone().poll_sync_interrupt()
			},
			1 => {
				self.sound_interrupt.get().unwrap().clone().poll_audio_interrupt()
			}
			_ => false
		}
	}
}
