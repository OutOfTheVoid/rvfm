use std::time::Instant;
use parking_lot::Mutex;
use rv_vsys::{MemReadResult, MemWriteResult, MTimer};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct MTimerPeripheralData {
	start_time: Instant,
	mtime_at_start: u64,
	mtime_compare: u64,
	
	mtime_atomic_buff: u64,
	mtime_compare_atomic_buff: u64,
}

#[derive(Clone, Debug)]
pub struct MTimerPeripheral {
	data: Arc<Mutex<MTimerPeripheralData>>
}

unsafe impl Sync for MTimerPeripheral {}

const OFFSET_MTIME_LOW: u32 = 0x00;
const OFFSET_MTIME_HIGH: u32 = 0x04;
const OFFSET_MTIME_LOW_ATOMIC_BUFFER: u32 = 0x08;
const OFFSET_MTIME_HIGH_ATOMIC_BUFFER: u32 = 0x0C;
const OFFSET_MTIME_ATOMIC_READ_TRIGGER: u32 = 0x10;
const OFFSET_MTIME_ATOMIC_WRITE_TRIGGER: u32 = 0x14;
const OFFSET_MTIME_ATOMIC_SWAP_TRIGGER: u32 = 0x18;

const OFFSET_MTIMECMP_LOW: u32 = 0x20;
const OFFSET_MTIMECMP_HIGH: u32 = 0x24;
const OFFSET_MTIMECMP_LOW_ATOMIC_BUFFER: u32 = 0x28;
const OFFSET_MTIMECMP_HIGH_ATOMIC_BUFFER: u32 = 0x2C;
const OFFSET_MTIMECMP_ATOMIC_READ_TRIGGER: u32 = 0x30;
const OFFSET_MTIMECMP_ATOMIC_WRITE_TRIGGER: u32 = 0x34;
const OFFSET_MTIMECMP_ATOMIC_SWAP_TRIGGER: u32 = 0x38;

const OFFSET_DUAL_ATOMIC_WRITE_TRIGGER: u32 = 0x40;
const OFFSET_DUAL_ATOMIC_SWAP_TRIGGER: u32 = 0x44;

impl MTimerPeripheral {
	pub fn new() -> Self {
		MTimerPeripheral {
			data: Arc::new(Mutex::new(MTimerPeripheralData {
				start_time: Instant::now(),
				mtime_at_start: 0,
				mtime_compare: 0xFFFF_FFFF_FFFF_FFFF,
				mtime_atomic_buff: 0,
				mtime_compare_atomic_buff: 0xFFFF_FFFF_FFFF_FFFF,
			}))
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		let data = self.data.lock();
		match offset {
			OFFSET_MTIME_LOW => return MemReadResult::Ok(data.mtime_at_start as u32),
			OFFSET_MTIME_HIGH => return MemReadResult::Ok((data.mtime_at_start >> 32) as u32),
			OFFSET_MTIME_LOW_ATOMIC_BUFFER => return MemReadResult::Ok(data.mtime_atomic_buff as u32),
			OFFSET_MTIME_HIGH_ATOMIC_BUFFER => return MemReadResult::Ok((data.mtime_atomic_buff >> 32) as u32),
			OFFSET_MTIME_ATOMIC_READ_TRIGGER | 
			OFFSET_MTIME_ATOMIC_WRITE_TRIGGER | 
			OFFSET_MTIME_ATOMIC_SWAP_TRIGGER => MemReadResult::Ok(0),
			OFFSET_MTIMECMP_LOW => return MemReadResult::Ok(data.mtime_compare as u32),
			OFFSET_MTIMECMP_HIGH => return MemReadResult::Ok((data.mtime_compare >> 32) as u32),
			OFFSET_MTIMECMP_LOW_ATOMIC_BUFFER => return MemReadResult::Ok(data.mtime_compare_atomic_buff as u32),
			OFFSET_MTIMECMP_HIGH_ATOMIC_BUFFER => return MemReadResult::Ok((data.mtime_compare_atomic_buff >> 32) as u32),
			OFFSET_MTIMECMP_ATOMIC_READ_TRIGGER | 
			OFFSET_MTIMECMP_ATOMIC_WRITE_TRIGGER | 
			OFFSET_MTIMECMP_ATOMIC_SWAP_TRIGGER => MemReadResult::Ok(0),
			_ => MemReadResult::PeripheralError,
		}
	}
	
	pub fn write_32(&self, offset: u32, value: u32) -> MemWriteResult {
		let mut data = self.data.lock();
		match offset {
			OFFSET_MTIME_LOW => {
				data.mtime_at_start = (data.mtime_at_start & 0xFFFF_FFFF_0000_0000) | (value as u64);
				data.start_time = Instant::now();
				MemWriteResult::Ok
			},
			OFFSET_MTIME_HIGH => {
				data.mtime_at_start = (data.mtime_at_start & 0x0000_0000_FFFF_FFFF) | ((value as u64) << 32);
				data.start_time = Instant::now();
				MemWriteResult::Ok
			}
			OFFSET_MTIME_LOW_ATOMIC_BUFFER => {
				data.mtime_atomic_buff = (data.mtime_atomic_buff & 0xFFFF_FFFF_0000_0000) | (value as u64);
				MemWriteResult::Ok
			},
			OFFSET_MTIME_HIGH_ATOMIC_BUFFER => {
				data.mtime_atomic_buff = (data.mtime_atomic_buff & 0x0000_0000_FFFF_FFFF) | ((value as u64) << 32);
				MemWriteResult::Ok
			},
			OFFSET_MTIME_ATOMIC_READ_TRIGGER => {
				data.mtime_atomic_buff = data.mtime_at_start;
				data.mtime_atomic_buff += (Instant::now() - data.start_time).as_millis() as u64;
				MemWriteResult::Ok
			},
			OFFSET_MTIME_ATOMIC_WRITE_TRIGGER => {
				data.mtime_at_start = data.mtime_atomic_buff;
				data.start_time = Instant::now();
				MemWriteResult::Ok
			},
			OFFSET_MTIME_ATOMIC_SWAP_TRIGGER => {
				let MTimerPeripheralData {
					start_time,
					mtime_at_start, 
					mtime_atomic_buff,
					..
				} = &mut *data;
				std::mem::swap(mtime_at_start, mtime_atomic_buff);
				*mtime_atomic_buff += (Instant::now() - *start_time).as_millis() as u64;
				data.start_time = Instant::now();
				MemWriteResult::Ok
			},
			
			OFFSET_MTIMECMP_LOW => {
				data.mtime_compare = (data.mtime_compare & 0xFFFF_FFFF_0000_0000) | (value as u64);
				MemWriteResult::Ok
			},
			OFFSET_MTIMECMP_HIGH => {
				data.mtime_compare = (data.mtime_compare & 0x0000_0000_FFFF_FFFF) | ((value as u64) << 32);
				MemWriteResult::Ok
			}
			OFFSET_MTIMECMP_LOW_ATOMIC_BUFFER => {
				data.mtime_compare_atomic_buff = (data.mtime_compare_atomic_buff & 0xFFFF_FFFF_0000_0000) | (value as u64);
				MemWriteResult::Ok
			},
			OFFSET_MTIMECMP_HIGH_ATOMIC_BUFFER => {
				data.mtime_compare_atomic_buff = (data.mtime_compare_atomic_buff & 0x0000_0000_FFFF_FFFF) | ((value as u64) << 32);
				MemWriteResult::Ok
			},
			OFFSET_MTIMECMP_ATOMIC_READ_TRIGGER => {
				data.mtime_compare_atomic_buff = data.mtime_compare;
				MemWriteResult::Ok
			},
			OFFSET_MTIMECMP_ATOMIC_WRITE_TRIGGER => {
				data.mtime_compare = data.mtime_compare_atomic_buff;
				MemWriteResult::Ok
			},
			OFFSET_MTIMECMP_ATOMIC_SWAP_TRIGGER => {
				let MTimerPeripheralData { 
					mtime_compare, 
					mtime_compare_atomic_buff,
					..
				} = &mut *data;
				std::mem::swap(mtime_compare, mtime_compare_atomic_buff);
				MemWriteResult::Ok
			},
			OFFSET_DUAL_ATOMIC_WRITE_TRIGGER => {
				data.mtime_compare = data.mtime_compare_atomic_buff;
				data.mtime_at_start = data.mtime_atomic_buff;
				data.start_time = Instant::now();
				MemWriteResult::Ok
			},
			OFFSET_DUAL_ATOMIC_SWAP_TRIGGER => {
				let MTimerPeripheralData { 
					start_time,
					mtime_at_start,
					mtime_compare, 
					mtime_atomic_buff,
					mtime_compare_atomic_buff,
				} = &mut *data;
				std::mem::swap(mtime_compare, mtime_compare_atomic_buff);
				std::mem::swap(mtime_at_start, mtime_atomic_buff);
				*mtime_atomic_buff += (Instant::now() - *start_time).as_millis() as u64;
				data.start_time = Instant::now();
				MemWriteResult::Ok
			}
			_ => MemWriteResult::ErrUnmapped,
		}
	}
	
	fn get_mtime(data: &MTimerPeripheralData) -> u64 {
		let t = Instant::now() - data.start_time;
		let millis = t.as_millis() as u64;
		millis + data.mtime_at_start
	}
}

impl MTimer for MTimerPeripheral {
	fn check_timer(&self) -> bool {
		let data = self.data.lock();
		let mtime = Self::get_mtime(&*data);
		data.mtime_compare <= mtime
	}
}

