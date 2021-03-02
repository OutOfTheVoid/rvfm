use rv_vsys::{InterruptBus, MemReadResult, MemWriteResult};
use std::{cell::RefCell, sync::atomic::AtomicBool};
use std::sync::Arc;
use crate::gpu::GpuInterruptOutput;
use once_cell::sync::OnceCell;

const OFFSET_SYNC_INTERRUPT: u32 = 0;

#[derive(Clone)]
pub struct FmInterruptBus {
	gpu_interrupts: Arc<OnceCell<GpuInterruptOutput>>,
}

impl FmInterruptBus {
	pub fn new() -> Self {
		Self {
			gpu_interrupts: Arc::new(OnceCell::default()),
		}
	}
	
	pub fn set_gpu_interrupts(&mut self, gpu_int_out: GpuInterruptOutput) {
		self.gpu_interrupts.set(gpu_int_out).unwrap();
	}
	
	pub fn write_32(&mut self, offset: u32, val: u32) -> MemWriteResult {
		match offset {
			OFFSET_SYNC_INTERRUPT => {
				if val == 0 {
					self.gpu_interrupts.get().unwrap().clone().clear_sync_interrupt();
				}
				MemWriteResult::Ok
			},
			_ => MemWriteResult::PeripheralError
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		match offset {
			OFFSET_SYNC_INTERRUPT => {
				MemReadResult::Ok(
					if self.gpu_interrupts.get().unwrap().clone().get_sync_interrupt_state() {
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
				self.gpu_interrupts.get().unwrap().clone().poll_sync_interrupt()
			},
			_ => false
		}
	}
}
