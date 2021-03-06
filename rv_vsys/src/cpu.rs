#![allow(dead_code)]
use crate::{MemIO, MemReadResult, MemWriteResult, Opcode, Op, OpImmFunct3, StoreFunct3, LoadFunct3, OpFunct3Funct7, BranchFunct3, LoadFpFunct3, StoreFpFunct3, SystemFunct3, SystemIntFunct7, InterruptBus, FpFunct7};
use std::{time::{Duration, Instant}, sync::Arc};
use parking_lot::{Condvar, Mutex};

const REG_NAMES: [&str; 32] = [
	"zero",
	"ra",
	"sp",
	"gp",
	"tp",
	"t0",
	"t1",
	"t2",
	"s0",
	"s1",
	"a0",
	"a1",
	"a2",
	"a3",
	"a4",
	"a5",
	"a6",
	"a7",
	"s2",
	"s3",
	"s4",
	"s5",
	"s6",
	"s7",
	"s8",
	"s9",
	"s10",
	"s11",
	"t3",
	"t4",
	"t5",
	"t6",
];

const REG_NAMES_PAD: [&str; 32] = [
	"zero",
	"ra  ",
	"sp  ",
	"gp  ",
	"tp  ",
	"t0  ",
	"t1  ",
	"t2  ",
	"s0  ",
	"s1  ",
	"a0  ",
	"a1  ",
	"a2  ",
	"a3  ",
	"a4  ",
	"a5  ",
	"a6  ",
	"a7  ",
	"s2  ",
	"s3  ",
	"s4  ",
	"s5  ",
	"s6  ",
	"s7  ",
	"s8  ",
	"s9  ",
	"s10 ",
	"s11 ",
	"t3  ",
	"t4  ",
	"t5  ",
	"t6  ",
];

const MSTATUS_MIE: u32 = 1 << 3;
const MSTATUS_MPIE: u32 = 1 << 7;
const MSTATUS_MPP: u32 = 3 << 11;
const MSTATUS_FS_MASK: u32 = 3 << 13;
const MSTATUS_FS_OFF: u32 = 0b00 << 13;
const MSTATUS_FS_INITIAL: u32 = 0b01 << 13;
const MSTATUS_FS_CLEAN: u32 = 0b10 << 13;
const MSTATUS_FS_DIRTY: u32 = 0b11 << 13;
const MSTATUS_SD: u32 = 1 << 31;

const MIE_MSIE: u32 = 1 << 3;
const MIE_MTIE: u32 = 1 << 7;
const MIE_MEIE: u32 = 1 << 11;

const MIP_MSIP: u32 = 1 << 3;
const MIP_MTIP: u32 = 1 << 7;
const MIP_MEIP: u32 = 1 << 11;

#[derive(Clone, Copy, Debug)]
enum Exception {
	InstructionMisaligned(u32),
	InstructionAccessFault(u32),
	IllegalInstruction{op: u32, addr: u32},
	Breakpoint(u32),
	LoadAddressMisaligned{instr_addr: u32, load_addr: u32},
	LoadAccessFault{instr_addr: u32, load_addr: u32},
	StoreAddressMisaligned{instr_addr: u32, store_addr: u32},
	StoreAccessFault{instr_addr: u32, store_addr: u32},
	ECall(u32),
}

enum InterruptType {
	Timer,
	Software,
	External,
	Platform(u32),
	Exception(Exception)
}

struct TrapCSRs {
	mstatus: u32,
	mie: u32,
	mtvec: u32,
	
	mscratch: u32,
	mepc: u32,
	mcause: u32,
	mtval: u32,
	mip: u32,
}

impl TrapCSRs {
	pub fn new() -> Self {
		Self {
			mstatus: MSTATUS_MPP,
			mie: 0,
			mtvec: 0,
			
			mscratch: 0,
			mepc: 0,
			mcause: 0,
			mtval: 0,
			mip: 0,
		}
	}
	
	pub fn reset(&mut self) {
		*self = Self::new();
	}
}

const FFLAG_INEXACT: u32 = 0x01;
const FFLAG_UNDERFLOW: u32 = 0x02;
const FFLAG_OVERFLOW: u32 = 0x04;
const FFLAG_DIVIDE_BY_ZERO: u32 = 0x08;
const FFLAG_INVALID: u32 = 0x10;

struct PendingInt {
	pc: u32,
	cause: u32,
	int_type: InterruptType,
	tval: u32,
}

pub struct Cpu <MIO: MemIO, IntBus: InterruptBus> {
	xr: [u32; 31],
	fr: [f32; 32],
	pc: u32,
	pub mio: MIO,
	pub int_bus: IntBus,
	csr_instrret: u64,
	csr_cycle: u64,
	csr_time_start: Instant,
	hart_id: u32,
	trap_csrs: TrapCSRs,
	pending_exception: Option<Exception>,
	waiting_for_interrupt: bool,
	wakeup_handle: CpuWakeupHandle,
	fcsr: u32,
}

#[derive(Debug, Clone)]
pub struct CpuWakeupHandle {
	wakeup_cond: Arc<Condvar>,
	wakeup_lock: Arc<Mutex<()>>
}

unsafe impl Sync for CpuWakeupHandle {}
unsafe impl Send for CpuWakeupHandle {}

impl CpuWakeupHandle {
	pub fn new() -> Self {
		Self {
			wakeup_cond: Arc::new(Condvar::new()),
			wakeup_lock: Arc::new(Mutex::new(()))
		}
	}
	
	pub fn cpu_wait(&mut self, until: Instant) {
		let mut gaurd = self.wakeup_lock.lock();
		self.wakeup_cond.wait_until(&mut gaurd, until);
	}
	
	pub fn cpu_wake(&mut self) {
		let _lock_gaurd = self.wakeup_lock.lock();
		self.wakeup_cond.notify_all();
	}
}

impl <MIO: MemIO, IntBus: InterruptBus> Cpu<MIO, IntBus> {
	pub fn new(mio: MIO, int_bus: IntBus, wakeup_handle: CpuWakeupHandle, id: u32) -> Cpu<MIO, IntBus> {
		Cpu {
			xr: [0; 31],
			fr: [0f32; 32],
			pc: 0,
			mio: mio,
			int_bus: int_bus,
			csr_instrret: 0,
			csr_cycle: 0,
			csr_time_start: Instant::now(),
			hart_id: id,
			trap_csrs: TrapCSRs::new(),
			pending_exception: None,
			waiting_for_interrupt: false,
			wakeup_handle: wakeup_handle,
			fcsr: 0
		}
	}

	pub fn reset(&mut self, pc: u32) {
		self.step_break();
		for i in 0 .. 31 {
			self.xr[i] = 0;
		}
		for i in 0 .. 32 {
			self.fr[i] = 0f32;
		}
		self.pc = pc;
		self.trap_csrs.reset();
		self.pending_exception = None;
		self.waiting_for_interrupt = false;
	}
	
	pub fn check_timer(&mut self) {
		//let t = self.get_time();
		// todo...
	}
	
	pub fn run_loop(&mut self, inst_per_period: u32, period_length: Duration) {
		let mut loop_t = Instant::now();
		loop {
			self.check_timer();
			if self.int_bus.poll_interrupts(self.hart_id) {
				self.signal_external_interrupt();
			}
			'period_loop: for _ in 0 .. inst_per_period {
				if ! self.step() {
					break 'period_loop;
				}
			}
			if cfg!(feature = "cpu_debug") { 
				if let Some(exception) = self.pending_exception {
					print!("exception @{:#010x}: {:?}", self.pc, exception);
				}
			}
			self.step_break();
			let current_t = Instant::now();
			let period_time_elapsed = current_t - loop_t;
			if period_time_elapsed < period_length {
				self.wakeup_handle.cpu_wait(Instant::now() + (period_length - period_time_elapsed));
			}
			loop_t = Instant::now();
		}
	}
	
	fn get_trap_vector_addr(&self, vector: u32) -> u32 {
		match self.trap_csrs.mtvec & 3 {
			0 => self.trap_csrs.mtvec,
			1 => (self.trap_csrs.mtvec & !3u32) + 4 * vector,
			_ => panic!("unimplemented mtvec!")
		}
	}
	
	fn handle_interrupts(&mut self, ) {
		if let Some(exception) = self.pending_exception {
			let (cause, tval, pc) = match exception {
				Exception::InstructionMisaligned(pc) => (0, pc, pc),
				Exception::InstructionAccessFault(pc) => (1, pc, pc),
				Exception::IllegalInstruction{op, addr} => (2, op, addr),
				Exception::Breakpoint(pc) => (3, pc, pc),
				Exception::LoadAddressMisaligned{instr_addr, load_addr} => (4, load_addr, instr_addr),
				Exception::LoadAccessFault{instr_addr, load_addr} => (5, load_addr, instr_addr),
				Exception::StoreAddressMisaligned{instr_addr, store_addr} => (6, store_addr, instr_addr),
				Exception::StoreAccessFault{instr_addr, store_addr} => (7, store_addr, instr_addr),
				Exception::ECall(pc) => (11, 0, pc),
			};
			self.trap_csrs.mepc = pc;
			self.trap_csrs.mcause = cause;
			self.trap_csrs.mtval = tval;
			self.pc = self.get_trap_vector_addr(0);
			let ie_before = (self.trap_csrs.mstatus & MSTATUS_MIE) != 0;
			self.trap_csrs.mstatus &= !(MSTATUS_MIE | MSTATUS_MPIE);
			self.trap_csrs.mstatus |= if ie_before {MSTATUS_MPIE} else {0};
			self.pending_exception = None;
		}
		if (self.trap_csrs.mstatus & MSTATUS_MIE) != 0 {
			let pending_interrupt_bits = self.trap_csrs.mip & self.trap_csrs.mie;
			if pending_interrupt_bits & MIP_MEIP != 0 {
				self.trap_csrs.mepc = self.pc;
				self.trap_csrs.mcause = 0x8000_000B;
				self.pc = self.get_trap_vector_addr(11);
				self.trap_csrs.mtval = 0;
				let ie_before = (self.trap_csrs.mstatus & MSTATUS_MIE) != 0;
				self.trap_csrs.mstatus &= !(MSTATUS_MIE | MSTATUS_MPIE);
				self.trap_csrs.mstatus |= if ie_before {MSTATUS_MPIE} else {0};
				self.pending_exception = None;
				self.waiting_for_interrupt = false;
			} else if pending_interrupt_bits & MIP_MSIP != 0 {
				self.trap_csrs.mepc = self.pc;
				self.trap_csrs.mcause = 0x8000_0003;
				self.pc = self.get_trap_vector_addr(3);
				self.trap_csrs.mtval = 0;
				let ie_before = (self.trap_csrs.mstatus & MSTATUS_MIE) != 0;
				self.trap_csrs.mstatus &= !(MSTATUS_MIE | MSTATUS_MPIE);
				self.trap_csrs.mstatus |= if ie_before {MSTATUS_MPIE} else {0};
				self.pending_exception = None;
				self.waiting_for_interrupt = false;
			} else if pending_interrupt_bits & MIP_MTIP != 0 {
				self.trap_csrs.mepc = self.pc;
				self.trap_csrs.mcause = 0x8000_0007;
				self.pc = self.get_trap_vector_addr(7);
				self.trap_csrs.mtval = 0;
				let ie_before = (self.trap_csrs.mstatus & MSTATUS_MIE) != 0;
				self.trap_csrs.mstatus &= !(MSTATUS_MIE | MSTATUS_MPIE);
				self.trap_csrs.mstatus |= if ie_before {MSTATUS_MPIE} else {0};
				self.pending_exception = None;
				self.waiting_for_interrupt = false;
			}
		}
	}
	
	pub fn signal_external_interrupt(&mut self) {
		self.trap_csrs.mip |= MIP_MEIP;
	}

	pub fn step(&mut self) -> bool {
		if self.trap_csrs.mip != 0 || self.pending_exception.is_some() {
			self.handle_interrupts();
		}
		if self.waiting_for_interrupt {
			return false;
		}
		let pc = self.pc;
		let opcode_value = match self.mio.read_32_ifetch(pc) {
			MemReadResult::Ok(value) => value,
			_ => return false
		};
		if cfg!(feature = "cpu_debug") { print!("step @{:#010x}: {:02x} {:02x} {:02x} {:02x}  | ", pc, opcode_value & 0xFF, (opcode_value >> 8) & 0xFF, (opcode_value >> 16) & 0xFF, opcode_value >> 24); }
		let opcode = Opcode::new(opcode_value);
		match opcode.op() {
			Op::Lui => {
				let imm = opcode.u_imm() << 12;
				let rd = opcode.rd();
				self.set_gpr(rd, imm);
				self.pc += 4;
				if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("lui {}, {:#07x}", REG_NAMES[rd as usize], opcode.u_imm()), REG_NAMES[rd as usize], imm); }
			},
			Op::Auipc => {
				let imm = opcode.u_imm() << 12;
				let pc = self.pc;
				let val = imm.wrapping_add(pc);
				let rd = opcode.rd();
				self.set_gpr(rd, val);
				self.pc += 4;
				if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("auipc {}, imm: {:#07x}", REG_NAMES[rd as usize], opcode.u_imm()), REG_NAMES[rd as usize], val); }
			}
			Op::Jal => {
				let imm = opcode.j_imm_signed();
				let return_addr = self.pc + 4;
				let rd = opcode.rd();
				self.set_gpr(rd, return_addr);
				self.pc = self.pc.wrapping_add(imm as u32);
				if cfg!(feature = "cpu_debug") { println!("{: <50} # pc           <=  {:#010x}     {: <12} <= {:#010x}", format!("jal {}, {:#010x}", REG_NAMES[rd as usize], self.pc), self.pc, REG_NAMES[rd as usize], return_addr); }
			},
			Op::Jalr => {
				let imm = opcode.i_imm_signed();
				let rs1 = opcode.rs1();
				let rd = opcode.rd();
				let base = self.get_gpr(rs1);
				let return_addr = self.pc + 4;
				self.set_gpr(rd, return_addr);
				self.pc = base.wrapping_add(imm as u32);
				if cfg!(feature = "cpu_debug") { println!("{: <50} # pc           <=  {:#010x}     {: <12} <= {:#010x}", format!("jalr {}, {}({})", REG_NAMES[rd as usize], if imm == 0 {"".to_string()} else {format!("{:#03x}", imm).to_string()}, REG_NAMES[rs1 as usize]), self.pc, REG_NAMES[rd as usize], return_addr); }
			},
			Op::OpImm => {
				let op_funct = opcode.funct3_op_imm();
				match op_funct {
					OpImmFunct3::AddI => {
						let imm = opcode.i_imm_signed();
						let rd = opcode.rd();
						let rs1 = opcode.rs1();
						let src_val = self.get_gpr(rs1) as i32;
						let dst_val = src_val.wrapping_add(imm);
						self.set_gpr(rd, dst_val as u32);
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("addi {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], imm), REG_NAMES[rd as usize], dst_val as u32); }
					},
					OpImmFunct3::SltI => {
						let imm = opcode.i_imm_signed();
						let rd = opcode.rd();
						let rs1 = opcode.rs1();
						let src_val = self.get_gpr(rs1) as i32;
						let dst_val = if src_val < imm { 1 } else { 0 };
						self.set_gpr(rd, dst_val as u32);
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= {}      .", format!("slti {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], imm), REG_NAMES[rd as usize], dst_val); }
					},
					OpImmFunct3::SltIU => {
						let imm = opcode.i_imm_signed() as u32;
						let rd = opcode.rd();
						let rs1 = opcode.rs1();
						let src_val = self.get_gpr(rs1);
						let dst_val = if src_val < imm { 1 } else { 0 };
						self.set_gpr(rd, dst_val);
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= {}      .", format!("sltiu {}, {}, {:#010x}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], imm), REG_NAMES[rd as usize], dst_val); }
					},
					OpImmFunct3::XorI => {
						let imm = opcode.i_imm_signed() as u32;
						let rd = opcode.rd();
						let rs1 = opcode.rs1();
						let src_val = self.get_gpr(rs1);
						let dst_val = src_val ^ imm;
						self.set_gpr(rd, dst_val);
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= {}      .", format!("xori {}, {}, {:#010x}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], imm), REG_NAMES[rd as usize], dst_val); }
					},
					OpImmFunct3::OrI => {
						let imm = opcode.i_imm_signed() as u32;
						let rd = opcode.rd();
						let rs1 = opcode.rs1();
						let src_val = self.get_gpr(rs1);
						let dst_val = src_val | imm;
						self.set_gpr(rd, dst_val);
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= {}      .", format!("ori {}, {}, {:#010x}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], imm), REG_NAMES[rd as usize], dst_val); }
					},
					OpImmFunct3::AndI => {
						let imm = opcode.i_imm_signed() as u32;
						let rd = opcode.rd();
						let rs1 = opcode.rs1();
						let src_val = self.get_gpr(rs1);
						let dst_val = src_val & imm;
						self.set_gpr(rd, dst_val);
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= {}      .", format!("andi {}, {}, {:#010x}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], imm), REG_NAMES[rd as usize], dst_val); }
					},
					OpImmFunct3::SllI => {
						let rd = opcode.rd();
						let shift = opcode.shamt();
						let rs1 = opcode.rs1();
						let src_val = self.get_gpr(rs1);
						let dst_val = src_val << shift;
						self.set_gpr(rd, dst_val);
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= {}      .", format!("slli {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], shift), REG_NAMES[rd as usize], dst_val); }
					},
					OpImmFunct3::SrxI => {
						let rd = opcode.rd();
						let shift = opcode.shamt();
						let rs1 = opcode.rs1();
						if opcode.srxi_is_arithmetic() {
							let src_val = self.get_gpr(rs1);
							let dst_val = src_val >> shift;
							self.set_gpr(rd, dst_val);
							if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= {}      .", format!("srli {}, {}, {:#03x}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], shift), REG_NAMES[rd as usize], dst_val); }
						} else {
							let src_val = self.get_gpr(rs1) as i32;
							let dst_val = src_val >> shift;
							self.set_gpr(rd, dst_val as u32);
							if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= {}      .", format!("srai {}, {}, {:#03x}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], shift), REG_NAMES[rd as usize], dst_val); }
						}
					},
				}
				self.pc += 4;
			},
			Op::Store => {
				let store_type = opcode.funct3_store();
				let src = opcode.rs2();
				let base = opcode.rs1();
				let offset = opcode.s_imm_signed();
				let address = self.get_gpr(base).wrapping_add(offset as u32);
				let value = self.get_gpr(src);
				match store_type {
					StoreFunct3::Byte => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # ({:#010x}) <= {:#04x}        .", format!("sb {}({}), {}", if offset != 0 { format!("{}", offset).to_string() } else { "".to_string() }, REG_NAMES[base as usize], REG_NAMES[src as usize]), address, value as u8); }
						match self.mio.write_8(address, value as u8) {
							MemWriteResult::Ok => {
							},
							MemWriteResult::ErrAlignment => {
								self.pending_exception = Some(Exception::StoreAddressMisaligned{
									instr_addr: self.pc,
									store_addr: address
								});
								return false;
							},
							_ => {
								self.pending_exception = Some(Exception::StoreAccessFault{
									instr_addr: self.pc,
									store_addr: address
								});
								return false;
							}
						}
					},
					StoreFunct3::Half => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # ({:#010x}) <= {:#06x}        .", format!("sh {}({}), {}", if offset != 0 { format!("{}", offset as i32).to_string() } else { "".to_string() }, REG_NAMES[base as usize], REG_NAMES[src as usize]), address, value as u16); }
						match self.mio.write_16(address, value as u16) {
							MemWriteResult::Ok => {
							},
							MemWriteResult::ErrAlignment => {
								self.pending_exception = Some(Exception::StoreAddressMisaligned{
									instr_addr: self.pc,
									store_addr: address
								});
								return false;
							},
							_ => {
								self.pending_exception = Some(Exception::StoreAccessFault {
									instr_addr: self.pc,
									store_addr: address
								});
								return false;
							}
						}
					},
					StoreFunct3::Word => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # ({:#010x}) <=  {:#010x}     .", format!("sw {}({}), {}", if offset != 0 { format!("{}", offset).to_string() } else { "".to_string() }, REG_NAMES[base as usize], REG_NAMES[src as usize]), address, value); }
						match self.mio.write_32(address, value) {
							MemWriteResult::Ok => {
							},
							MemWriteResult::ErrAlignment => {
								self.pending_exception = Some(Exception::StoreAddressMisaligned {
									instr_addr: self.pc,
									store_addr: address
								});
								return false;
							},
							_ => {
								self.pending_exception = Some(Exception::StoreAccessFault {
									instr_addr: self.pc,
									store_addr: address
								});
								return false;
							}
						}
					},
					StoreFunct3::Unknown => {
						return self.illegal_instruction(opcode);
					}
				}
				self.pc += 4;
			},
			Op::Load => {
				let load_type = opcode.funct3_load();
				let dest = opcode.rd();
				let base = opcode.rs1();
				let offset = opcode.i_imm_signed();
				let address = self.get_gpr(base).wrapping_add(offset as u32);
				match load_type {
					LoadFunct3::Byte => {
						let load_result = self.mio.read_8(address);
						match load_result {
							MemReadResult::Ok(value) => {
								self.set_gpr(dest, ((value as i8) as i32) as u32);
								if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("lb {}, {}({})", REG_NAMES[dest as usize], if offset != 0 { format!("{}", offset).to_string() } else { "".to_string() }, REG_NAMES[base as usize]), REG_NAMES[dest as usize], value as i8 as i32); }
							},
							MemReadResult::ErrAlignment => {
								self.pending_exception = Some(Exception::LoadAddressMisaligned{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							}
							_ => {
								self.pending_exception = Some(Exception::LoadAccessFault{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							}
						}
					},
					LoadFunct3::Half => {
						let load_result = self.mio.read_16(address);
						match load_result {
							MemReadResult::Ok(value) => {
								self.set_gpr(dest, ((value as i16) as i32) as u32);
								if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= ({:#010x})    .", format!("lh {}, {}({})", REG_NAMES[dest as usize], if offset != 0 { format!("{}", offset).to_string() } else { "".to_string() }, REG_NAMES[base as usize]), REG_NAMES[dest as usize], value as i16 as i32); }
							},
							MemReadResult::ErrAlignment => {
								self.pending_exception = Some(Exception::LoadAddressMisaligned{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							}
							_ => {
								self.pending_exception = Some(Exception::LoadAccessFault{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							}
						}
					},
					LoadFunct3::Word => {
						let load_result = self.mio.read_32(address);
						match load_result {
							MemReadResult::Ok(value) => {
								self.set_gpr(dest, value as u32);
								if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= ({:#010x})    .", format!("lw {}, {}({})", REG_NAMES[dest as usize], if offset != 0 { format!("{}", offset).to_string() } else { "".to_string() }, REG_NAMES[base as usize]), REG_NAMES[dest as usize], value as i32); }
							},
							MemReadResult::ErrAlignment => {
								self.pending_exception = Some(Exception::LoadAddressMisaligned{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							}
							_ => {
								self.pending_exception = Some(Exception::LoadAccessFault{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							}
						}
					},
					LoadFunct3::ByteUnsigned => {
						let load_result = self.mio.read_8(address);
						match load_result {
							MemReadResult::Ok(value) => {
								self.set_gpr(dest, value as u32);
								if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= ({:#010x})    .", format!("lbu {}, {}({})", REG_NAMES[dest as usize], if offset != 0 { format!("{}", offset).to_string() } else { "".to_string() }, REG_NAMES[base as usize]), REG_NAMES[dest as usize], value); }
							},
							MemReadResult::ErrAlignment => {
								self.pending_exception = Some(Exception::LoadAddressMisaligned{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							}
							_ => {
								self.pending_exception = Some(Exception::LoadAccessFault{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							}
						}
					},
					LoadFunct3::HalfUnsigned => {
						let load_result = self.mio.read_16(address);
						match load_result {
							MemReadResult::Ok(value) => {
								self.set_gpr(dest, value as u32);
								if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <= ({:#010x})    .", format!("lhu {}, {}({})", REG_NAMES[dest as usize], if offset != 0 { format!("{}", offset).to_string() } else { "".to_string() }, REG_NAMES[base as usize]), REG_NAMES[dest as usize], value); }
							},
							MemReadResult::ErrAlignment => {
								self.pending_exception = Some(Exception::LoadAddressMisaligned{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							}
							_ => {
								self.pending_exception = Some(Exception::LoadAccessFault{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							}
						}
					},
					LoadFunct3::Unknown => {
						return self.illegal_instruction(opcode);
					},
				}
				self.pc += 4;
			},
			Op::Op => {
				let rd: u32 = opcode.rd();
				let rs1: u32 = opcode.rs1();
				let rs2: u32 = opcode.rs2();
				let s1_value: u32 = self.get_gpr(rs1);
				let s2_value: u32 = self.get_gpr(rs2);
				let op_funct = opcode.funct3funct7_op();
				let value = match op_funct {
					OpFunct3Funct7::Add => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("add {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], s1_value.wrapping_add(s2_value)); }
						s1_value.wrapping_add(s2_value)
					},
					OpFunct3Funct7::Sub => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("sub {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], s1_value.wrapping_sub(s2_value)); }
						s1_value.wrapping_sub(s2_value)
					}
					OpFunct3Funct7::Sll => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("sll {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], s1_value << (s2_value & 0x1F)); }
						s1_value << (s2_value & 0x1F)
					},
					OpFunct3Funct7::Slt => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("slt {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], if (s1_value as i32) < (s2_value as i32) { 1 } else { 0 }); }
						if (s1_value as i32) < (s2_value as i32) { 1 } else { 0 }
					},
					OpFunct3Funct7::SltU => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("sltu {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], if s1_value < s2_value { 1 } else { 0 }); }
						if s1_value < s2_value { 1 } else { 0 }
					},
					OpFunct3Funct7::Xor => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("xor {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], s1_value ^ s2_value); }
						s1_value ^ s2_value
					},
					OpFunct3Funct7::Sra => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("sra {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], (s1_value as i32 >> (s2_value & 0x1F)) as u32); }
						(s1_value as i32 >> (s2_value & 0x1F)) as u32
					},
					OpFunct3Funct7::Srl => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("srl {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], s1_value >> (s2_value & 0x1F)); }
						s1_value >> (s2_value & 0x1F)
					},
					OpFunct3Funct7::Or => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("or {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], s1_value | s2_value); }
						s1_value | s2_value
					},
					OpFunct3Funct7::And => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("and {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], s1_value & s2_value); }
						s1_value & s2_value
					},
					OpFunct3Funct7::Mul => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("mul {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], s1_value * s2_value); }
						s1_value * s2_value
					},
					OpFunct3Funct7::MulH => {
						let mult = ((s1_value as i32) as i64) * ((s2_value as i32) as i64);
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("mulh {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], (mult >> 32) as u32); }
						(mult >> 32) as u32
					},
					OpFunct3Funct7::MulHSU => {
						let mult = (s1_value as u64) as i64 * (s2_value as i64);
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("mulhsu {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], (mult >> 32) as u32); }
						(mult >> 32) as u32
					},
					OpFunct3Funct7::MulHU => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("mulhu {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], (((s1_value as u64) * (s2_value as u64)) >> 32) as u32); }
						(((s1_value as u64) * (s2_value as u64)) >> 32) as u32
					},
					OpFunct3Funct7::Div => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("div {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], ((s1_value as i32) / (s2_value as i32)) as u32); }
						((s1_value as i32) / (s2_value as i32)) as u32
					},
					OpFunct3Funct7::DivU => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("divu {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], s1_value / s2_value); }
						s1_value / s2_value
					},
					OpFunct3Funct7::Rem => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("rem {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], ((s1_value as i32) % (s2_value as i32)) as u32); }
						((s1_value as i32) % (s2_value as i32)) as u32
					},
					OpFunct3Funct7::RemU => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # {: <12} <=  {:#010x}     .", format!("remu {}, {}, {}", REG_NAMES[rd as usize], REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize]), REG_NAMES[rd as usize], s1_value % s2_value); }
						s1_value % s2_value
					},
					OpFunct3Funct7::Unknown => {
						return self.illegal_instruction(opcode);
					},
				};
				self.set_gpr(rd, value);
				self.pc += 4;
			},
			Op::Branch => {
				let branch_type = opcode.funct3_branch();
				let rs1 = opcode.rs1();
				let rs2 = opcode.rs2();
				let s1_value = self.get_gpr(rs1);
				let s2_value = self.get_gpr(rs2);
				let offset = opcode.b_imm_signed();
				let branch_addr = self.pc.wrapping_add(offset as u32);
				if match branch_type {
					BranchFunct3::Eq => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # pc           <=  {:#010x}     .", format!("beq {}, {}, {:#010x}", REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize], branch_addr), if s1_value == s2_value {branch_addr} else {self.pc + 4}); }
						s1_value == s2_value
					},
					BranchFunct3::NEq => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # pc           <=  {:#010x}     .", format!("bne {}, {}, {:#010x}", REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize], branch_addr), if s1_value != s2_value {branch_addr} else {self.pc + 4}); }
						s1_value != s2_value
					},
					BranchFunct3::Lt => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # pc           <=  {:#010x}     .", format!("blt {}, {}, {:#010x}", REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize], branch_addr), if (s1_value as i32) < (s2_value as i32) {branch_addr} else {self.pc + 4}); }
						(s1_value as i32) < (s2_value as i32)
					},
					BranchFunct3::GEq => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # pc           <=  {:#010x}     .", format!("bge {}, {}, {:#010x}", REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize], branch_addr), if (s1_value as i32) >= (s2_value as i32) {branch_addr} else {self.pc + 4}); }
						(s1_value as i32) >= (s2_value as i32)
					},
					BranchFunct3::LtU => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # pc           <=  {:#010x}     .", format!("bltu {}, {}, {:#010x}", REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize], branch_addr), if s1_value < s2_value {branch_addr} else {self.pc + 4}); }
						s1_value < s2_value
					},
					BranchFunct3::GEqU => {
						if cfg!(feature = "cpu_debug") { println!("{: <50} # pc           <=  {:#010x}     .", format!("bgeu {}, {}, {:#010x}", REG_NAMES[rs1 as usize], REG_NAMES[rs2 as usize], branch_addr), if s1_value >= s2_value {branch_addr} else {self.pc + 4}); }
						s1_value >= s2_value
					},
					BranchFunct3::Unknown => {
						return self.illegal_instruction(opcode);
					},
				} {
					self.pc = self.pc.wrapping_add(offset as u32);
				} else {
					self.pc += 4;
				}
			},
			Op::LoadFp => {
				let rd = opcode.rd();
				let rbase = opcode.rs1();
				let offset = opcode.i_imm_signed();
				let address = self.get_gpr(rbase).wrapping_add(offset as u32);
				let width = opcode.funct3_loadfp();
				match width {
					LoadFpFunct3::Width32 => {
						let value = match self.mio.read_32(address) {
							MemReadResult::Ok(value) => {
								f32::from_bits(value)
							},
							MemReadResult::ErrAlignment => {
								self.pending_exception = Some(Exception::LoadAddressMisaligned{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							},
							_ => {
								self.pending_exception = Some(Exception::LoadAccessFault{
									instr_addr: self.pc,
									load_addr: address
								});
								return false;
							}
						};
						self.set_fpr(rd, value);
					},
					LoadFpFunct3::Unknown => {
						return self.illegal_instruction(opcode);
					},
				}
				self.pc += 4;
			},
			Op::StoreFp => {
				let rs = opcode.rs2();
				let rbase = opcode.rs1();
				let offset = opcode.s_imm_signed();
				let address = self.get_gpr(rbase).wrapping_add(offset as u32);
				let width = opcode.funct3_storefp();
				match width {
					StoreFpFunct3::Width32 => {
						let value = self.get_fpr(rs);
						let value_raw = f32::to_bits(value);
						match self.mio.write_32(address, value_raw) {
							MemWriteResult::Ok => {},
							MemWriteResult::ErrAlignment => {
								self.pending_exception = Some(Exception::StoreAddressMisaligned{
									instr_addr: self.pc,
									store_addr: address
								});
								return false;
							},
							_ => {
								self.pending_exception = Some(Exception::StoreAccessFault{
									instr_addr: self.pc,
									store_addr: address
								});
								return false;
							}
						}
					},
					StoreFpFunct3::Unknown => {
						return self.illegal_instruction(opcode);
					},
				}
				self.pc += 4;
			},
			Op::System => {
				let funct = opcode.funct3_system();
				let rd: u32 = opcode.rd();
				let rs1: u32 = opcode.rs1();
				let csr = opcode.i_imm();
				match funct {
					SystemFunct3::Int => {
						let ifunct = opcode.funct7_system_int();
						match ifunct {
							SystemIntFunct7::WaitForInterrupt => {
								if cfg!(feature = "cpu_debug") { println!("{: <50} # .                               .", format!("wfi")); }
								self.pc += 4;
								self.waiting_for_interrupt = true;
								return false;
							},
							SystemIntFunct7::MRet => {
								self.pc = self.trap_csrs.mepc;
								self.trap_csrs.mstatus &= !MSTATUS_MIE;
								self.trap_csrs.mstatus |= if self.trap_csrs.mstatus & MSTATUS_MPIE != 0 {
									MSTATUS_MIE
								} else {
									0
								};
								if cfg!(feature = "cpu_debug") { println!("{: <50} # pc           <=  {:#010x}     mstatus      <=  {:032b}", format!("mret"), self.pc, self.trap_csrs.mstatus); }
							},
							SystemIntFunct7::Unknown => {
								return self.illegal_instruction(opcode);
							},
						}
					},
					SystemFunct3::CsrRW => {
						if cfg!(feature = "cpu_debug") { println!("csrrw"); }
						let csr_value = if rd != 0 {
							let csr_value = self.read_csr(csr);
							self.set_gpr(rd, csr_value);
							csr_value
						} else {0};
						let rs1_value = self.get_gpr(rs1);
						self.pc += 4;
						if cfg!(feature = "cpu_debug") { 
							if rd == 0 {
								println!("{: <50} # CSR {:#05x}   <=  {:#010x} .", format!("csrrw {}, {:#05x}, {}", REG_NAMES[rd as usize], csr, REG_NAMES[rs1 as usize]), csr, rs1_value);
							} else {
								println!("{: <50} # {}           <=  {:#010x} . CSR {:#05x}   <=  {:010x} ", format!("csrrw {}, {:#05x}, {}", REG_NAMES[rd as usize], csr, REG_NAMES[rs1 as usize]), REG_NAMES[rd as usize], csr_value, csr, rs1_value);
							}
						}
						return self.write_csr(csr, rs1_value);
					},
					SystemFunct3::CsrRS => {
						if cfg!(feature = "cpu_debug") { println!("csrrs"); }
						let csr_value_old = self.read_csr(csr);
						self.set_gpr(rd, csr_value_old);
						self.pc += 4;
						if rs1 != 0 {
							let rs1_value = self.get_gpr(rs1);
							let updated_value = csr_value_old | rs1_value;
							return self.write_csr(csr, updated_value);
						}
					},
					SystemFunct3::CsrRC => {
						if cfg!(feature = "cpu_debug") { println!("csrrc"); }
						let csr_value_old = self.read_csr(csr);
						self.set_gpr(rd, csr_value_old);
						self.pc += 4;
						if rs1 != 0 {
							let rs1_value = self.get_gpr(rs1);
							let updated_value = csr_value_old & !rs1_value;
							return self.write_csr(csr, updated_value);
						}
					},
					SystemFunct3::CsrRWI => {
						let csr_value = if rd != 0 {
							let csr_value_old = self.read_csr(csr);
							self.set_gpr(rd, csr_value_old);
							csr_value_old
						} else { 0 };
						self.pc += 4;
						let csr_value_new = if (rs1 & 0x10) != 0 {
							rs1 & 0xFFFFFFF0
						} else {
							rs1
						};
						if cfg!(feature = "cpu_debug") { 
							if rd == 0 {
								println!("{: <50} # CSR {:#05x}    <=  {:010x}     .", format!("csrrwi {}, {:#05x}, {:#010x}", REG_NAMES[rd as usize], csr, csr_value_new), csr, csr_value_new);
							} else {
								println!("{: <50} # {}            <=  {:010x}     CSR {:#05x}    <=  {:010x}     ", format!("csrrwi {}, {:#05x}, {}", REG_NAMES[rd as usize], csr, csr_value_new), REG_NAMES[rd as usize], csr_value, csr, csr_value_new);
							}
						}
						return self.write_csr(csr, csr_value_new);
					},
					SystemFunct3::CsrRSI => {
						if cfg!(feature = "cpu_debug") { println!("csrrsi"); }
						let csr_value_old = self.read_csr(csr);
						let csr_bits_new = if (rs1 & 0x10) != 0 {
							rs1 & 0xFFFFFFF0
						} else {
							rs1
						};
						self.pc += 4;
						let csr_value_new = csr_value_old | csr_bits_new;
						return self.write_csr(csr, csr_value_new);
					},
					SystemFunct3::CsrRCI => {
						if cfg!(feature = "cpu_debug") { println!("csrrci"); }
						let csr_value_old = self.read_csr(csr);
						let csr_bits_new = if (rs1 & 0x10) != 0 {
							rs1 & 0xFFFFFFF0
						} else {
							rs1
						};
						self.pc += 4;
						let csr_value_new = csr_value_old & !csr_bits_new;
						return self.write_csr(csr, csr_value_new);
					},
					SystemFunct3::Unknown => {
						return self.illegal_instruction(opcode);
					},
				}
			},
			Op::MAdd => {
				let rd = opcode.rd();
				let rm = opcode.fp_rm();
				let rs1 = opcode.rs1();
				let rs2 = opcode.rs2();
				let rs3 = opcode.rs3();
				// todo
			},
			Op::MSub => {
				let rd = opcode.rd();
				let rm = opcode.fp_rm();
				let rs1 = opcode.rs1();
				let rs2 = opcode.rs2();
				let rs3 = opcode.rs3();
				// todo
			},
			Op::NMAdd => {
				let rd = opcode.rd();
				let rm = opcode.fp_rm();
				let rs1 = opcode.rs1();
				let rs2 = opcode.rs2();
				let rs3 = opcode.rs3();
				// todo
			},
			Op::NMSub => {
				let rd = opcode.rd();
				let rm = opcode.fp_rm();
				let rs1 = opcode.rs1();
				let rs2 = opcode.rs2();
				let rs3 = opcode.rs3();
				// todo
			},
			Op::OpFp => {
				let rd = opcode.rd();
				let rs1 = opcode.rs1();
				let rs2 = opcode.rs2();
				let rm = opcode.fp_rm(); // rounding mode ignored - https://github.com/rust-lang/rust/issues/72252
				let funct7 = opcode.funct7_fp();
				match funct7 {
					FpFunct7::Add_S => {
						let a = self.get_fpr(rs1);
						let b = self.get_fpr(rs2);
						let result = a + b;
						self.set_fpr(rd, result);
					},
					FpFunct7::Sub_S => {
						let a = self.get_fpr(rs1);
						let b = self.get_fpr(rs2);
						let result = a - b;
						self.set_fpr(rd, result);
					},
					FpFunct7::Mul_S => {
						let a = self.get_fpr(rs1);
						let b = self.get_fpr(rs2);
						let result = a * b;
						self.set_fpr(rd, result);
					},
					FpFunct7::Div_S => {
						let a = self.get_fpr(rs1);
						let b = self.get_fpr(rs2);
						let result = a / b;
						self.set_fpr(rd, result);
					},
					FpFunct7::Sqrt_S => {
						// todo
					},
					FpFunct7::Sign_S => {
						// todo
					},
					FpFunct7::MinMax_S => {
						// todo
					},
					FpFunct7::CvtW_S => {
						// todo
					},
					FpFunct7::MvXWClass_S => {
						// todo
					},
					FpFunct7::Cmp_S => {
						// todo
					},
					FpFunct7::CvtS_W => {
						// todo
					},
					FpFunct7::MvWX_S => {
						// todo
					},
					FpFunct7::Unknown => {
						return self.illegal_instruction(opcode);
					}
				}
			}
			_ => {
				return self.illegal_instruction(opcode);
			}
		}
		true
	}
	
	fn illegal_instruction(&mut self, opcode: Opcode) -> bool {
		if cfg!(feature = "cpu_debug") { println!("unknown opcode: {:#08}", opcode.value); }
		self.pending_exception = Some(Exception::IllegalInstruction{
			op: opcode.value,
			addr: self.pc
		});
		return false
	}
	
	pub fn step_break(&mut self) {
		self.mio.access_break();
	}

	fn set_gpr(&mut self, reg: u32, val: u32) {
		if reg != 0 {
			self.xr[(reg - 1) as usize] = val;
		}
	}

	fn get_gpr(&self, reg: u32) -> u32 {
		if reg == 0 {
			0
		} else {
			self.xr[(reg - 1) as usize]
		}
	}

	fn trace_gpr(&self, gpr: u32) -> String {
		format!("{}: {:#010x}", REG_NAMES_PAD[gpr as usize], self.get_gpr(gpr)).to_string()
	}
	
	pub fn trace_regs(&self) -> String {
		let mut s: String = String::new();
		for i in 0 .. 32 {
			s += self.trace_gpr(i).as_str();
			s += if (i & 1) == 1 {
				"\n"
			} else {
				"  "
			}
		}
		s += format!("\npc:  {:#010x}\n", self.pc).as_str();
		s
	}
	
	fn set_fpr(&mut self, reg: u32, value: f32) {
		self.fr[reg as usize] = value;
	}
	
	fn get_fpr(&self, reg: u32) -> f32 {
		self.fr[reg as usize]
	}
	
	fn get_time(&self) -> u64 {
		let t = Instant::now() - self.csr_time_start;
		t.as_millis() as u64
	}
	
	fn read_csr(&mut self, csr: u32) -> u32 {
		match csr {
			// FPU
			// FFlags: floating point accrued exceptions
			0x001 => {
				self.fcsr & 0x1F
			},
			// FRM: floating point rounding mode
			0x002 => {
				(self.fcsr >> 5) & 0x07
			},
			// fcsr: floating point control/status
			0x003 => {
				self.fcsr
			},
			
			// mstatus: machine status
			0x300 => {
				self.trap_csrs.mstatus
			},
			// misa: ISA and extensions
			0x301 => {
				(1 << 30) | // RV32 ISA
				(1 << ('i' as u32 - 'a' as u32)) | // "I" extension support
				(1 << ('f' as u32 - 'a' as u32)) | // "F" extension support
				(1 << ('m' as u32 - 'a' as u32)) | // "M" extension support
				(1 << ('x' as u32 - 'a' as u32))   // "X" not standard extensions implemented
			},
			// mie: machine interrupt enable
			0x304 => {
				self.trap_csrs.mie
			},
			// mtvec: machine trap handler base address
			0x305 => {
				self.trap_csrs.mtvec
			},
			
			// mscratch: machine scratch register
			0x340 => {
				self.trap_csrs.mscratch
			},
			// mepc: machine exception program counter
			0x341 => {
				self.trap_csrs.mepc
			},
			// mcause: machine trap cause
			0x342 => {
				self.trap_csrs.mepc
			},
			// mtval: machine bad instruction or address
			0x343 => {
				self.trap_csrs.mtval
			},
			// mip: machine interrupts pending
			0x344 => {
				self.trap_csrs.mip
			},
			
			// RV32I Counters
			// Cycle: Cycle counter
			0xC00 => {
				self.csr_cycle as u32
			},
			0xC80 => {
				(self.csr_cycle >> 32) as u32
			},
			// Time: general timer
			0xC01 => {
				self.get_time() as u32
			},
			0xC81 => {
				(self.get_time() >> 32) as u32
			},
			// InstrRet: retired instrution counter
			0xC02 => {
				self.csr_instrret as u32
			},
			0xC82 => {
				(self.csr_instrret >> 32) as u32
			},
			
			// Machine Vendor ID
			0xF11 => { 0 },
			// Machine Architecture ID
			0xF12 => { 0 },
			// Machine Implementation ID
			0xF13 => { 0 },
			// Hart ID
			0xF14 => { self.hart_id },
			
			
			_ => 0
		}
	}
	
	fn write_csr(&mut self, csr: u32, value: u32) -> bool {
		match csr {
			0xBFF => {
				return false;
			},
			0x300 => {
				let mut fixed_value = (value & (MSTATUS_MIE | MSTATUS_MPIE | MSTATUS_FS_MASK)) | MSTATUS_MPP;
				if (fixed_value & MSTATUS_FS_MASK) == MSTATUS_FS_DIRTY {
					fixed_value |= MSTATUS_SD;
				}
				self.trap_csrs.mstatus = fixed_value;
			},
			0x304 => {
				let fixed_value = value & (MIE_MSIE | MIE_MTIE | MIE_MEIE);
				self.trap_csrs.mie = fixed_value;
			},
			0x305 => {
				self.trap_csrs.mtvec = value;
			},
			0x340 => {
				self.trap_csrs.mscratch = value;
			},
			0x344 => {
				let fixed_value = value & (MIP_MSIP | MIP_MTIP | MIP_MEIP);
				self.trap_csrs.mip = fixed_value;
			},
			0x001 => {
				self.fcsr = (self.fcsr & 0xE0) | (value & 0x1F);
			},
			0x002 => {
				self.fcsr = (self.fcsr & 0x1F) | (value & 0x07) << 5;
			},
			0x003 => {
				self.fcsr = value & 0xFF;
			}
			_ => {
			}
		}
		true
	}
}