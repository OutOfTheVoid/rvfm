pub trait InterruptBus: Send + Clone {
	fn poll_interrupts(&mut self, hart_id: u32) -> bool;
}
