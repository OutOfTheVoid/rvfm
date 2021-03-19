use rv_vsys::{InterruptBus, MemReadResult, MemWriteResult};
use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}};
use crate::gpu::GpuInterruptOutput;
use crate::sound_device::SoundInterruptOutput;
use once_cell::sync::OnceCell;

const OFFSET_VSYNC_INTERRUPT: u32 = 0;
const OFFSET_SOUND_INTERRUPT: u32 = 4;
const OFFSET_CPU0_IPI: u32 = 8;
const OFFSET_CPU1_IPI: u32 = 12;

#[derive(Clone)]
pub struct FmInterruptBus {
	gpu_interrupts: Arc<OnceCell<GpuInterruptOutput>>,
	sound_interrupt: Arc<OnceCell<SoundInterruptOutput>>,
	cpu0_ipi: Arc<AtomicBool>,
	cpu1_ipi: Arc<AtomicBool>,
}

impl FmInterruptBus {
	pub fn new() -> Self {
		Self {
			gpu_interrupts: Arc::new(OnceCell::default()),
			sound_interrupt: Arc::new(OnceCell::default()),
			cpu0_ipi: Arc::new(AtomicBool::new(false)),
			cpu1_ipi: Arc::new(AtomicBool::new(false)),
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
			OFFSET_CPU0_IPI => {
				self.cpu0_ipi.store(val != 0, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			OFFSET_CPU1_IPI => {
				self.cpu1_ipi.store(val != 0, Ordering::SeqCst);
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
			},
			OFFSET_CPU0_IPI => {
				MemReadResult::Ok(
					if self.cpu0_ipi.load(Ordering::SeqCst) {
						1
					} else {
						0
					}
				)
			},
			OFFSET_CPU1_IPI => {
				MemReadResult::Ok(
					if self.cpu0_ipi.load(Ordering::SeqCst) {
						1
					} else {
						0
					}
				)
			},
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
				self.gpu_interrupts.get().unwrap().clone().get_sync_interrupt_state()// || self.cpu0_ipi.load(Ordering::SeqCst)
			},
			1 => {
				self.sound_interrupt.get().unwrap().clone().poll_audio_interrupt()// || self.cpu1_ipi.load(Ordering::SeqCst)
			},
			_ => false
		}
	}
}
