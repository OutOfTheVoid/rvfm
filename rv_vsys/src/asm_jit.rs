use crate::{BranchFunct3, LoadFunct3, Op, OpFunct3Funct7, OpImmFunct3, StoreFunct3, SystemFunct3};
use crate::mem::{MemIO, MemWriteResult};
use num_traits::ToPrimitive;

#[allow(non_camel_case_types)]
pub enum OpcodeType {
	RV32I_R,
	RV32I_I,
	RV32I_S,
	RV32I_B,
	RV32I_U,
	RV32I_J,
}

pub struct AsmResult {
	pub instruction_address: u32,
	pub next_instruction_address: u32,
	pub opcode_type: OpcodeType,
	pub immediate: Option<i32>,
	pub op: Op,
	pub funct3: Option<u32>,
	pub rs1: Option<u32>,
	pub rs2: Option<u32>,
	pub rd: Option<u32>,
	pub funct7: Option<u32>,
}

impl AsmResult {
	
	pub fn new_r_type(op: Op, rd: u32, funct3: u32, rs1: u32, rs2: u32, funct7: u32, address: u32) -> Self {
		AsmResult {
			instruction_address: address,
			next_instruction_address: address + 4,
			opcode_type: OpcodeType::RV32I_R,
			immediate: None,
			op: op,
			funct3: Some(funct3),
			rs1: Some(rs1),
			rs2: Some(rs2),
			rd: Some(rd),
			funct7: Some(funct7),
		}
	}
	
	pub fn new_i_type(op: Op, rd: u32, funct3: u32, rs1: u32, imm: i32, address: u32) -> Self {
		AsmResult {
			instruction_address: address,
			next_instruction_address: address + 4,
			opcode_type: OpcodeType::RV32I_I,
			immediate: Some(imm),
			op: op,
			funct3: Some(funct3),
			rs1: Some(rs1),
			rs2: None,
			rd: Some(rd),
			funct7: None,
		}
	}
	
	pub fn new_s_type(op: Op, funct3: u32, rs1: u32, rs2: u32, imm: i32, address: u32) -> Self {
		AsmResult {
			instruction_address: address,
			next_instruction_address: address + 4,
			opcode_type: OpcodeType::RV32I_S,
			immediate: Some(imm),
			op: op,
			funct3: Some(funct3),
			rs1: Some(rs1),
			rs2: Some(rs2),
			rd: None,
			funct7: None,
		}
	}
	
	pub fn new_b_type(op: Op, funct3: u32, rs1: u32, rs2: u32, imm: i32, address: u32) -> Self {
		AsmResult {
			instruction_address: address,
			next_instruction_address: address + 4,
			opcode_type: OpcodeType::RV32I_B,
			immediate: Some(imm),
			op: op,
			funct3: Some(funct3),
			rs1: Some(rs1),
			rs2: Some(rs2),
			rd: None,
			funct7: None,
		}
	}
	
	pub fn new_u_type(op: Op, rd: u32, imm: i32, address: u32) -> Self {
		AsmResult {
			instruction_address: address,
			next_instruction_address: address + 4,
			opcode_type: OpcodeType::RV32I_U,
			immediate: Some(imm),
			op: op,
			funct3: None,
			rs1: None,
			rs2: None,
			rd: Some(rd),
			funct7: None,
		}
	}
	
	pub fn new_j_type(op: Op, rd: u32, imm: i32, address: u32) -> Self {
		AsmResult {
			instruction_address: address,
			next_instruction_address: address + 4,
			opcode_type: OpcodeType::RV32I_J,
			immediate: Some(imm),
			op: op,
			funct3: None,
			rs1: None,
			rs2: None,
			rd: Some(rd),
			funct7: None,
		}
	}
}

pub struct AsmJit {
	start_address: u32,
	bytes: Vec<u8>
}

const fn bitfield(x: u32, count: u32, src_bit: u32, dst_bit: u32) -> u32 {
	let y = x >> src_bit;
	let mask = (1_i64 << count).wrapping_sub(1);
	let z = y & (mask as u32);
	z << dst_bit
}

impl AsmJit {
	pub fn new(start_address: u32) -> Self {
		AsmJit {
			start_address: start_address,
			bytes: Vec::new()
		}
	}

	pub fn write_to_mem<MIO: MemIO>(&self, mio: &mut MIO) -> MemWriteResult {
		for i in 0 .. self.bytes.len() {
			let write_result = mio.write_8(self.start_address + i as u32, self.bytes[i]);
			match write_result {
				MemWriteResult::Ok => {
				},
				_ => {
					return write_result;
				}
			}
		}
		return MemWriteResult::Ok;
	}
	
	pub fn nop(&mut self) -> AsmResult {
		self.i_type(Op::OpImm, 0, OpImmFunct3::XorI.to_u32().unwrap(), 0, 0)
	}

	pub fn lui(&mut self, rd: u32, immediate: i32) -> AsmResult {
		self.u_type(Op::Lui, rd, immediate)
	}

	pub fn auipc(&mut self, rd: u32, immediate: i32) -> AsmResult {
		self.u_type(Op::Auipc, rd, immediate)
	}

	pub fn jal(&mut self, rd: u32, offset: i32) -> AsmResult {
		self.j_type(Op::Jal, rd, offset)
	}
	
	pub fn jalr(&mut self, rd: u32, rs: u32, immediate: i32) -> AsmResult {
		self.i_type(Op::Jalr, rd, 0, rs, immediate)
	}
	
	pub fn add(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::Add.imm3(), rs1, rs2, OpFunct3Funct7::Add.imm7())
	}
	
	pub fn sub(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::Sub.imm3(), rs1, rs2, OpFunct3Funct7::Sub.imm7())
	}
	
	pub fn sll(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::Sll.imm3(), rs1, rs2, OpFunct3Funct7::Sll.imm7())
	}
	
	pub fn slt(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::Slt.imm3(), rs1, rs2, OpFunct3Funct7::Slt.imm7())
	}
	
	pub fn sltu(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::SltU.imm3(), rs1, rs2, OpFunct3Funct7::SltU.imm7())
	}
	
	pub fn xor(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::Xor.imm3(), rs1, rs2, OpFunct3Funct7::Xor.imm7())
	}
	
	pub fn srl(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::Srl.imm3(), rs1, rs2, OpFunct3Funct7::Srl.imm7())
	}
	
	pub fn sra(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::Sra.imm3(), rs1, rs2, OpFunct3Funct7::Sra.imm7())
	}
	
	pub fn or(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::Or.imm3(), rs1, rs2, OpFunct3Funct7::Or.imm7())
	}
	
	pub fn and(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::And.imm3(), rs1, rs2, OpFunct3Funct7::And.imm7())
	}
	
	pub fn addi(&mut self, rd: u32, rs: u32, immediate: i32) -> AsmResult {
		self.i_type(Op::OpImm, rd, OpImmFunct3::AddI.to_u32().unwrap(), rs, immediate)
	}
	
	pub fn slti(&mut self, rd: u32, rs: u32, immediate: i32) -> AsmResult {
		self.i_type(Op::OpImm, rd, OpImmFunct3::SltI.to_u32().unwrap(), rs, immediate)
	}
	
	pub fn sltiu(&mut self, rd: u32, rs: u32, immediate: i32) -> AsmResult {
		self.i_type(Op::OpImm, rd, OpImmFunct3::SltIU.to_u32().unwrap(), rs, immediate)
	}
	
	pub fn andi(&mut self, rd: u32, rs: u32, immediate: i32) -> AsmResult {
		self.i_type(Op::OpImm, rd, OpImmFunct3::AndI.to_u32().unwrap(), rs, immediate)
	}
	
	pub fn xori(&mut self, rd: u32, rs: u32, immediate: i32) -> AsmResult {
		self.i_type(Op::OpImm, rd, OpImmFunct3::XorI.to_u32().unwrap(), rs, immediate)
	}
	
	pub fn ori(&mut self, rd: u32, rs: u32, immediate: i32) -> AsmResult {
		self.i_type(Op::OpImm, rd, OpImmFunct3::OrI.to_u32().unwrap(), rs, immediate)
	}
	
	pub fn slli(&mut self, rd: u32, rs: u32, shift: u32) -> AsmResult {
		self.i_type(Op::OpImm, rd, OpImmFunct3::SllI.to_u32().unwrap(), rs, (shift & 0x1F) as i32)
	}
	
	pub fn srli(&mut self, rd: u32, rs: u32, shift: u32) -> AsmResult {
		self.i_type(Op::OpImm, rd, OpImmFunct3::SrxI.to_u32().unwrap(), rs, (shift & 0x1F) as i32)
	}
	
	pub fn srai(&mut self, rd: u32, rs: u32, shift: u32) -> AsmResult {
		self.i_type(Op::OpImm, rd, OpImmFunct3::SrxI.to_u32().unwrap(), rs, ((shift & 0x1F) | 0x4000_0000) as i32)
	}
	
	pub fn sb(&mut self, rs: u32, rbase: u32, offset: i32) -> AsmResult {
		self.s_type(Op::Store, StoreFunct3::Byte.to_u32().unwrap(), rbase, rs, offset)
	}
	
	pub fn sh(&mut self, rs: u32, rbase: u32, offset: i32) -> AsmResult {
		self.s_type(Op::Store, StoreFunct3::Half.to_u32().unwrap(), rbase, rs, offset)
	}
	
	pub fn sw(&mut self, rs: u32, rbase: u32, offset: i32) -> AsmResult {
		self.s_type(Op::Store, StoreFunct3::Word.to_u32().unwrap(), rbase, rs, offset)
	}
	
	pub fn lb(&mut self, rd: u32, rbase: u32, offset: i32) -> AsmResult {
		self.i_type(Op::Load, rd, LoadFunct3::Byte.to_u32().unwrap(), rbase, offset)
	}
	
	pub fn lh(&mut self, rd: u32, rbase: u32, offset: i32) -> AsmResult {
		self.i_type(Op::Load, rd, LoadFunct3::Half.to_u32().unwrap(), rbase, offset)
	}
	
	pub fn lw(&mut self, rd: u32, rbase: u32, offset: i32) -> AsmResult {
		self.i_type(Op::Load, rd, LoadFunct3::Word.to_u32().unwrap(), rbase, offset)
	}
	
	pub fn lbu(&mut self, rd: u32, rbase: u32, offset: i32) -> AsmResult {
		self.i_type(Op::Load, rd, LoadFunct3::ByteUnsigned.to_u32().unwrap(), rbase, offset)
	}
	
	pub fn lhu(&mut self, rd: u32, rbase: u32, offset: i32) -> AsmResult {
		self.i_type(Op::Load, rd, LoadFunct3::HalfUnsigned.to_u32().unwrap(), rbase, offset)
	}
	
	pub fn beq(&mut self, rs1: u32, rs2: u32, offset: i32) -> AsmResult {
		self.b_type(Op::Branch, offset, BranchFunct3::Eq.to_u32().unwrap(), rs1, rs2)
	}
	
	pub fn bne(&mut self, rs1: u32, rs2: u32, offset: i32) -> AsmResult {
		self.b_type(Op::Branch, offset, BranchFunct3::NEq.to_u32().unwrap(), rs1, rs2)
	}
	
	pub fn blt(&mut self, rs1: u32, rs2: u32, offset: i32) -> AsmResult {
		self.b_type(Op::Branch, offset, BranchFunct3::Lt.to_u32().unwrap(), rs1, rs2)
	}
	
	pub fn bge(&mut self, rs1: u32, rs2: u32, offset: i32) -> AsmResult {
		self.b_type(Op::Branch, offset, BranchFunct3::GEq.to_u32().unwrap(), rs1, rs2)
	}
	
	pub fn bltu(&mut self, rs1: u32, rs2: u32, offset: i32) -> AsmResult {
		self.b_type(Op::Branch, offset, BranchFunct3::LtU.to_u32().unwrap(), rs1, rs2)
	}
	
	pub fn bgeu(&mut self, rs1: u32, rs2: u32, offset: i32) -> AsmResult {
		self.b_type(Op::Branch, offset, BranchFunct3::GEqU.to_u32().unwrap(), rs1, rs2)
	}
	
	pub fn mul(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::Mul.imm3(), rs1, rs2, OpFunct3Funct7::Mul.imm7())
	}
	
	pub fn mulh(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::MulH.imm3(), rs1, rs2, OpFunct3Funct7::MulH.imm7())
	}
	
	pub fn mulhu(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::MulHU.imm3(), rs1, rs2, OpFunct3Funct7::MulHU.imm7())
	}
	
	pub fn mulhsu(&mut self, rd: u32, rs1: u32, rs2: u32) -> AsmResult {
		self.r_type(Op::Op, rd, OpFunct3Funct7::MulHSU.imm3(), rs1, rs2, OpFunct3Funct7::MulHSU.imm7())
	}
	
	pub fn csrrw(&mut self, rd: u32, csr: u32, rs1: u32) -> AsmResult {
		self.i_type(Op::System, rd, SystemFunct3::CsrRW.to_u32().unwrap(), rs1, csr as i32)
	}
	
	pub fn rewrite_immediate(&mut self, asm_result: &AsmResult, immediate: i32) {
		match asm_result.opcode_type {
			OpcodeType::RV32I_I => {
				self.i_type_at(asm_result.op, asm_result.rd.unwrap(), asm_result.funct3.unwrap(), asm_result.rs1.unwrap(), immediate, asm_result.instruction_address);
			}
			OpcodeType::RV32I_S => {
				self.s_type_at(asm_result.op, asm_result.funct3.unwrap(), asm_result.rs1.unwrap(), asm_result.rs2.unwrap(), immediate, asm_result.instruction_address);
			}
			OpcodeType::RV32I_B => {
				self.b_type_at(asm_result.op, immediate, asm_result.funct3.unwrap(), asm_result.rs1.unwrap(), asm_result.rs2.unwrap(), asm_result.instruction_address);
			}
			OpcodeType::RV32I_U => {
				self.u_type_at(asm_result.op, asm_result.rd.unwrap(), immediate, asm_result.instruction_address);
			},
			OpcodeType::RV32I_J => {
				self.j_type_at(asm_result.op, asm_result.rd.unwrap(), immediate, asm_result.instruction_address);
			},
			_ => {
			}
		}
	}

	fn r_type(&mut self, op: Op, rd: u32, funct3: u32, rs1: u32, rs2: u32, funct7: u32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(rd, 5, 0, 7) | 
			bitfield(funct3, 3, 0, 12) | 
			bitfield(rs1, 5, 0, 15) | 
			bitfield(rs2, 5, 0, 20) |
			bitfield(funct7, 7, 0, 25);
		let address = self.bytes.len() as u32 + self.start_address;
		self.push_u32(opcode_value);
		AsmResult::new_r_type(op, rd, funct3, rs1, rs2, funct7, address)
	}
	
	#[allow(unused)]
	fn r_type_at(&mut self, op: Op, rd: u32, funct3: u32, rs1: u32, rs2: u32, funct7: u32, address: u32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(rd, 5, 0, 7) | 
			bitfield(funct3, 3, 0, 12) | 
			bitfield(rs1, 5, 0, 15) | 
			bitfield(rs2, 5, 0, 20) |
			bitfield(funct7, 7, 0, 25);
		self.write_u32(opcode_value, address);
		AsmResult::new_r_type(op, rd, funct3, rs1, rs2, funct7, address)
	}

	fn i_type(&mut self, op: Op, rd: u32, funct3: u32, rs1: u32, imm: i32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(rd, 5, 0, 7) | 
			bitfield(funct3, 3, 0, 12) | 
			bitfield(rs1, 5, 0, 15) | 
			bitfield(imm as u32, 12, 0, 20);
		let address = self.bytes.len() as u32 + self.start_address;
		self.push_u32(opcode_value);
		AsmResult::new_i_type(op, rd, funct3, rs1, imm, address)
	}
	
	fn i_type_at(&mut self, op: Op, rd: u32, funct3: u32, rs1: u32, imm: i32, address: u32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(rd, 5, 0, 7) | 
			bitfield(funct3, 3, 0, 12) | 
			bitfield(rs1, 5, 0, 15) | 
			bitfield(imm as u32, 12, 0, 20);
		self.write_u32(opcode_value, address);
		AsmResult::new_i_type(op, rd, funct3, rs1, imm, address)
	}

	fn s_type(&mut self, op: Op, funct3: u32, rs1: u32, rs2: u32, imm: i32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(imm as u32, 5, 0, 7) | 
			bitfield(funct3, 3, 0, 12) | 
			bitfield(rs1, 5, 0, 15) | 
			bitfield(rs2, 5, 0, 20) |
			bitfield(imm as u32, 7, 5, 25);
		let address = self.bytes.len() as u32 + self.start_address;
		self.push_u32(opcode_value);
		AsmResult::new_s_type(op, funct3, rs1, rs2, imm, address)
	}
	
	fn s_type_at(&mut self, op: Op, funct3: u32, rs1: u32, rs2: u32, imm: i32, address: u32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(imm as u32, 5, 0, 7) | 
			bitfield(funct3, 3, 0, 12) | 
			bitfield(rs1, 5, 0, 15) | 
			bitfield(rs2, 5, 0, 20) |
			bitfield(imm as u32, 7, 5, 25);
		self.write_u32(opcode_value, address);
		AsmResult::new_s_type(op, funct3, rs1, rs2, imm, address)
	}
	
	fn b_type(&mut self, op: Op, imm: i32, funct3: u32, rs1: u32, rs2: u32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(imm as u32, 1, 11, 7) |
			bitfield(imm as u32, 4, 1, 8) |
			bitfield(funct3, 3, 0, 12) | 
			bitfield(rs1, 5, 0, 15) | 
			bitfield(rs2, 5, 0, 20) |
			bitfield(imm as u32, 6, 5, 25) |
			bitfield(imm as u32, 1, 12, 31);
		let address = self.bytes.len() as u32 + self.start_address;
		self.push_u32(opcode_value);
		AsmResult::new_b_type(op, funct3, rs1, rs2, imm, address)
	}
	
	fn b_type_at(&mut self, op: Op, imm: i32, funct3: u32, rs1: u32, rs2: u32, address: u32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(imm as u32, 1, 11, 7) |
			bitfield(imm as u32, 4, 1, 8) |
			bitfield(funct3, 3, 0, 12) | 
			bitfield(rs1, 5, 0, 15) | 
			bitfield(rs2, 5, 0, 20) |
			bitfield(imm as u32, 6, 5, 25) |
			bitfield(imm as u32, 1, 12, 31);
		self.write_u32(opcode_value, address);
		AsmResult::new_b_type(op, funct3, rs1, rs2, imm, address)
	}

	fn u_type(&mut self, op: Op, rd: u32, imm: i32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(rd, 5, 0, 7) | 
			bitfield(imm as u32, 20, 0, 12);
		let address = self.bytes.len() as u32 + self.start_address;
		self.push_u32(opcode_value);
		AsmResult::new_u_type(op, rd, imm, address)
	}
	
	fn u_type_at(&mut self, op: Op, rd: u32, imm: i32, address: u32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(rd, 5, 0, 7) | 
			bitfield(imm as u32, 20, 0, 12);
		self.write_u32(opcode_value, address);
		AsmResult::new_u_type(op, rd, imm, address)
	}

	fn j_type(&mut self, op: Op, rd: u32, imm: i32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(rd, 5, 0, 7) | 
			bitfield(imm as u32, 10, 1, 21) |
			bitfield(imm as u32, 1, 11, 20) |
			bitfield(imm as u32, 8, 12, 12) |
			bitfield(imm as u32, 1, 20, 31);
		let address = self.bytes.len() as u32 + self.start_address;
		self.push_u32(opcode_value);
		AsmResult::new_j_type(op, rd, imm, address)
	}
	
	fn j_type_at(&mut self, op: Op, rd: u32, imm: i32, address: u32) -> AsmResult {
		let opcode_value = op.to_u32().unwrap() | 
			bitfield(rd, 5, 0, 7) | 
			bitfield(imm as u32, 10, 1, 21) |
			bitfield(imm as u32, 1, 11, 20) |
			bitfield(imm as u32, 8, 12, 12) |
			bitfield(imm as u32, 1, 20, 31);
		self.write_u32(opcode_value, address);
		AsmResult::new_j_type(op, rd, imm, address)
	}
	
	fn write_u32(&mut self, value: u32, address: u32) {
		if (address + 4) > (self.bytes.len() as u32 + self.start_address) || address < self.start_address {
			panic!("attempt to write instruction outside of jit bounds");
		}
		let offset = (address - self.start_address) as usize;
		self.bytes[offset + 0] = (value >> 0) as u8;
		self.bytes[offset + 1] = (value >> 8) as u8;
		self.bytes[offset + 2] = (value >> 16) as u8;
		self.bytes[offset + 3] = (value >> 24) as u8;
	}
	
	fn push_u32(&mut self, value: u32) {
		self.bytes.push((value >> 0) as u8);
		self.bytes.push((value >> 8) as u8);
		self.bytes.push((value >> 16) as u8);
		self.bytes.push((value >> 24) as u8);
	}
}
