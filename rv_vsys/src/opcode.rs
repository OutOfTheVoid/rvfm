#![allow(dead_code)]

use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use num_traits::FromPrimitive;
use unsafe_unwrap::UnsafeUnwrap;

pub struct Opcode {
	pub value: u32
}

#[derive(Debug, Clone, Copy)]
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
	
	Unknown      = 0b00_000_00,
}

impl Op {
	pub fn from_raw(val: u32) -> Self {
		match val {
			0b00_000_11 => Self::Load,
			0b00_001_11 => Self::LoadFp,
			0b00_011_11 => Self::Fence,
			0b00_100_11 => Self::OpImm,
			0b00_101_11 => Self::Auipc,
			0b01_000_11 => Self::Store,
			0b01_001_11 => Self::StoreFp,
			0b01_011_11 => Self::Atomic,
			0b01_100_11 => Self::Op,
			0b01_101_11 => Self::Lui,
			0b10_000_11 => Self::MAdd,
			0b10_001_11 => Self::MSub,
			0b10_010_11 => Self::NMSub,
			0b10_011_11 => Self::NMAdd,
			0b10_100_11 => Self::OpFp,
			0b11_000_11 => Self::Branch,
			0b11_001_11 => Self::Jalr,
			0b11_011_11 => Self::Jal,
			0b11_100_11 => Self::System,
			_ => Self::Unknown,
		}
	}
	
	pub fn to_raw(&self) -> u32 {
		match self {
			Self::Load    => 0b00_000_11,
			Self::LoadFp  => 0b00_001_11,
			Self::Fence   => 0b00_011_11,
			Self::OpImm   => 0b00_100_11,
			Self::Auipc   => 0b00_101_11,
			Self::Store   => 0b01_000_11,
			Self::StoreFp => 0b01_001_11,
			Self::Atomic  => 0b01_011_11,
			Self::Op      => 0b01_100_11,
			Self::Lui     => 0b01_101_11,
			Self::MAdd    => 0b10_000_11,
			Self::MSub    => 0b10_001_11,
			Self::NMSub   => 0b10_010_11,
			Self::NMAdd   => 0b10_011_11,
			Self::OpFp    => 0b10_100_11,
			Self::Branch  => 0b11_000_11,
			Self::Jalr    => 0b11_001_11,
			Self::Jal     => 0b11_011_11,
			Self::System  => 0b11_100_11,
			_ => 0
		}
	}
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

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum StoreFunct3 {
	Byte,
	Half,
	Word,
	Unknown,
}

impl StoreFunct3 {
	pub fn from_raw(raw: u32) -> Self {
		match raw {
			0b000 => Self::Byte,
			0b001 => Self::Half,
			0b010 => Self::Word,
			_ => Self::Unknown
		}
	}
	
	pub fn to_raw(&self) -> u32 {
		match self {
			Self::Byte => 0b000,
			Self::Half => 0b001,
			Self::Word => 0b010,
			Self::Unknown => 0b011,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum LoadFunct3 {
	Byte,
	Half,
	Word,
	ByteUnsigned,
	HalfUnsigned,
	Unknown,
}

impl LoadFunct3 {
	pub fn from_raw(raw: u32) -> Self {
		match raw {
			0b000 => Self::Byte,
			0b001 => Self::Half,
			0b010 => Self::Word,
			0b100 => Self::ByteUnsigned,
			0b101 => Self::HalfUnsigned,
			_ => Self::Unknown,
		}
	}
	
	pub fn to_raw(&self) -> u32 {
		match self {
			Self::Byte => 0b000,
			Self::Half => 0b001,
			Self::Word => 0b010,
			Self::Unknown => 0b011,
			Self::ByteUnsigned => 0b100,
			Self::HalfUnsigned => 0b101,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum OpFunct3Funct7 {
	Add,
	Sub,
	Sll,
	Slt,
	SltU,
	Xor,
	Srl,
	Sra,
	Or,
	And,
	
	Mul,
	MulH,
	MulHSU,
	MulHU,
	Div,
	DivU,
	Rem,
	RemU,
	
	Unknown,
}

impl OpFunct3Funct7 {
	pub fn from_raw(raw: u32) -> Self {
		match raw {
			0b0000000_000 => Self::Add,
			0b0100000_000 => Self::Sub,
			0b0000000_001 => Self::Sll,
			0b0000000_010 => Self::Slt,
			0b0000000_011 => Self::SltU,
			0b0000000_100 => Self::Xor,
			0b0000000_101 => Self::Srl,
			0b0100000_101 => Self::Sra,
			0b0000000_110 => Self::Or,
			0b0000000_111 => Self::And,
			
			0b0000001_000 => Self::Mul,
			0b0000001_001 => Self::MulH,
			0b0000001_010 => Self::MulHSU,
			0b0000001_011 => Self::MulHU,
			0b0000001_100 => Self::Div,
			0b0000001_101 => Self::DivU,
			0b0000001_110 => Self::Rem,
			0b0000001_111 => Self::RemU,
			
			_ => Self::Unknown,
		}
	}
	
	pub fn to_raw(&self) -> u32 {
		match self {
			Self::Add    => 0b0000000_000,
			Self::Sub    => 0b0100000_000,
			Self::Sll    => 0b0000000_001,
			Self::Slt    => 0b0000000_010,
			Self::SltU   => 0b0000000_011,
			Self::Xor    => 0b0000000_100,
			Self::Srl    => 0b0000000_101,
			Self::Sra    => 0b0100000_101,
			Self::Or     => 0b0000000_110,
			Self::And    => 0b0000000_111,
			
			Self::Mul    => 0b0000001_000,
			Self::MulH   => 0b0000001_001,
			Self::MulHSU => 0b0000001_010,
			Self::MulHU  => 0b0000001_011,
			Self::Div    => 0b0000001_100,
			Self::DivU   => 0b0000001_101,
			Self::Rem    => 0b0000001_110,
			Self::RemU   => 0b0000001_111,
			
			Self::Unknown => 0b1111111_111,
		}
	}
	
	pub fn imm3(&self) -> u32 {
		self.to_raw() & 0x07
	}
	
	pub fn imm7(&self) -> u32 {
		self.to_raw() >> 3
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum FpFormatFunct3 {
	Width32,
	Unknown
}

impl FpFormatFunct3 {
	pub fn from_raw(raw: u32) -> Self {
		match raw {
			0b010 => Self::Width32,
			_ => Self::Unknown
		}
	}
	
	pub fn to_raw(&self) -> u32 {
		match self {
			Self::Width32 => 0b010,
			Self::Unknown => 0b000,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum BranchFunct3 {
	Eq,
	NEq,
	Lt,
	GEq,
	LtU,
	GEqU,
	Unknown,
}

impl BranchFunct3 {
	pub fn from_raw(raw: u32) -> Self {
		match raw {
			0b000 => Self::Eq,
			0b001 => Self::NEq,
			0b100 => Self::Lt,
			0b101 => Self::GEq,
			0b110 => Self::LtU,
			0b111 => Self::GEqU,
			_ => Self::Unknown,
		}
	}
	
	pub fn to_raw(&self) -> u32 {
		match self {
			Self::Eq   => 0b000,
			Self::NEq  => 0b001,
			Self::Unknown => 0b010,
			Self::Lt   => 0b100,
			Self::GEq  => 0b101,
			Self::LtU  => 0b110,
			Self::GEqU => 0b111,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum SystemFunct3 {
	Int,
	CsrRW,
	CsrRS,
	CsrRC,
	CsrRWI,
	CsrRSI,
	CsrRCI,
	Unknown
}

impl SystemFunct3 {
	pub fn from_raw(raw: u32) -> Self {
		match raw {
			0b000 => Self::Int,
			0b001 => Self::CsrRW,
			0b010 => Self::CsrRS,
			0b011 => Self::CsrRC,
			0b101 => Self::CsrRWI,
			0b110 => Self::CsrRSI,
			0b111 => Self::CsrRCI,
			_ => Self::Unknown,
		}
	}
	
	pub fn to_raw(&self) -> u32 {
		match self {
			Self::Int => 0b000,
			Self::CsrRW => 0b001,
			Self::CsrRS => 0b010,
			Self::CsrRC => 0b011,
			Self::Unknown => 0b100,
			Self::CsrRWI => 0b101,
			Self::CsrRSI => 0b110,
			Self::CsrRCI => 0b111,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum SystemIntFunct7 {
	WaitForInterrupt,
	MRet,
	Unknown,
}

impl SystemIntFunct7 {
	pub fn from_raw(raw: u32) -> Self {
		match raw {
			0b0001000 => Self::WaitForInterrupt,
			0b0011000 => Self::MRet,
			_ => Self::Unknown,
		}
	}
	
	pub fn to_raw(&self) -> u32 {
		match self {
			Self::WaitForInterrupt => 0b0001000,
			Self::MRet => 0b0011000,
			Self::Unknown => 0xFFFF_FFFF,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum FpFunct7 {
	Add_S,
	Sub_S,
	Mul_S,
	Div_S,
	Sqrt_S,
	Sign_S,
	MinMax_S,
	CvtW_S,
	MvXWClass_S,
	Cmp_S,
	CvtS_W,
	MvWX_S,
	Unknown,
}

impl FpFunct7 {
	fn from_raw(raw: u32) -> Self {
		match raw {
			0b00000_00 => Self::Add_S,
			0b00001_00 => Self::Sub_S,
			0b00010_00 => Self::Mul_S,
			0b00011_00 => Self::Div_S,
			0b01011_00 => Self::Sqrt_S,
			0b00100_00 => Self::Sign_S,
			0b00101_00 => Self::MinMax_S,
			0b11000_00 => Self::CvtW_S,
			0b11100_00 => Self::MvXWClass_S,
			0b10100_00 => Self::Cmp_S,
			0b11010_00 => Self::CvtS_W,
			0b11110_00 => Self::MvWX_S,
			_ => Self::Unknown,
		}
	}
	
	fn to_raw(&self) -> u32 {
		match self {
			Self::Add_S       => 0b00000_00,
			Self::Sub_S       => 0b00001_00,
			Self::Mul_S       => 0b00010_00,
			Self::Div_S       => 0b00011_00,
			Self::Sqrt_S      => 0b01011_00,
			Self::Sign_S      => 0b00100_00,
			Self::MinMax_S    => 0b00101_00,
			Self::CvtW_S      => 0b11000_00,
			Self::MvXWClass_S => 0b11100_00,
			Self::Cmp_S       => 0b10100_00,
			Self::CvtS_W      => 0b11010_00,
			Self::MvWX_S      => 0b11110_00,
			Self::Unknown     => 0b00000_01,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum FpRm {
	ToNearestTieEven         = 0b000,
	ToZero                   = 0b001,
	Down                     = 0b010,
	Up                       = 0b011,
	ToNearestTieMaxMagnitude = 0b100,
	Dynamic                  = 0b111,
	Unknown
}

impl FpRm {
	pub fn from_raw(raw: u32) -> Self {
		match raw {
			0b000 => Self::ToNearestTieEven,
			0b001 => Self::ToZero,
			0b010 => Self::Down,
			0b011 => Self::Up,
			0b100 => Self::ToNearestTieMaxMagnitude,
			0b111 => Self::ToZero,
			_ => Self::Unknown
		}
	}
	
	fn to_raw(&self) -> u32 {
		match self {
			Self::ToNearestTieEven         => 0b000,
			Self::ToZero                   => 0b001,
			Self::Down                     => 0b010,
			Self::Up                       => 0b011,
			Self::ToNearestTieMaxMagnitude => 0b100,
			Self::Dynamic                  => 0b111,
			Self::Unknown                  => 0b101,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum FMvXWClassFunct3 {
	MvXW,
	Class,
	Unknown,
}

impl FMvXWClassFunct3 {
	pub fn from_raw(raw: u32) -> Self {
		match raw {
			0b000 => Self::MvXW,
			0b001 => Self::Class,
			_ => Self::Unknown,
		}
	}
	
	fn to_raw(&self) -> u32 {
		match self {
			Self::MvXW => 0b000,
			Self::Class => 0b001,
			Self::Unknown => 0b010,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum FpSignFunct3 {
	SignFromRs2, // J
	SignFromNotRs2, // JN
	SignFromRs1XorRs2, // JX
	Unknown,
}

impl FpSignFunct3 {
	fn from_raw(raw: u32) -> Self {
		match raw {
			0b000 => Self::SignFromRs2,
			0b001 => Self::SignFromNotRs2,
			0b010 => Self::SignFromRs1XorRs2,
			_ => Self::Unknown
		}
	}
	
	fn to_raw(&self) -> u32 {
		match self {
			Self::SignFromRs2 => 0b000,
			Self::SignFromNotRs2 => 0b001,
			Self::SignFromRs1XorRs2 => 0b010,
			Self::Unknown => 0b011,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum FpMinMaxFunct3 {
	Min,
	Max,
	Unknown,
}

impl FpMinMaxFunct3 {
	fn from_raw(raw: u32) -> Self {
		match raw {
			0b000 => Self::Min,
			0b001 => Self::Max,
			_ => Self::Unknown
		}
	}
	
	fn to_raw(&self) -> u32 {
		match self {
			Self::Min => 0b000,
			Self::Max => 0b001,
			Self::Unknown => 0b010,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum FCvtType {
	Signed,
	Unsigned,
	Unknown,
}

impl FCvtType {
	fn from_raw(raw: u32) -> Self {
		match raw {
			0b000 => Self::Signed,
			0b001 => Self::Unsigned,
			_ => Self::Unknown
		}
	}
	
	fn to_raw(&self) -> u32 {
		match self {
			Self::Signed => 0b000,
			Self::Unsigned => 0b001,
			Self::Unknown => 0b010,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum FpCmpFunct3 {
	LEq,
	Lt,
	Eq,
	Unknown
}

impl FpCmpFunct3 {
	fn from_raw(raw: u32) -> Self {
		match raw {
			0b000 => Self::LEq,
			0b001 => Self::Lt,
			0b010 => Self::Eq,
			_ => Self::Unknown
		}
	}
	
	fn to_raw(&self) -> u32 {
		match self {
			Self::LEq => 0b000,
			Self::Lt => 0b001,
			Self::Eq => 0b010,
			Self::Unknown => 0b011,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum AtomicFunct7 {
	LoadReserve,
	StoreConditional,
	Swap,
	Add,
	Xor,
	And,
	Or,
	Min,
	Max,
	MinU,
	MaxU,
	Unknown,
}

impl AtomicFunct7 {
	pub fn from_raw(raw: u32) -> Self {
		match raw {
			0b00000 => Self::Add,
			0b00001 => Self::Swap,
			0b00010 => Self::LoadReserve,
			0b00011 => Self::StoreConditional,
			0b00100 => Self::Xor,
			0b01000 => Self::Or,
			0b01100 => Self::And,
			0b10000 => Self::Min,
			0b10100 => Self::Max,
			0b11000 => Self::MinU,
			0b11100 => Self::MaxU,
			_ => Self::Unknown,
		}
	}
	
	pub fn to_raw(&self) -> u32 {
		match self {
			Self::LoadReserve =>      0b00010,
			Self::StoreConditional => 0b00011,
			Self::Swap =>             0b00001,
			Self::Add =>              0b00000,
			Self::Xor =>              0b00100,
			Self::And =>              0b00100,
			Self::Or =>               0b01000,
			Self::Min =>              0b10000,
			Self::Max =>              0b10100,
			Self::MinU =>             0b11000,
			Self::MaxU =>             0b11100,
			Self::Unknown =>          0b11101,
		}
	}
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum AtomicSizeFunct3 {
	Word,
	Unknown,
}

impl AtomicSizeFunct3 {
	pub fn from_raw(raw: u32) -> Self {
		match raw {
			0b010 => Self::Word,
			_ => Self::Unknown,
		}
	}
	
	pub fn to_raw(&self) -> u32 {
		match self {
			Self::Word => 0b010,
			Self::Unknown => 0b000,
		}
	}
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

	pub fn op(&self) -> Op {
		Op::from_raw(self.value & 0x7F)
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
	
	pub fn fp_rm(&self) -> FpRm {
		FpRm::from_raw(bitfield(self.value, 3, 12, 0))
	}
	
	pub fn funct3_fpsign(&self) -> FpSignFunct3 {
		FpSignFunct3::from_raw(bitfield(self.value, 3, 12, 0))
	}
	
	pub fn funct3_fpminmax(&self) -> FpMinMaxFunct3 {
		FpMinMaxFunct3::from_raw(bitfield(self.value, 3, 12, 0))
	}
	
	pub fn funct3_fmvxwclass(&self) -> FMvXWClassFunct3 {
		FMvXWClassFunct3::from_raw(bitfield(self.value, 3, 12, 0))
	}
	
	pub fn funct3_fpcmp(&self) -> FpCmpFunct3 {
		FpCmpFunct3::from_raw(bitfield(self.value, 3, 12, 0))
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
		unsafe { OpImmFunct3::from_u32(self.funct3()).unsafe_unwrap() }
	}
	
	pub fn funct3_store(&self) -> StoreFunct3 {
		StoreFunct3::from_raw(self.funct3())
	}
	
	pub fn funct3_load(&self) -> LoadFunct3 {
		LoadFunct3::from_raw(self.funct3())
	}
	
	pub fn funct3_loadfp(&self) -> LoadFpFunct3 {
		LoadFpFunct3::from_raw(self.funct3())
	}
	
	pub fn funct3_fpformat(&self) -> FpFormatFunct3 {
		FpFormatFunct3::from_raw(self.funct3())
	}
	
	pub fn funct3funct7_op(&self) -> OpFunct3Funct7 {
		OpFunct3Funct7::from_raw(self.funct3_7())
	}
	
	pub fn funct3_branch(&self) -> BranchFunct3 {
		BranchFunct3::from_raw(self.funct3())
	}
	
	pub fn funct3_atomicsize(&self) -> AtomicSizeFunct3 {
		AtomicSizeFunct3::from_raw(self.funct3())
	}
	
	pub fn funct3_system(&self) -> SystemFunct3 {
		SystemFunct3::from_raw(self.funct3())
	}
	
	pub fn funct7_system_int(&self) -> SystemIntFunct7 {
		SystemIntFunct7::from_raw(self.funct7())
	}
	
	pub fn funct7_fp(&self) -> FpFunct7 {
		FpFunct7::from_raw(self.funct7())
	}
	
	pub fn funct7_atomic(&self) -> AtomicFunct7 {
		AtomicFunct7::from_raw(self.funct7())
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
	
	pub fn rs2_fcvtws(&self) -> FCvtType {
		FCvtType::from_raw(bitfield(self.value, 5, 20, 0))
	}
	
	pub fn rs3(&self) -> u32 {
		bitfield(self.value, 5, 27, 0)
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