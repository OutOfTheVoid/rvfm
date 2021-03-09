/* Debug Device
base address: 0xF000_0000

regs:
	0x0000: message_addr: R/W u32 - address of message to be written
	0x0004: message_len:  R/W u32 - length of message to be written
	0x0008: write:        W/0 u32 - write trigger
	0x000C: status:       R/W u32 - status
*/

use rv_vsys::{MemReadResult, MemWriteResult};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct DebugDeviceData {
	message_addr: u32,
	message_len: u32,
	error: u32,
}

#[derive(Debug)]
pub struct DebugDevice {
	data: Arc<Mutex<DebugDeviceData>>
}

unsafe impl Sync for DebugDevice {
}

impl DebugDevice {
	pub const ERROR_SUCCESS: u32 = 0;
	pub const ERROR_MESSAGE_TOO_LONG: u32 = 1;
	pub const ERROR_MESSAGE_NOT_IN_RAM: u32 = 2;
	pub const ERROR_BAD_UTF8: u32 = 3;
	
	pub fn new() -> Self {
		DebugDevice {
			data: Arc::new(Mutex::new(DebugDeviceData {
				message_addr: 0,
				message_len: 0,
				error: Self::ERROR_SUCCESS
			}))
		}
	}
	
	fn print_debug_message(&mut self, ram: &[u8]) -> bool {
		let mut data = self.data.lock().unwrap();
		if data.message_len > 0x1000 {
			if cfg!(rvfm_debug_device_debug) { println!("DEBUG DEVICE ERROR: message too long: {:#010x}", data.message_len); }
			data.error = Self::ERROR_MESSAGE_TOO_LONG;
			return false;
		}
		let mut message_vec = Vec::new();
		if cfg!(rvfm_debug_device_debug) { print!("Debug message bytes: "); }
		for i in 0 .. data.message_len {
			let addr = data.message_addr + i;
			if addr >= 0x1000_0000 {
				println!("DEBUG DEVICE ERROR: message not in ram");
				data.error = Self::ERROR_MESSAGE_NOT_IN_RAM;
				return false;
			}
			if cfg!(rvfm_debug_device_debug) { print!(" {:02x}", ram[addr as usize]); }
			message_vec.push(ram[addr as usize]);
		}
		if let Ok(message) = String::from_utf8(message_vec) {
			println!("DEBUG: {}", message);
			true
		} else {
			data.error = Self::ERROR_BAD_UTF8;
			false
		}
	}
	
	fn print_debug_u32(&mut self) {
		let data = self.data.lock().unwrap();
		println!("DEBUG: {}", data.message_addr);
	}
	
	fn print_debug_u32_hex(&mut self) {
		let data = self.data.lock().unwrap();
		println!("DEBUG: {:#010x}", data.message_addr);
	}
	
	fn print_debug_f32(&mut self) {
		let data = self.data.lock().unwrap();
		println!("DEBUG: {}", f32::from_bits(data.message_addr));
	}
	// feb 19th 1:00 pm
	pub fn write_32(&mut self, offset: u32, value: u32, ram: &[u8]) -> MemWriteResult{
		if cfg!(rvfm_debug_device_debug) { println!("Debug Device Write: offset: {:#06x}, value: {:#010x}", offset, value); }
		if offset > 0x000C {
			MemWriteResult::ErrUnmapped
		} else {
			match offset {
				0x0000 => {
					let mut data = self.data.lock().unwrap();
					data.message_addr = value;
					MemWriteResult::Ok
				},
				0x0004 => {
					let mut data = self.data.lock().unwrap();
					data.message_len = value;
					MemWriteResult::Ok
				},
				0x0008 => {
					match value {
						0 => {
							if self.print_debug_message(ram) {
								let mut data = self.data.lock().unwrap();
								data.error = 0;
								MemWriteResult::Ok
							} else {
								MemWriteResult::PeripheralError
							}
						},
						1 => {
							self.print_debug_u32();
							MemWriteResult::Ok
						},
						2 => {
							self.print_debug_f32();
							MemWriteResult::Ok
						},
						3 => {
							self.print_debug_u32_hex();
							MemWriteResult::Ok
						}
						_ => MemWriteResult::Ok
					}
				},
				0x000C => {
					if value == 0 {
						let mut data = self.data.lock().unwrap();
						data.error = 0;
					}
					MemWriteResult::Ok
				},
				_ => {
					MemWriteResult::ErrAlignment
				}
			}
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		if offset > 0x000C {
			MemReadResult::ErrUnmapped
		} else {
			match offset {
				0x0000 => {
					let data = self.data.lock().unwrap();
					MemReadResult::Ok(data.message_addr)
				},
				0x0004 => {
					let data = self.data.lock().unwrap();
					MemReadResult::Ok(data.message_len)
				},
				0x0008 => {
					MemReadResult::Ok(0)
				},
				0x000C => {
					let data = self.data.lock().unwrap();
					MemReadResult::Ok(data.error)
				},
				_ => {
					MemReadResult::ErrAlignment
				}
			}
		}
	}
}