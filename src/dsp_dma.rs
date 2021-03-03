#![allow(dead_code)]
use std::{cell::RefCell, cmp::min, usize};

use parking_lot::ReentrantMutex;
use rv_vsys::{MemIO, MemReadResult, MemWriteResult};
use crate::{fm_mio::FmMemoryIO};

#[derive(Debug, Clone, Copy)]
struct DspDmaMemIOParams {
	addr: u32,
	increment: i32,
	restart_count: u32
}

impl DspDmaMemIOParams {
	pub fn get_addr(&self, transfer: u32) -> u32 {
		self.addr.wrapping_add((transfer.wrapping_rem(self.restart_count) as i32).wrapping_mul(self.increment) as u32)
	}
}

#[derive(Debug, Clone, Copy)]
enum DspDmaSource {
	None,
	Mem8(DspDmaMemIOParams),
	Mem16(DspDmaMemIOParams),
	Mem32(DspDmaMemIOParams),
}

#[derive(Debug, Clone, Copy)]
enum DspDmaDest {
	None,
	Mem8(DspDmaMemIOParams),
	Mem16(DspDmaMemIOParams),
	Mem32(DspDmaMemIOParams),
}

#[derive(Debug, Clone, Copy)]
enum DspDmaIOpSource {
	Const(u32),
	IBuffer(u32),
	Source(u32),
}

#[derive(Debug, Clone, Copy)]
enum DspDmaIOpDest {
	IBuffer(u32),
	Dest(u32),
}

#[derive(Debug, Clone, Copy)]
enum DspDmaOp {
	End,
	Copy {source: DspDmaIOpSource, dest: DspDmaIOpDest},
}

#[allow(dead_code)]
pub struct DspDmaDevice {
	src_list: [DspDmaSource; 4],
	dst_list: [DspDmaDest; 4],
	copy_buffer_a: Box<[u32]>,
	copy_buffer_b: Box<[u32]>,
	ibuffer_list: Box<[Box<[u32]>]>,
	fbuffer_list: Box<[Box<[f32]>]>,
	program: [DspDmaOp; 256],
	transfer_size: u32,
	
	mmreg_type: u32,
	mmreg_index: u32,
	mmreg_param0: u32,
	mmreg_param1: u32,
	mmreg_param2: u32,
	mmreg_param3: u32,
	mmreg_param4: u32,
	mmreg_param5: u32,
	mmreg_error: u32,
	mmreg_error_param0: u32,
	mmreg_error_param1: u32,
	mmreg_error_param2: u32,
}

const MAX_PROGRAM_SIZE: u32 = 0x100;
const BUFFER_SIZE: usize = 0x400;
const BUFFER_COUNT: usize = 4;
const SOURCE_COUNT: usize = 4;
const DEST_COUNT: usize = 4;

const REG_TYPE: u32 = 0;
const REG_INDEX: u32 = 4;
const REG_PARAM0: u32 = 8;
const REG_PARAM1: u32 = 12;
const REG_PARAM2: u32 = 16;
const REG_PARAM3: u32 = 20;
const REG_PARAM4: u32 = 24;
const REG_PARAM5: u32 = 28;
const REG_COMMAND: u32 = 32;
const REG_TRANSFER_SIZE: u32 = 36;
const REG_ERROR: u32 = 40;
const REG_ERROR_PARAM0: u32 = 44;
const REG_ERROR_PARAM1: u32 = 48;
const REG_ERROR_PARAM2: u32 = 52;

const SOURCE_TYPE_NONE: u32 = 0;
const SOURCE_TYPE_MEM8: u32 = 1;
const SOURCE_TYPE_MEM16: u32 = 2;
const SOURCE_TYPE_MEM32: u32 = 3;

const DEST_TYPE_NONE: u32 = 0;
const DEST_TYPE_MEM8: u32 = 1;
const DEST_TYPE_MEM16: u32 = 2;
const DEST_TYPE_MEM32: u32 = 3;

const OP_TYPE_END: u32 = 0;
const OP_TYPE_COPY: u32 = 1;

const COMMAND_TRIGGER: u32 = 0;
const COMMAND_WRITE_SOURCE: u32 = 1;
const COMMAND_WRITE_DEST: u32 = 2;
const COMMAND_WRITE_PROGRAM_OP: u32 = 3;

const IOP_SOURCE_TYPE_SOURCE: u32 = 0;
const IOP_SOURCE_TYPE_IBUFFER: u32 = 1;
const IOP_SOURCE_TYPE_CONST: u32 = 2;

const IOP_DEST_TYPE_DEST: u32 = 0;
const IOP_DEST_TYPE_IBUFFER: u32 = 1;

const ERROR_NONE: u32 = 0;
const ERROR_INDEX_OUT_OF_RANGE: u32 = 1;
const ERROR_TYPE_OUT_OF_RANGE: u32 = 2;
const ERROR_PARAM0_OUT_OF_RANGE: u32 = 3;
const ERROR_PARAM1_OUT_OF_RANGE: u32 = 4;
const ERROR_PARAM2_OUT_OF_RANGE: u32 = 5;
const ERROR_SOURCE_OVERLAPS_PERIPHERAL: u32 = 6;
const ERROR_DEST_OVERLAPS_PERIPHERAL: u32 = 7;
const ERROR_TRANSFER_SIZE_TOO_LARGE: u32 = 8;
const ERROR_BAD_COMMAND: u32 = 9;
const ERROR_SOURCE_OUT_OF_RANGE: u32 = 10;
const ERROR_DEST_OUT_OF_RANGE: u32 = 11;
const ERROR_IOP_SOURCE_TYPE_OUT_OF_RANGE: u32 = 12;
const ERROR_IOP_DEST_TYPE_OUT_OF_RANGE: u32 = 13;
const ERROR_USAGE_OF_NULL_SOURCE: u32 = 14;
const ERROR_USAGE_OF_NULL_DEST: u32 = 15;
const ERROR_MEMORY_ACCESS: u32 = 80;

const MEM_ACCESS_ERROR_TYPE_READ: u32 = 0;
const MEM_ACCESS_ERROR_TYPE_WRITE: u32 = 0;

impl DspDmaDevice {
	fn make_ibuffer() -> Box<[u32]> {
		vec![0; BUFFER_SIZE].into_boxed_slice()
	}
	
	fn make_fbuffer() -> Box<[f32]> {
		vec![0f32; BUFFER_SIZE].into_boxed_slice()
	}
	
	pub fn new() -> Self {
		let mut ibuffers = Vec::new();
		let mut fbuffers = Vec::new();
		for _ in 0 .. BUFFER_COUNT {
			ibuffers.push(Self::make_ibuffer());
			fbuffers.push(Self::make_fbuffer());
		}
		DspDmaDevice {
			src_list: [DspDmaSource::None; SOURCE_COUNT],
			dst_list: [DspDmaDest::None; DEST_COUNT],
			copy_buffer_a: Self::make_ibuffer(),
			copy_buffer_b: Self::make_ibuffer(),
			ibuffer_list: ibuffers.into_boxed_slice(),
			fbuffer_list: fbuffers.into_boxed_slice(),
			program: [DspDmaOp::End; MAX_PROGRAM_SIZE as usize],
			transfer_size: 0,
			
			mmreg_type: 0,
			mmreg_index: 0,
			mmreg_param0: 0,
			mmreg_param1: 0,
			mmreg_param2: 0,
			mmreg_param3: 0,
			mmreg_param4: 0,
			mmreg_param5: 0,
			mmreg_error: 0,
			mmreg_error_param0: 0,
			mmreg_error_param1: 0,
			mmreg_error_param2: 0,
		}
	}
	
	fn read_source(&mut self, mio: &mut FmMemoryIO, source: DspDmaSource, source_index: u32, start_transfer: u32, transfer_count: u32, target_buffer: &mut [u32]) -> bool {
		match source {
			DspDmaSource::None => {
				self.mmreg_error = ERROR_USAGE_OF_NULL_SOURCE;
				self.mmreg_error_param0 = source_index;
				false
			},
			DspDmaSource::Mem8(mem_params) => {
				for i in 0 .. transfer_count {
					let addr = mem_params.get_addr(i + start_transfer);
					match mio.read_8(addr) {
						MemReadResult::Ok(val) => target_buffer[i as usize] = val as u32,
						_ => {
							self.mmreg_error = ERROR_MEMORY_ACCESS;
							self.mmreg_error_param0 = addr;
							self.mmreg_error_param1 = MEM_ACCESS_ERROR_TYPE_READ;
							self.mmreg_error_param2 = source_index;
							return false;
						}
					}
				}
				true
			}
			DspDmaSource::Mem16(mem_params) => {
				for i in 0 .. transfer_count {
					let addr = mem_params.get_addr(i + start_transfer);
					match mio.read_16(addr) {
						MemReadResult::Ok(val) => target_buffer[i as usize] = val as u32,
						_ => {
							self.mmreg_error = ERROR_MEMORY_ACCESS;
							self.mmreg_error_param0 = addr;
							self.mmreg_error_param1 = MEM_ACCESS_ERROR_TYPE_READ;
							self.mmreg_error_param2 = source_index;
							return false;
						}
					}
				}
				true
			}
			DspDmaSource::Mem32(mem_params) => {
				for i in 0 .. transfer_count {
					let addr = mem_params.get_addr(i + start_transfer);
					match mio.read_32(addr) {
						MemReadResult::Ok(val) => target_buffer[i as usize] = val,
						_ => {
							self.mmreg_error = ERROR_MEMORY_ACCESS;
							self.mmreg_error_param0 = addr;
							self.mmreg_error_param1 = MEM_ACCESS_ERROR_TYPE_READ;
							self.mmreg_error_param2 = source_index;
							return false;
						}
					}
				}
				true
			}
		}
	}
	
	fn read_op_source(&mut self, mio: &mut FmMemoryIO, source: &DspDmaIOpSource, start_transfer: u32, transfer_count: u32, target_buffer: &mut [u32]) -> bool {
		match source {
			DspDmaIOpSource::Const(value) => {
				for i in 0 .. transfer_count {
					target_buffer[i as usize] = *value;
				}
				true
			},
			DspDmaIOpSource::IBuffer(buffer_index) => {
				let buffer = &mut self.ibuffer_list[*buffer_index as usize];
				target_buffer[0..transfer_count as usize].clone_from_slice(&buffer[0..transfer_count as usize]);
				true
			},
			DspDmaIOpSource::Source(source_index) => {
				self.read_source(mio, self.src_list[*source_index as usize], *source_index, start_transfer, transfer_count, target_buffer)
			}
		}
	}
	
	fn write_dest(&mut self, mio: &mut FmMemoryIO, dest: DspDmaDest, dest_index: u32, start_transfer: u32, transfer_count: u32, source_buffer: &[u32]) -> bool {
		match dest {
			DspDmaDest::None => {
				self.mmreg_error = ERROR_USAGE_OF_NULL_DEST;
				self.mmreg_error_param0 = dest_index;
				false
			},
			DspDmaDest::Mem8(mem_params) => {
				for i in 0 .. transfer_count {
					let addr = mem_params.get_addr(i + start_transfer);
					match mio.write_8(addr, source_buffer[i as usize] as u8) {
						MemWriteResult::Ok => {}
						_ => {
							self.mmreg_error = ERROR_MEMORY_ACCESS;
							self.mmreg_error_param0 = addr;
							self.mmreg_error_param1 = MEM_ACCESS_ERROR_TYPE_WRITE;
							self.mmreg_error_param2 = dest_index;
							return false;
						}
					}
				}
				true
			},
			DspDmaDest::Mem16(mem_params) => {
				for i in 0 .. transfer_count {
					let addr = mem_params.get_addr(i + start_transfer);
					match mio.write_16(addr, source_buffer[i as usize] as u16) {
						MemWriteResult::Ok => {}
						_ => {
							self.mmreg_error = ERROR_MEMORY_ACCESS;
							self.mmreg_error_param0 = addr;
							self.mmreg_error_param1 = MEM_ACCESS_ERROR_TYPE_WRITE;
							self.mmreg_error_param2 = dest_index;
							return false;
						}
					}
				}
				true
			},
			DspDmaDest::Mem32(mem_params) => {
				for i in 0 .. transfer_count {
					let addr = mem_params.get_addr(i + start_transfer);
					match mio.write_32(addr, source_buffer[i as usize]) {
						MemWriteResult::Ok => {}
						_ => {
							self.mmreg_error = ERROR_MEMORY_ACCESS;
							self.mmreg_error_param0 = addr;
							self.mmreg_error_param1 = MEM_ACCESS_ERROR_TYPE_WRITE;
							self.mmreg_error_param2 = dest_index;
							return false;
						}
					}
				}
				true
			}
		}
	}
	
	fn write_op_dest(&mut self, mio: &mut FmMemoryIO, dest: &DspDmaIOpDest, start_transfer: u32, transfer_count: u32, source_buffer: &[u32]) -> bool {
		match dest {
			DspDmaIOpDest::IBuffer(buffer_index) => {
				let target_buffer = &mut self.ibuffer_list[*buffer_index as usize];
				target_buffer[0..transfer_count as usize].clone_from_slice(&source_buffer[0..transfer_count as usize]);
				true
			},
			DspDmaIOpDest::Dest(dest_index) => {
				self.write_dest(mio, self.dst_list[*dest_index as usize], *dest_index, start_transfer, transfer_count, source_buffer)
			}
		}
	}
	
	fn run(&mut self, mio: &mut FmMemoryIO) -> bool {
		let mut transfer_count = 0;
		while transfer_count < self.transfer_size {
			let block_size = min(self.transfer_size - transfer_count, BUFFER_SIZE as u32);
			let mut pc = 0;
			while pc < MAX_PROGRAM_SIZE {
				match self.program[pc as usize] {
					DspDmaOp::End => {
						break;
					},
					DspDmaOp::Copy {source, dest} => {
						if let (&DspDmaIOpSource::IBuffer(src_ibuff), &DspDmaIOpDest::IBuffer(dst_ibuff)) = (&source, &dest) {
							if src_ibuff != dst_ibuff {
								unsafe {
									let src_buffer = &mut *self.ibuffer_list[src_ibuff as usize] as *mut [u32];
									let dst_buffer = &mut *self.ibuffer_list[dst_ibuff as usize] as *mut [u32];
									(*dst_buffer)[0..block_size as usize].clone_from_slice(&(*src_buffer)[0..block_size as usize])
								}
							}
						} else {
							unsafe {
								let copy_buffer = &mut *self.copy_buffer_a as *mut [u32];
								if ! self.read_op_source(mio, &source, transfer_count, block_size, &mut *copy_buffer) {
									return false;
								}
								if ! self.write_op_dest(mio, &dest, transfer_count, block_size, &*copy_buffer) {
									return false;
								}
							}
						}
					}
				}
				pc = pc + 1;
				if pc >= MAX_PROGRAM_SIZE {
					break;
				}
			}
			transfer_count += block_size;
		}
		mio.access_break();
		true
	}
	
	fn parse_iop_source(&mut self, source_type: u32, source_val: u32) -> Option<DspDmaIOpSource> {
		match source_type {
			IOP_SOURCE_TYPE_SOURCE => {
				if source_val >= SOURCE_COUNT as u32 {
					self.mmreg_error = ERROR_SOURCE_OUT_OF_RANGE;
					self.mmreg_error_param0 = source_val;
					return None;
				}
				Some(DspDmaIOpSource::Source(source_val))
			},
			IOP_SOURCE_TYPE_IBUFFER => {
				if source_val >= BUFFER_COUNT as u32 {
					self.mmreg_error = ERROR_SOURCE_OUT_OF_RANGE;
					self.mmreg_error_param0 = source_val;
					return None;
				}
				Some(DspDmaIOpSource::IBuffer(source_val))
			},
			IOP_SOURCE_TYPE_CONST => {
				Some(DspDmaIOpSource::Const(source_val))
			},
			_ => {
				self.mmreg_error = ERROR_IOP_SOURCE_TYPE_OUT_OF_RANGE;
				self.mmreg_error_param0 = source_type;
				None
			}
		}
	}
	
	fn parse_iop_dest(&mut self, dest_type: u32, dest_val: u32) -> Option<DspDmaIOpDest> {
		match dest_type {
			IOP_DEST_TYPE_DEST => {
				if dest_val >= DEST_COUNT as u32 {
					self.mmreg_error = ERROR_SOURCE_OUT_OF_RANGE;
					self.mmreg_error_param0 = dest_val;
					return None;
				}
				Some(DspDmaIOpDest::Dest(dest_val))
			},
			IOP_DEST_TYPE_IBUFFER => {
				if dest_val >= BUFFER_COUNT as u32 {
					self.mmreg_error = ERROR_SOURCE_OUT_OF_RANGE;
					self.mmreg_error_param0 = dest_val;
					return None;
				}
				Some(DspDmaIOpDest::IBuffer(dest_val))
			},
			_ => {
				self.mmreg_error = ERROR_IOP_DEST_TYPE_OUT_OF_RANGE;
				self.mmreg_error_param0 = dest_type;
				None
			}
		}
	}
	
	fn command(&mut self, cmd: u32, mio: &mut FmMemoryIO) -> bool {
		match cmd {
			COMMAND_TRIGGER => {
				self.run(mio)
			},
			COMMAND_WRITE_DEST => {
				if self.mmreg_index >= DEST_COUNT as u32 {
					self.mmreg_error = ERROR_INDEX_OUT_OF_RANGE;
					self.mmreg_error_param0 = self.mmreg_index;
					return false;
				}
				match self.mmreg_type {
					DEST_TYPE_NONE => {
						self.dst_list[self.mmreg_index as usize] = DspDmaDest::None;
						true
					},
					DEST_TYPE_MEM8 => {
						let mem_params = DspDmaMemIOParams {
							addr: self.mmreg_param0,
							increment: self.mmreg_param1 as i32,
							restart_count: self.mmreg_param2
						};
						self.dst_list[self.mmreg_index as usize] = DspDmaDest::Mem8(mem_params);
						true
					},
					DEST_TYPE_MEM16 => {
						let mem_params = DspDmaMemIOParams {
							addr: self.mmreg_param0,
							increment: self.mmreg_param1 as i32,
							restart_count: self.mmreg_param2
						};
						self.dst_list[self.mmreg_index as usize] = DspDmaDest::Mem16(mem_params);
						true
					},
					DEST_TYPE_MEM32 => {
						let mem_params = DspDmaMemIOParams {
							addr: self.mmreg_param0,
							increment: self.mmreg_param1 as i32,
							restart_count: self.mmreg_param2
						};
						self.dst_list[self.mmreg_index as usize] = DspDmaDest::Mem32(mem_params);
						true
					},
					_ => {
						self.mmreg_error = ERROR_TYPE_OUT_OF_RANGE;
						self.mmreg_error_param0 = self.mmreg_type;
						false
					}
				}
			},
			COMMAND_WRITE_SOURCE => {
				if self.mmreg_index >= SOURCE_COUNT as u32 {
					self.mmreg_error = ERROR_INDEX_OUT_OF_RANGE;
					self.mmreg_error_param0 = self.mmreg_index;
					return false;
				}
				match self.mmreg_type {
					SOURCE_TYPE_NONE => {
						self.src_list[self.mmreg_index as usize] = DspDmaSource::None;
						true
					},
					SOURCE_TYPE_MEM8 => {
						let mem_params = DspDmaMemIOParams {
							addr: self.mmreg_param0,
							increment: self.mmreg_param1 as i32,
							restart_count: self.mmreg_param2
						};
						self.src_list[self.mmreg_index as usize] = DspDmaSource::Mem8(mem_params);
						true
					},
					SOURCE_TYPE_MEM16 => {
						let mem_params = DspDmaMemIOParams {
							addr: self.mmreg_param0,
							increment: self.mmreg_param1 as i32,
							restart_count: self.mmreg_param2
						};
						self.src_list[self.mmreg_index as usize] = DspDmaSource::Mem16(mem_params);
						true
					},
					SOURCE_TYPE_MEM32 => {
						let mem_params = DspDmaMemIOParams {
							addr: self.mmreg_param0,
							increment: self.mmreg_param1 as i32,
							restart_count: self.mmreg_param2
						};
						self.src_list[self.mmreg_index as usize] = DspDmaSource::Mem32(mem_params);
						true
					},
					_ => {
						self.mmreg_error = ERROR_TYPE_OUT_OF_RANGE;
						self.mmreg_error_param0 = self.mmreg_type;
						false
					}
				}
			},
			COMMAND_WRITE_PROGRAM_OP => {
				if self.mmreg_index >= MAX_PROGRAM_SIZE as u32 {
					self.mmreg_error = ERROR_INDEX_OUT_OF_RANGE;
					self.mmreg_error_param0 = self.mmreg_index;
					return false;
				}
				match self.mmreg_type {
					OP_TYPE_END => {
						self.program[self.mmreg_index as usize] = DspDmaOp::End;
						true
					},
					OP_TYPE_COPY => {
						let source = match self.parse_iop_source(self.mmreg_param0, self.mmreg_param1) {
							Some(source) => source,
							None => return false,
						};
						let dest = match self.parse_iop_dest(self.mmreg_param2, self.mmreg_param3) {
							Some(dest) => dest,
							None => return false,
						};
						self.program[self.mmreg_index as usize] = DspDmaOp::Copy{
							source,
							dest
						};
						true
					},
					_ => {
						self.mmreg_error = ERROR_TYPE_OUT_OF_RANGE;
						self.mmreg_error_param0 = self.mmreg_type;
						false
					}
				}
			},
			_ => {
				self.mmreg_error = ERROR_BAD_COMMAND;
				self.mmreg_error_param0 = cmd;
				false
			}
		}
	}
	
	pub fn write_32(&mut self, mio: &mut FmMemoryIO, offset: u32, value: u32) -> MemWriteResult {
		match offset {
			REG_TYPE => {
				self.mmreg_type = value;
				MemWriteResult::Ok
			},
			REG_INDEX => {
				self.mmreg_index = value;
				MemWriteResult::Ok
			},
			REG_PARAM0 => {
				self.mmreg_param0 = value;
				MemWriteResult::Ok
			},
			REG_PARAM1 => {
				self.mmreg_param1 = value;
				MemWriteResult::Ok
			},
			REG_PARAM2 => {
				self.mmreg_param2 = value;
				MemWriteResult::Ok
			},
			REG_PARAM3 => {
				self.mmreg_param3 = value;
				MemWriteResult::Ok
			},
			REG_PARAM4 => {
				self.mmreg_param4 = value;
				MemWriteResult::Ok
			},
			REG_PARAM5 => {
				self.mmreg_param5 = value;
				MemWriteResult::Ok
			},
			REG_TRANSFER_SIZE => {
				if value > 0x10000 {
					self.mmreg_error = ERROR_TRANSFER_SIZE_TOO_LARGE;
					self.mmreg_error_param0 = value;
					return MemWriteResult::PeripheralError;
				}
				self.transfer_size = value;
				MemWriteResult::Ok
			},
			REG_COMMAND => {
				if ! self.command(value, mio) {
					println!("DSPDMA ERROR: {:#010x} [{:#010x}, {:#010x}, {:#010x}]", self.mmreg_error, self.mmreg_error_param0, self.mmreg_error_param1, self.mmreg_error_param2);
					MemWriteResult::PeripheralError
				} else {
					MemWriteResult::Ok
				}
			},
			REG_ERROR => {
				self.mmreg_error = 0;
				self.mmreg_error_param0 = 0;
				self.mmreg_error_param1 = 0;
				MemWriteResult::Ok
			},
			REG_ERROR_PARAM0 => MemWriteResult::Ok,
			REG_ERROR_PARAM1 => MemWriteResult::Ok,
			REG_ERROR_PARAM2 => MemWriteResult::Ok,
			_ => {
				MemWriteResult::ErrUnmapped
			},
		}
	}
	
	pub fn read_32(&mut self, offset: u32) -> MemReadResult<u32> {
		match offset {
			REG_TYPE => MemReadResult::Ok(self.mmreg_type),
			REG_INDEX => MemReadResult::Ok(self.mmreg_index),
			REG_PARAM0 => MemReadResult::Ok(self.mmreg_param0),
			REG_PARAM1 => MemReadResult::Ok(self.mmreg_param1),
			REG_PARAM2 => MemReadResult::Ok(self.mmreg_param2),
			REG_PARAM3 => MemReadResult::Ok(self.mmreg_param3),
			REG_PARAM4 => MemReadResult::Ok(self.mmreg_param4),
			REG_PARAM5 => MemReadResult::Ok(self.mmreg_param5),
			REG_COMMAND => MemReadResult::Ok(0),
			REG_TRANSFER_SIZE => MemReadResult::Ok(self.transfer_size),
			REG_ERROR => MemReadResult::Ok(self.mmreg_error),
			REG_ERROR_PARAM0 => MemReadResult::Ok(self.mmreg_error_param0),
			REG_ERROR_PARAM1 => MemReadResult::Ok(self.mmreg_error_param1),
			REG_ERROR_PARAM2 => MemReadResult::Ok(self.mmreg_error_param2),
			_ => {
				MemReadResult::ErrUnmapped
			}
		}
	}
}

pub struct DspDmaDeviceInterface {
	device_lock: ReentrantMutex<RefCell<DspDmaDevice>>
}

unsafe impl Sync for DspDmaDeviceInterface {}
unsafe impl Send for DspDmaDeviceInterface {}

impl DspDmaDeviceInterface {
	pub fn new(device: DspDmaDevice) -> Self {
		Self {
			device_lock: ReentrantMutex::new(RefCell::new(device)), 
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		let gaurd = self.device_lock.lock();
		let result = {
				match gaurd.try_borrow_mut() {
				Ok(mut device) => {
					device.read_32(offset)
				},
				_ =>  {
					MemReadResult::PeripheralError
				}
			}
		};
		result
	}
	
	pub fn write_32(&self, mio: &mut FmMemoryIO, offset: u32, value: u32) -> MemWriteResult {
		let cell = &*self.device_lock.lock();
		let result = {
			match cell.try_borrow_mut() {
				Ok(mut device) => {
					device.write_32(mio, offset, value)
				},
				_ => {
					MemWriteResult::PeripheralError
				}
			}
		};
		result
	}
}
