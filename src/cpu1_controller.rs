use rv_vsys::{Cpu, MemWriteResult, MemReadResult, CpuKillHandle};
use crate::{application_core, fm_interrupt_bus::FmInterruptBus, fm_mio::FmMemoryIO, mtimer::MTimerPeripheral};
use std::{fmt, thread, time::Duration};
use std::sync::Arc;
use parking_lot::Mutex;

enum Cpu1State {
	Idle(Cpu<MTimerPeripheral, FmMemoryIO, FmInterruptBus>),
	Running,
}

impl fmt::Debug for Cpu1State {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Cpu1State::Running => f.write_str("Running"),
			Cpu1State::Idle(..) => f.write_str("Idle"),
		}
	}
}

#[derive(Debug)]
pub struct Cpu1ControllerInternal {
	cpu_kill_handle: CpuKillHandle,
	state: Cpu1State,
	start_address: u32,
}

#[derive(Clone, Debug)]
pub struct Cpu1Controller {
	lock: Arc<Mutex<Cpu1ControllerInternal>>
}

unsafe impl Send for Cpu1Controller {}
unsafe impl Sync for Cpu1Controller {}

const OFFSET_START_ADDRESS: u32 = 0;
const OFFSET_STARTUP_TRIGGER: u32 = 4;
const OFFSET_IS_RUNNING: u32 = 8;

impl Cpu1Controller {
	pub fn new(cpu: Cpu<MTimerPeripheral, FmMemoryIO, FmInterruptBus>) -> Self {
		Self {
			lock: Arc::new(Mutex::new(Cpu1ControllerInternal {
				cpu_kill_handle: cpu.get_kill_handle(),
				state: Cpu1State::Idle(cpu),
				start_address: 0
			}))
		}
	}
	
	pub fn write_32(&mut self, offset: u32, value: u32) -> MemWriteResult {
		let state_lock = self.lock.clone();
		let mut gaurd = self.lock.lock();
		match offset {
			OFFSET_START_ADDRESS => {
				gaurd.start_address = value;
				MemWriteResult::Ok
			},
			OFFSET_STARTUP_TRIGGER => {
				match & gaurd.state {
					Cpu1State::Idle(..) => {
						let start_pc = gaurd.start_address;
						thread::spawn(move || {
							let cpu_opt = {
								let mut local_gaurd = state_lock.lock();
								let mut state_swap = Cpu1State::Running;
								std::mem::swap(&mut local_gaurd.state, &mut state_swap);
								match state_swap {
									Cpu1State::Idle(cpu) => Some(cpu),
									Cpu1State::Running => None,
								}
							};
							match cpu_opt {
								Some(mut cpu) => {
									cpu.reset(start_pc);
									cpu.run_loop(application_core::CPU_INSTRUCTIONS_PER_PERIOD, Duration::from_micros(application_core::CPU_PERIOD_MICROSECONDS));
									let mut local_gaurd = state_lock.lock();
									let mut state_swap = Cpu1State::Idle(cpu);
									std::mem::swap(&mut local_gaurd.state, &mut state_swap);
								},
								None => {
									return;
								}
							}
						});
						MemWriteResult::Ok
					},
					Cpu1State::Running => MemWriteResult::PeripheralError,
				}
			},
			_ => MemWriteResult::PeripheralError
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		let gaurd = self.lock.lock();
		match offset {
			OFFSET_START_ADDRESS => MemReadResult::Ok(gaurd.start_address),
			OFFSET_STARTUP_TRIGGER => MemReadResult::Ok(0),
			OFFSET_IS_RUNNING => {
				match &gaurd.state {
					Cpu1State::Idle(..) => MemReadResult::Ok(0),
					Cpu1State::Running => MemReadResult::Ok(1),
				}
			},
			_ => MemReadResult::PeripheralError,
		}
	}
}
