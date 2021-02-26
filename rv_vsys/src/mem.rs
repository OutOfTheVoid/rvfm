pub enum MemReadResult<T> {
	Ok(T),
	ErrUnmapped,
	ErrAlignment,
	ErrSize,
	PeripheralError,
}

impl <T> MemReadResult<T> {
	pub fn unwrap(self) -> T {
		if let Self::Ok(x) = self {
			x
		} else {
			panic!("unwrapped bad MemReadResult!");
		}
	}
}

pub enum MemWriteResult {
	Ok,
	ErrUnmapped,
	ErrReadOnly,
	ErrAlignment,
	ErrSize,
	PeripheralError,
}


impl MemWriteResult {
	pub fn unwrap(self) {
		match self {
			MemWriteResult::Ok => {},
			MemWriteResult::ErrUnmapped => panic!("unwrapped MemWriteResult::ErrUnmapped!"),
			MemWriteResult::ErrReadOnly => panic!("unwrapped MemWriteResult::ErrReadOnly!"),
			MemWriteResult::ErrAlignment => panic!("unwrapped MemWriteResult::ErrAlignment!"),
			MemWriteResult::ErrSize => panic!("unwrapped MemWriteResult::ErrSize!"),
			MemWriteResult::PeripheralError => panic!("unwrapped MemWriteResult::PeripheralError!"),
		}
	}
}

pub trait MemIO: Send + Clone {
	fn read_8(&self, addr: u32) -> MemReadResult<u8>;
	fn read_16(&self, addr: u32) -> MemReadResult<u16>;
	fn read_32(&self, addr: u32) -> MemReadResult<u32>;
	fn read_32_ifetch(&self, addr: u32) -> MemReadResult<u32>;
	
	fn write_8(&mut self, addr: u32, value: u8) -> MemWriteResult;
	fn write_16(&mut self, addr: u32, value: u16) -> MemWriteResult;
	fn write_32(&mut self, addr: u32, value: u32) -> MemWriteResult;
	
	fn access_break(&mut self);
}