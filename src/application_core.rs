use std::{env::args, time::Duration};
use std::fs::File;
use std::io::Read;

use crate::{cart_loader::{CartLoader, CartLoaderCpuBarrier}, cpu1_controller::Cpu1Controller, mtimer::MTimerPeripheral, gpu::GpuResetHandle};

use rv_vsys::{Cpu, CpuWakeupHandle};
use crate::fm_mio::FmMemoryIO;
use crate::fm_interrupt_bus::FmInterruptBus;
use crate::elf_loader;

pub const CPU_INSTRUCTIONS_PER_PERIOD: u32 = 50000;
pub const CPU_PERIOD_MICROSECONDS: u64 = 2500;

pub struct ApplicationCore {
	cpu0: Cpu<MTimerPeripheral, FmMemoryIO, FmInterruptBus>,
	cpu1: Cpu<MTimerPeripheral, FmMemoryIO, FmInterruptBus>,
	cart_loader_barrier: CartLoaderCpuBarrier,
}

impl ApplicationCore {
	pub fn new(mio: FmMemoryIO, interrupt_bus: FmInterruptBus, cpu0_wakeup_handle: CpuWakeupHandle, cpu1_wakeup_handle: CpuWakeupHandle, gpu_reset_handle: GpuResetHandle) -> Self {
		let cpu0 = Cpu::new(mio.clone(), interrupt_bus.clone(), cpu0_wakeup_handle, 0);
		let cpu1 = Cpu::new(mio.clone(), interrupt_bus, cpu1_wakeup_handle, 1);
		let cart_loader_barrier = CartLoader::start(mio, &cpu0, &cpu1, gpu_reset_handle);
		ApplicationCore {
			cpu0,
			cpu1,
			cart_loader_barrier
		}
	}
	
	pub fn run(mut self) {
		let args: Vec<String> = args().collect();
		if args.len() < 1 {
			panic!("no boot rom specified!");
		}
		let mut start_pc = {
			let mut file = File::open(&args[1]).unwrap();
			let mut data = Vec::new();
			file.read_to_end(&mut data).unwrap();
			let data_box = data.into_boxed_slice();
			elf_loader::load_elf(data_box.as_ref(), &mut self.cpu0.mio, 0x0000_0000).unwrap()
		};
		
		let ApplicationCore {
			mut cpu0,
			cpu1,
			..
		} = self;
		
		let cpu1_controller = Cpu1Controller::new(cpu1);
		cpu0.mio.set_cpu1_controller(cpu1_controller);
		
		loop {
			cpu0.reset(start_pc);
			cpu0.run_loop(CPU_INSTRUCTIONS_PER_PERIOD, Duration::from_micros(CPU_PERIOD_MICROSECONDS));
			start_pc = self.cart_loader_barrier.wait_barrier();
		}
	}
}