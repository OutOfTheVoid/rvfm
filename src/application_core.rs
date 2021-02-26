use std::{env::args, time::Duration};
use std::fs::File;
use std::io::Read;

use crate::application_gui::ApplicationGUI;

use rv_vsys::{Cpu, CpuWakeupHandle};
use crate::fm_mio::FmMemoryIO;
use crate::fm_interrupt_bus::FmInterruptBus;
use crate::debug_device::DebugDevice;
use crate::elf_loader;

pub struct ApplicationCore {
	cpu: Cpu<FmMemoryIO, FmInterruptBus>,
	gui: ApplicationGUI,
}

impl ApplicationCore {
	pub fn new(gui: ApplicationGUI, mio: FmMemoryIO, interrupt_bus: FmInterruptBus, wakeup_handle: CpuWakeupHandle) -> Self {
		let cpu = Cpu::new(mio, interrupt_bus, wakeup_handle);
		ApplicationCore {
			cpu: cpu,
			gui: gui
		}
	}
	
	pub fn run(&mut self) {
		let args: Vec<String> = args().collect();
		if args.len() < 1 {
			panic!("no rom specified!");
		}
		let start_address = {
			let mut file = File::open(&args[1]).unwrap();
			let mut data = Vec::new();
			file.read_to_end(&mut data).unwrap();
			let data_box = data.into_boxed_slice();
			elf_loader::load_elf(data_box.as_ref(), &mut self.cpu.mio, 0x0000_0000).unwrap()
		};
		
		self.cpu.reset(start_address);
		self.cpu.run_loop(200000, Duration::from_millis(10))
	}
}