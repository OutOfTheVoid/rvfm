use rv_vsys::{InterruptBus, MemReadResult, MemWriteResult};
use std::{sync::{Arc, atomic::{AtomicBool, Ordering, AtomicU32}}};
use crate::gpu::GpuInterruptOutput;
use crate::sound_out::SoundInterruptOutput;
use once_cell::sync::OnceCell;

const OFFSET_VSYNC_INTERRUPT: u32 = 0;
const OFFSET_SOUND_INTERRUPT: u32 = 4;
const OFFSET_CPU0_IPI: u32 = 8;
const OFFSET_CPU1_IPI: u32 = 12;

const OFFSET_CPU0_IMASK: u32 = 512;
const OFFSET_CPU1_IMASK: u32 = 516;

const IMASK_BIT_VSYNC: u32 = 1 << 0;
const IMASK_BIT_SOUND_FIFO: u32 = 1 << 1;
const IMASK_BIT_IPI: u32 = 1 << 2;

#[derive(Clone)]
pub struct FmInterruptBus {
	gpu_interrupts: Arc<OnceCell<GpuInterruptOutput>>,
	sound_interrupt: Arc<OnceCell<SoundInterruptOutput>>,
	cpu0_ipi: Arc<AtomicBool>,
	cpu1_ipi: Arc<AtomicBool>,
	cpu0_imask: Arc<AtomicU32>,
	cpu1_imask: Arc<AtomicU32>,
}

impl FmInterruptBus {
	pub fn new() -> Self {
		Self {
			gpu_interrupts: Arc::new(OnceCell::default()),
			sound_interrupt: Arc::new(OnceCell::default()),
			cpu0_ipi: Arc::new(AtomicBool::new(false)),
			cpu1_ipi: Arc::new(AtomicBool::new(false)),
			cpu0_imask: Arc::new(AtomicU32::new(IMASK_BIT_IPI | IMASK_BIT_VSYNC)),
			cpu1_imask: Arc::new(AtomicU32::new(IMASK_BIT_IPI | IMASK_BIT_SOUND_FIFO))
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
					self.sound_interrupt.get().unwrap().clone().clear_fifo_int_state();
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
			
			OFFSET_CPU0_IMASK => {
				self.cpu0_imask.store(val, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			OFFSET_CPU1_IMASK => {
				self.cpu0_imask.store(val, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			_ => MemWriteResult::PeripheralError
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		match offset {
			OFFSET_VSYNC_INTERRUPT => MemReadResult::Ok(if self.gpu_interrupts.get().unwrap().clone().get_sync_interrupt_state() { 1 } else { 0 }),
			OFFSET_SOUND_INTERRUPT => MemReadResult::Ok(if self.sound_interrupt.get().unwrap().clone().get_fifo_int_state() { 1 } else { 0 }),
			OFFSET_CPU0_IPI => MemReadResult::Ok(if self.cpu0_ipi.load(Ordering::SeqCst) { 1 } else { 0 }),
			OFFSET_CPU1_IPI => MemReadResult::Ok(if self.cpu1_ipi.load(Ordering::SeqCst) { 1 } else { 0 }),
			_ => MemReadResult::PeripheralError
		}
	}
	
	fn get_ibits(&self, hart_id: u32) -> u32 {
		(if self.gpu_interrupts.get().unwrap().clone().get_sync_interrupt_state() { IMASK_BIT_VSYNC } else { 0 }) |
		(if self.sound_interrupt.get().unwrap().clone().get_fifo_int_state() { IMASK_BIT_SOUND_FIFO } else { 0 }) |
		match hart_id {
			0 => if self.cpu0_ipi.load(Ordering::SeqCst) { IMASK_BIT_IPI } else { 0 },
			1 => if self.cpu1_ipi.load(Ordering::SeqCst) { IMASK_BIT_IPI } else { 0 },
			_ => 0
		}
	}
	
	fn get_imask(&self, hart_id: u32) -> u32 {
		match hart_id {
			0 => self.cpu0_imask.load(Ordering::SeqCst),
			1 => self.cpu1_imask.load(Ordering::SeqCst),
			_ => 0,
		}
	}
}

unsafe impl Send for FmInterruptBus {}
unsafe impl Sync for FmInterruptBus {}

impl InterruptBus for FmInterruptBus {
	fn poll_interrupts(&mut self, hart_id: u32) -> bool {
		(self.get_ibits(hart_id) & self.get_imask(hart_id)) != 0
	}
}
