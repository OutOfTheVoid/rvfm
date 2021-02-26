use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use num_traits::FromPrimitive;
use num_traits::ToPrimitive;

pub struct Opcode {
	pub value: u32
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Op {
	Load         = 0b00_000_11,
	LoadFp       = 0b00_001_11,
	Fence        = 0b00_011_11,
	OpImm        = 0b00_100_11,
	Auipc        = 0b00_101_11,
	Store        = 0b01_000_11,
	StoreFp      = 0b01_001_11,
	Atomic       = 0b01_011_11,
	Op           = 0b01_100_11,
	Lui          = 0b01_101_11,
	MAdd         = 0b10_000_11,
	MSub         = 0b10_001_11,
	NMSub        = 0b10_010_11,
	NMAdd        = 0b10_011_11,
	OpFp         = 0b10_100_11,
	Branch       = 0b11_000_11,
	Jalr         = 0b11_001_11,
	Jal          = 0b11_011_11,
	System       = 0b11_100_11,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum OpImmFunct3 {
	AddI  = 0b000,
	SllI  = 0b001,
	SltI  = 0b010,
	SltIU = 0b011,
	XorI  = 0b100,
	SrxI  = 0b101,
	OrI   = 0b110,
	AndI  = 0b111,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum StoreFunct3 {
	Byte = 0b000,
	Half = 0b001,
	Word = 0b010,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum LoadFunct3 {
	Byte = 0b000,
	Half = 0b001,
	Word = 0b010,
	ByteUnsigned = 0b100,
	HalfUnsigned = 0b101,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum OpFunct3Funct7 {
	//       funct7    funct3
	Add    = 0b0000000_000,
	Sub    = 0b0100000_000,
	Sll    = 0b0000000_001,
	Slt    = 0b0000000_010,
	SltU   = 0b0000000_011,
	Xor    = 0b0000000_100,
	Srl    = 0b0000000_101,
	Sra    = 0b0100000_101,
	Or     = 0b0000000_110,
	And    = 0b0000000_111,
	
	Mul    = 0b0000001_000,
	MulH   = 0b0000001_001,
	MulHSU = 0b0000001_010,
	MulHU  = 0b0000001_011,
	Div    = 0b0000001_100,
	DivU   = 0b0000001_101,
	Rem    = 0b0000001_110,
	RemU   = 0b0000001_111,
}

impl OpFunct3Funct7 {
	pub fn imm3(&self) -> u32 {
		self.to_u32().unwrap() & 0x07
	}
	
	pub fn imm7(&self) -> u32 {
		self.to_u32().unwrap() >> 3
	}
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum LoadFpFunct3 {
	Width32 = 0b010
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum StoreFpFunct3 {
	Width32 = 0b010
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum BranchFunct3 {
	Eq   = 0b000,
	NEq  = 0b001,
	Lt   = 0b100,
	GEq  = 0b101,
	LtU  = 0b110,
	GEqU = 0b111,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum SystemFunct3 {
	Int    = 0b000,
	CsrRW  = 0b001,
	CsrRS  = 0b010,
	CsrRC  = 0b011,
	CsrRWI = 0b101,
	CsrRSI = 0b110,
	CsrRCI = 0b111,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum SystemIntFunct7 {
	WaitForInterrupt = 0b0001000,
	MRet             = 0b0011000,
}

fn bitfield(x: u32, count: u32, src_bit: u32, dst_bit: u32) -> u32 {
	let mask = (1u32 << count).wrapping_sub(1);
	let y = x >> src_bit;
	(y & mask) << dst_bit
}

impl Opcode {
	pub fn new(value: u32) -> Self {
		Opcode {
			value: value
		}
	}

	pub fn op(&self) -> Option<Op> {
		Op::from_u32(self.value & 0x7F)
	}

	pub fn rd(&self) -> u32 {
		bitfield(self.value, 5, 7, 0)
	}

	pub fn s_imm_low(&self) -> u32 {
		bitfield(self.value, 5, 7, 0)
	}

	pub fn s_imm_high(&self) -> u32 {
		bitfield(self.value, 7, 25, 0)
	}

	pub fn s_imm_signed(&self) -> i32 {
		let val_raw = 
			bitfield(self.value, 5, 7, 0) |
			bitfield(self.value, 7, 25, 5);
		if (val_raw & 0x800) != 0 {
			(val_raw | 0xFFFFF000) as i32
		} else {
			val_raw as i32
		}
	}

	pub fn funct3(&self) -> u32 {
		bitfield(self.value, 3, 12, 0)
	}
	
	pub fn funct3_7(&self) -> u32 {
		bitfield(self.value, 3, 12, 0) |
		bitfield(self.value, 7, 25, 3)
	}
	
	pub fn funct3_op_imm(&self) -> OpImmFunct3 {
		OpImmFunct3::from_u32(self.funct3()).unwrap()
	}
	
	pub fn funct3_store(&self) -> StoreFunct3 {
		StoreFunct3::from_u32(self.funct3()).unwrap()
	}
	
	pub fn funct3_load(&self) -> LoadFunct3 {
		LoadFunct3::from_u32(self.funct3()).unwrap()
	}
	
	pub fn funct3_loadfp(&self) -> LoadFpFunct3 {
		LoadFpFunct3::from_u32(self.funct3()).unwrap()
	}
	
	pub fn funct3_storefp(&self) -> StoreFpFunct3 {
		StoreFpFunct3::from_u32(self.funct3()).unwrap()
	}
	
	pub fn funct3funct7_op(&self) -> OpFunct3Funct7 {
		OpFunct3Funct7::from_u32(self.funct3_7()).unwrap()
	}
	
	pub fn funct3_branch(&self) -> BranchFunct3 {
		BranchFunct3::from_u32(self.funct3()).unwrap()
	}
	
	pub fn funct3_system(&self) -> SystemFunct3 {
		SystemFunct3::from_u32(self.funct3()).unwrap()
	}
	
	pub fn funct7_system_int(&self) -> SystemIntFunct7 {
		SystemIntFunct7::from_u32(self.funct7()).unwrap()
	}

	pub fn rs1(&self) -> u32 {
		bitfield(self.value, 5, 15, 0)
	}
	
	pub fn shamt(&self) -> u32 {
		bitfield(self.value, 5, 20, 0)
	}
	
	pub fn srxi_is_arithmetic(&self) -> bool {
		self.value & 0x4000_0000 != 0
	}
	
	pub fn srx_is_arithmetic(&self) -> bool {
		self.value & 0x4000_0000 != 0
	}
	
	pub fn addsub_is_sub(&self) -> bool {
		self.value & 0x4000_0000 != 0
	}

	pub fn rs2(&self) -> u32 {
		bitfield(self.value, 5, 20, 0)
	}

	pub fn i_imm(&self) -> u32 {
		bitfield(self.value, 12, 20, 0)
	}

	pub fn i_imm_signed(&self) -> i32 {
		let val_raw = self.i_imm();
		if (val_raw & 0x800) != 0 {
			(val_raw | 0xFFFFF000) as i32
		} else {
			val_raw as i32
		}
	}

	pub fn funct7(&self) -> u32 {
		bitfield(self.value, 7, 25, 0)
	}

	pub fn u_imm(&self) -> u32 {
		bitfield(self.value, 20, 12, 0)
	}

	pub fn u_imm_signed(&self) -> i32 {
		let val_raw = self.u_imm();
		if (val_raw & 0x80000) != 0 {
			(val_raw | 0xFFF00000) as i32
		} else {
			val_raw as i32
		}
	}
	
	pub fn j_imm(&self) -> u32 {
		bitfield(self.value, 10, 21, 1) |
		bitfield(self.value, 1, 20, 11) |
		bitfield(self.value, 8, 12, 12) |
		bitfield(self.value, 1, 31, 20)
	}

	pub fn j_imm_signed(&self) -> i32 {
		let val_raw = self.j_imm();
		if (val_raw &  0x00100000) != 0 {
			(val_raw | 0xFFF00000) as i32
		} else {
			val_raw as i32
		}
	}
	
	pub fn b_imm(&self) -> u32 {
		bitfield(self.value, 1, 7, 11) |
		bitfield(self.value, 4, 8, 1) |
		bitfield(self.value, 6, 25, 5) |
		bitfield(self.value, 1, 31, 12)
	}
	
	pub fn b_imm_signed(&self) -> i32 {
		let val_raw = self.b_imm();
		if (val_raw &  0x00001000) != 0 {
			(val_raw | 0xFFFFF000) as i32
		} else {
			val_raw as i32
		}
	}
}