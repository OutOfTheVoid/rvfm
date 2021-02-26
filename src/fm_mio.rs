use std::{cell::{UnsafeCell, RefCell}, fmt::Write, ops::{Deref, DerefMut}, sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}}};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use core::convert::{AsMut, AsRef};
use atomic_counter::{AtomicCounter, ConsistentCounter};

use rv_vsys::{MemIO, MemReadResult, MemWriteResult};
use byteorder::{LE, ByteOrder};
use crate::{debug_device::DebugDevice, gpu::GpuPeripheralInterface};
use once_cell::sync::OnceCell;

const RAM_SIZE: usize = 0x1000_0000;
const LOCK_GRANULARITY: usize = 0x1000;

struct ArcMutPtr<T: ?Sized> {
	data_ptr: *mut T,
	data_owner: Arc<Mutex<Box<T>>>
}

impl<T: ?Sized> ArcMutPtr<T> {
	pub fn new(mut val: Box<T>) -> Self {
		let data_ptr: *mut T = {
			let val_ref = &mut val;
				val_ref.as_mut()
		};
		ArcMutPtr {
			data_owner: Arc::new(Mutex::new(val)),
			data_ptr: data_ptr
		}
	}
	
	pub fn deref_mut_static(&self) -> &'static mut T {
		unsafe {
			&mut *self.data_ptr
		}
	}
}

impl<T: ?Sized> Clone for ArcMutPtr<T> {
	fn clone(&self) -> Self {
		ArcMutPtr {
			data_owner: self.data_owner.clone(),
			data_ptr: self.data_ptr
		}
	}
}

impl<T: ?Sized> Deref for ArcMutPtr<T> {
	type Target = T;
	fn deref(&self) -> & T {
		unsafe {
			&*self.data_ptr
		}
	}
}

impl<T: ?Sized> DerefMut for ArcMutPtr<T> {
	fn deref_mut(&mut self) -> &mut T {
		unsafe {
			&mut *self.data_ptr
		}
	}
}

impl <T: ?Sized> AsRef<T> for ArcMutPtr<T> {
	fn as_ref(&self) -> &T {
		unsafe {
			&*self.data_ptr
		}
	}
}

unsafe impl<T: ?Sized> Send for ArcMutPtr<T> {
}

enum MemLockHold {
	Read(u32, RwLockReadGuard<'static, ()>),
	Write(u32, RwLockWriteGuard<'static, ()>, Arc<AtomicUsize>, Arc<ConsistentCounter>, ),
	Clear,
}

impl Drop for MemLockHold {
	fn drop(&mut self) {
		match self {
			MemLockHold::Write(_, write_gaurd, write_cycle, write_cycle_counter) => {
				let cycle = write_cycle_counter.inc();
				write_cycle.store(cycle, Ordering::SeqCst);
			},
			MemLockHold::Read(_, read_gaurd) => {
			}
			_ => {}
		}
	}
}

pub struct FmMemoryIO {
	ram: ArcMutPtr<[u8]>,
	page_locks: ArcMutPtr<[Arc<PageGaurd>]>,
	debug_device: ArcMutPtr<DebugDevice>,
	gpu_interface_device: Arc<OnceCell<GpuPeripheralInterface>>,
	mem_lock_hold_d: UnsafeCell<MemLockHold>,
	mem_lock_hold_i: UnsafeCell<MemLockHold>,
	write_cycle_counter: Arc<ConsistentCounter>,
	id_counter: Arc<ConsistentCounter>,
	interface_id: u32,
}

unsafe impl Send for FmMemoryIO {
}

struct PageGaurd {
	pub lock: Arc<RwLock<()>>,
	pub write_cycle: Arc<AtomicUsize>,
}

impl PageGaurd {
	pub fn new() -> Self {
		PageGaurd {
			lock: Arc::new(RwLock::new(())),
			write_cycle: Arc::new(AtomicUsize::new(0)),
		}
	}
}

impl Clone for PageGaurd {
	fn clone(&self) -> Self {
		PageGaurd {
			lock: Arc::new(RwLock::new(())),
			write_cycle: self.write_cycle.clone(),
		}
	}
}

impl Clone for FmMemoryIO {
	fn clone(&self) -> Self {
		FmMemoryIO {
			ram: self.ram.clone(),
			page_locks: self.page_locks.clone(),
			debug_device: self.debug_device.clone(),
			gpu_interface_device: self.gpu_interface_device.clone(),
			mem_lock_hold_d: UnsafeCell::new(MemLockHold::Clear),
			mem_lock_hold_i: UnsafeCell::new(MemLockHold::Clear),
			write_cycle_counter: self.write_cycle_counter.clone(),
			id_counter: self.id_counter.clone(),
			interface_id: self.id_counter.inc() as u32
		}
	}
}

impl FmMemoryIO {
	pub fn new() -> FmMemoryIO {
		let mut lock_vec = Vec::new();
		for _ in 0 .. (RAM_SIZE / LOCK_GRANULARITY) {
			lock_vec.push(Arc::new(PageGaurd::new()));
		}
		// the problem i'm having is that page_locks is initialized with clone(), meaning every page shares the same Arc'd PageGaurd
		// to solve, fill lock_vec with individual Arc<PageGaurd>'s constructed separately
		FmMemoryIO {
			ram: ArcMutPtr::new(vec![0u8; RAM_SIZE].into_boxed_slice()),
			page_locks: ArcMutPtr::new(lock_vec.into_boxed_slice()),
			debug_device: ArcMutPtr::new(Box::new(DebugDevice::new())),
			gpu_interface_device: Arc::new(OnceCell::default()),
			mem_lock_hold_d: UnsafeCell::new(MemLockHold::Clear),
			mem_lock_hold_i: UnsafeCell::new(MemLockHold::Clear),
			write_cycle_counter: Arc::new(ConsistentCounter::new(1)),
			id_counter: Arc::new(ConsistentCounter::new(1)),
			interface_id: 0
		}
	}
	
	pub fn ram_sync_read(&self, addr: u32) {
		let page_num = addr / 0x1000;
		unsafe {
			match &*self.mem_lock_hold_i.get() {
				&MemLockHold::Read(page, _) => if page == page_num {
					return;
				},
				_ => {}
			}
			match &*self.mem_lock_hold_d.get() {
				&MemLockHold::Read(page, _) => if page != page_num {
					*self.mem_lock_hold_d.get() = MemLockHold::Clear;
					*self.mem_lock_hold_d.get() = MemLockHold::Read(page_num, self.page_locks.deref_mut_static()[page_num as usize].lock.read());
				},
				&MemLockHold::Write(page, _, _, _) => if page != page_num {
					*self.mem_lock_hold_d.get() = MemLockHold::Clear;
					*self.mem_lock_hold_d.get() = MemLockHold::Read(page_num, self.page_locks.deref_mut_static()[page_num as usize].lock.read());
				},
				&MemLockHold::Clear => {
					*self.mem_lock_hold_d.get() = MemLockHold::Read(page_num, self.page_locks.deref_mut_static()[page_num as usize].lock.read());
				}
			}
		}
	}
	
	pub fn ram_sync_read_ifetch(&self, addr: u32) {
		let page_num = addr / 0x1000;
		unsafe {
			match &*self.mem_lock_hold_d.get() {
				&MemLockHold::Read(page, _) => if page == page_num {
					return;
				},
				&MemLockHold::Write(page, _, _, _) => if page == page_num {
					return;
				},
				_ => {}
			}
			match &*self.mem_lock_hold_i.get() {
				&MemLockHold::Read(page, _) => if page != page_num {
					*self.mem_lock_hold_i.get() = MemLockHold::Clear;
					*self.mem_lock_hold_i.get() = MemLockHold::Read(page_num, self.page_locks.deref_mut_static()[page_num as usize].lock.read());
				},
				&MemLockHold::Write(_, _, _, _) => {
					panic!("instruction mem lock hold should never have Write status!");
				},
				&MemLockHold::Clear => {
					*self.mem_lock_hold_i.get() = MemLockHold::Read(page_num, self.page_locks.deref_mut_static()[page_num as usize].lock.read());
				}
			}
		}
	}
	
	pub fn ram_sync_write(&self, addr: u32) {
		let page_num = addr / 0x1000;
		unsafe {
			match &*self.mem_lock_hold_i.get() {
				&MemLockHold::Read(page, _) => if page == page_num {
					*self.mem_lock_hold_i.get() = MemLockHold::Clear;
				},
				_ => {}
			}
			match &*self.mem_lock_hold_d.get() {
				&MemLockHold::Read(page, _) => {
					*self.mem_lock_hold_d.get() = MemLockHold::Clear;
					let page_gaurd = &self.page_locks.deref_mut_static()[page_num as usize];
					*self.mem_lock_hold_d.get() = MemLockHold::Write(page_num, page_gaurd.lock.write(), page_gaurd.write_cycle.clone(), self.write_cycle_counter.clone());
				},
				&MemLockHold::Write(page, _, _, _) => if page != page_num {
					*self.mem_lock_hold_d.get() = MemLockHold::Clear;
					let page_gaurd = &self.page_locks.deref_mut_static()[page_num as usize];
					*self.mem_lock_hold_d.get() = MemLockHold::Write(page_num, page_gaurd.lock.write(), page_gaurd.write_cycle.clone(), self.write_cycle_counter.clone());
				},
				&MemLockHold::Clear => {
					let page_gaurd = &self.page_locks.deref_mut_static()[page_num as usize];
					let lock_gaurd = page_gaurd.lock.write();
					*self.mem_lock_hold_d.get() = MemLockHold::Write(page_num, lock_gaurd, page_gaurd.write_cycle.clone(), self.write_cycle_counter.clone());
				}
			}
		}
	}
	
	pub fn get_page_write_cycle(&self, addr: u32) -> usize {
		let page_num = addr / 0x1000;
		self.page_locks.deref_mut_static()[page_num as usize].write_cycle.load(Ordering::SeqCst)
	}
	
	pub fn raw_ram_ptr(&mut self, addr: u32) -> MemReadResult<*const u8> {
		if addr < 0x1000_0000 {
			MemReadResult::Ok(self.ram.as_mut()[addr as usize..].as_ptr())
		} else {
			MemReadResult::ErrUnmapped
		}
	}
	
	pub fn set_gpu_interface(&mut self, interface: GpuPeripheralInterface) {
		self.gpu_interface_device.set(interface);
	}
}

impl MemIO for FmMemoryIO {
	fn access_break(&mut self) {
		unsafe {
			match &*self.mem_lock_hold_d.get() {
				MemLockHold::Read(page, _) => {
					*self.mem_lock_hold_d.get() = MemLockHold::Clear;
				},
				MemLockHold::Write(page, ..) => {
					*self.mem_lock_hold_d.get() = MemLockHold::Clear;
				},
				_ => {},
			}
			match &*self.mem_lock_hold_i.get() {
				MemLockHold::Read(page, _) => {
					*self.mem_lock_hold_i.get() = MemLockHold::Clear;
				},
				MemLockHold::Write(..) => {
					panic!("instruction mem lock hold should never have Write status!");
				}
				_ => {},
			}
		}
	}
	
	fn read_8(&self, addr: u32) -> MemReadResult<u8> {
		if addr == 0 {
			return MemReadResult::ErrUnmapped;
		}
		match addr >> 28 {
			0 => {
				self.ram_sync_read(addr);
				MemReadResult::Ok(self.ram.as_ref()[addr as usize])
			},
			_ => MemReadResult::ErrUnmapped
		}
	}

	fn read_16(&self, addr: u32) -> MemReadResult<u16> {
		if addr == 0 {
			return MemReadResult::ErrUnmapped;
		}
		match (addr + 1) >> 28 {
			0 => {
				self.ram_sync_read(addr);
				MemReadResult::Ok(LE::read_u16(&self.ram.as_ref()[addr as usize ..]))
			},
			_ => MemReadResult::ErrUnmapped
		}
	}

	fn read_32(&self, addr: u32) -> MemReadResult<u32> {
		if addr == 0 {
			return MemReadResult::ErrUnmapped;
		}
		match (addr + 3) >> 28 {
			0 => {
				self.ram_sync_read(addr);
				MemReadResult::Ok(LE::read_u32(&self.ram.as_ref()[addr as usize ..]))
			},
			0xF => {
				let peripheral_offset = addr & 0xFFFF;
				let peripheral = (addr >> 16) & 0xFFF;
				match peripheral {
					0 => {
						self.debug_device.as_ref().read_32(peripheral_offset)
					},
					_ => {
						MemReadResult::ErrUnmapped
					}
				}
			},
			_ => MemReadResult::ErrUnmapped,
		}
	}
	
	fn read_32_ifetch(&self, addr: u32) -> MemReadResult<u32> {
		if addr == 0 {
			return MemReadResult::ErrUnmapped;
		}
		match (addr + 3) >> 28 {
			0 => {
				self.ram_sync_read_ifetch(addr);
				MemReadResult::Ok(LE::read_u32(&self.ram.as_ref()[addr as usize ..]))
			},
			0xF => {
				let peripheral_offset = addr & 0xFFFF;
				let peripheral = (addr >> 16) & 0xFFF;
				match peripheral {
					0 => {
						self.debug_device.as_ref().read_32(peripheral_offset)
					},
					_ => {
						MemReadResult::ErrUnmapped
					}
				}
			},
			_ => MemReadResult::ErrUnmapped,
		}
	}
	
	fn write_8(&mut self, addr: u32, value: u8) -> MemWriteResult {
		if addr == 0 {
			return MemWriteResult::ErrUnmapped;
		}
		match addr >> 28 {
			0 => {
				self.ram_sync_write(addr);
				self.ram.as_mut()[addr as usize] = value;
				MemWriteResult::Ok
			}
			_ => {
				MemWriteResult::ErrUnmapped
			}
		}
	}

	fn write_16(&mut self, addr: u32, value: u16) -> MemWriteResult {
		if addr == 0 {
			return MemWriteResult::ErrUnmapped;
		}
		match (addr + 1) >> 28 {
			0 => {
				self.ram_sync_write(addr);
				LE::write_u16(&mut self.ram.as_mut()[addr as usize ..], value);
				MemWriteResult::Ok
			}
			_ => MemWriteResult::ErrUnmapped
		}
	}

	fn write_32(&mut self, addr: u32, value: u32) -> MemWriteResult {
		if addr == 0 {
			return MemWriteResult::ErrUnmapped;
		}
		match (addr + 3) >> 28 {
			0 => {
				self.ram_sync_write(addr);
				LE::write_u32(&mut self.ram.as_mut()[addr as usize ..], value);
				MemWriteResult::Ok
			},
			0xF => {
				let peripheral_offset = addr & 0xFFFF;
				let peripheral = (addr >> 16) & 0xFFF;
				match peripheral {
					0 => {
						self.debug_device.deref_mut().write_32(peripheral_offset, value, &self.ram.as_ref())
					},
					1 => {
						self.gpu_interface_device.get().unwrap().clone().write_u32(peripheral_offset, value)
					},
					_ => {
						MemWriteResult::ErrUnmapped
					}
				}
			}
			_ => MemWriteResult::ErrUnmapped
		}
	}
}
