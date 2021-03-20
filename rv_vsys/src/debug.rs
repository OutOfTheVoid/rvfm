enum DebugStep {
	Pause,
	Continue,
	Kill,
}

pub trait DebugAdapter<Timer: MTimer, MIO: MemIO<Timer>, IntBus: InterruptBus> {
	fn debug_begin(hart_id: u32);
	fn debug_end(hart_id: u32);
	fn debug_step_poll(cpu: &mut Cpu<Timer, MIO, IntBus>) -> DebugStep;
}
