use rv_vsys::InterruptBus;
use std::cell::RefCell;
use std::sync::Arc;
use crate::gpu::GpuInterruptOutput;
use once_cell::sync::OnceCell;

#[derive(Clone)]
pub struct FmInterruptBus {
	gpu_interrupts: Arc<OnceCell<GpuInterruptOutput>>
}

impl FmInterruptBus {
	pub fn new() -> Self {
		Self {
			gpu_interrupts: Arc::new(OnceCell::default())
		}
	}
	
	pub fn set_gpu_interrupts(&mut self, gpu_int_out: GpuInterruptOutput) {
		self.gpu_interrupts.set(gpu_int_out).unwrap();
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
