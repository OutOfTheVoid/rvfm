#![allow(dead_code, non_snake_case)]

use std::slice;
use num_traits::pow::Pow;
use parking_lot::Mutex;
use rv_vsys::{MemIO, MemReadResult, MemWriteResult};
use crate::fm_mio::FmMemoryIO;

pub struct MathAcceleratorData {
	regs: Box<[f32]>,
	error: u32,
}

pub struct MathAccelerator {
	data: Mutex<MathAcceleratorData>
}

unsafe impl Sync for MathAccelerator {}

const REG_NUM_0: u32 = 0;           // 0x00
const REG_NUM_63: u32 = 63;         // 0xFC

const REG_LOAD_VEC2_0: u32 = 64;    // 0x100
const REG_LOAD_VEC2_15: u32 = 79;   // 0x13C
const REG_STORE_VEC2_0: u32 = 80;   // 0x140
const REG_STORE_VEC2_15: u32 = 95;  // 0x17C

const REG_LOAD_VEC3_0: u32 = 96;    // 0x180
const REG_LOAD_VEC3_15: u32 = 111;  // 0x1BC
const REG_STORE_VEC3_0: u32 = 112;  // 0x1C0
const REG_STORE_VEC3_15: u32 = 127; // 0x1FC

const REG_LOAD_VEC4_0: u32 = 128;   // 0x200
const REG_LOAD_VEC4_15: u32 = 143;  // 0x23C
const REG_STORE_VEC4_0: u32 = 144;  // 0x240
const REG_STORE_VEC4_15: u32 = 149; // 0x27C

const REG_NUM_COMMAND: u32 = 255;    // 0x3FC
const REG_NUM_ERROR: u32 = 254;      // 0x3F8

// VEC OP VEC => VEC

const COMMAND_OP_VEC_VEC_ADD2_VEC: u32 = 0x00;
const COMMAND_OP_VEC_VEC_ADD3_VEC: u32 = 0x01;
const COMMAND_OP_VEC_VEC_ADD4_VEC: u32 = 0x02;

const COMMAND_OP_VEC_VEC_SUB2_VEC: u32 = 0x03;
const COMMAND_OP_VEC_VEC_SUB3_VEC: u32 = 0x04;
const COMMAND_OP_VEC_VEC_SUB4_VEC: u32 = 0x05;

const COMMAND_OP_VEC_VEC_MUL2_VEC: u32 = 0x06;
const COMMAND_OP_VEC_VEC_MUL3_VEC: u32 = 0x07;
const COMMAND_OP_VEC_VEC_MUL4_VEC: u32 = 0x08;

const COMMAND_OP_VEC_VEC_DIV2_VEC: u32 = 0x09;
const COMMAND_OP_VEC_VEC_DIV3_VEC: u32 = 0x0A;
const COMMAND_OP_VEC_VEC_DIV4_VEC: u32 = 0x0B;

const COMMAND_OP_VEC_VEC_REM2_VEC: u32 = 0x0C;
const COMMAND_OP_VEC_VEC_REM3_VEC: u32 = 0x0D;
const COMMAND_OP_VEC_VEC_REM4_VEC: u32 = 0x0E;

const COMMAND_OP_VEC_VEC_POW2_VEC: u32 = 0x0F;
const COMMAND_OP_VEC_VEC_POW3_VEC: u32 = 0x10;
const COMMAND_OP_VEC_VEC_POW4_VEC: u32 = 0x11;

const COMMAND_OP_VEC_VEC_PROJECT2_VEC: u32 = 0x12;
const COMMAND_OP_VEC_VEC_PROJECT3_VEC: u32 = 0x13;
const COMMAND_OP_VEC_VEC_PROJECT4_VEC: u32 = 0x14;

const COMMAND_OP_VEC_VEC_CROSS_VEC: u32 = 0x15;

const COMMAND_OP_VEC_VEC_QROTATE_VEC: u32 = 0x16;
const COMMAND_OP_VEC_VEC_QMUL_VEC: u32 = 0x17;

// VEC OP VEC => R

const COMMAND_OP_VEC_VEC_DOT2_R: u32 = 0x20;
const COMMAND_OP_VEC_VEC_DOT3_R: u32 = 0x21;
const COMMAND_OP_VEC_VEC_DOT4_R: u32 = 0x22;

// VEC OP => R

const COMMAND_OP_VEC_LENGTH2_R: u32 = 0x40;
const COMMAND_OP_VEC_LENGTH3_R: u32 = 0x41;
const COMMAND_OP_VEC_LENGTH4_R: u32 = 0x42;

// VEC OP => VEC

const COMMAND_OP_VEC_NORM2_VEC: u32 = 0x50;
const COMMAND_OP_VEC_NORM3_VEC: u32 = 0x51;
const COMMAND_OP_VEC_NORM4_VEC: u32 = 0x52;

// VEC OP R => VEC

const COMMAND_OP_VEC_R_SCALE2_VEC: u32 = 0x60;
const COMMAND_OP_VEC_R_SCALE3_VEC: u32 = 0x61;
const COMMAND_OP_VEC_R_SCALE4_VEC: u32 = 0x62;
const COMMAND_OP_VEC_R_ANGLEAXISQUAT_VEC: u32 = 0x63;
const COMMAND_OP_VEC_R_ROTATE_VEC: u32 = 0x64;

// R OP R => R

const COMMAND_OP_R_R_ADD_R: u32 = 0x80;
const COMMAND_OP_R_R_SUB_R: u32 = 0x81;
const COMMAND_OP_R_R_MUL_R: u32 = 0x82;
const COMMAND_OP_R_R_DIV_R: u32 = 0x83;
const COMMAND_OP_R_R_REM_R: u32 = 0x84;
const COMMAND_OP_R_R_POW_R: u32 = 0x85;
const COMMAND_OP_R_R_ATAN2_R: u32 = 0x86;
const COMMAND_OP_R_R_LOG_R: u32 = 0x87;

// R OP => R

const COMMAND_OP_R_SIN_R: u32 = 0xA0;
const COMMAND_OP_R_COS_R: u32 = 0xA1;
const COMMAND_OP_R_TAN_R: u32 = 0xA2;
const COMMAND_OP_R_ARCSIN_R: u32 = 0xA3;
const COMMAND_OP_R_ARCCOS_R: u32 = 0xA4;
const COMMAND_OP_R_ARCTAN_R: u32 = 0xA5;
const COMMAND_OP_R_EXP_R: u32 = 0xA6;
const COMMAND_OP_R_LN_R: u32 = 0xA7;
const COMMAND_OP_R_INV_R: u32 = 0xA8;

// VEC, VEC OP R => VEC

const COMMAND_OP_VEC_VEC_R_QSLERP_VEC: u32 = 0xC0;

const ERROR_NONE: u32 = 0;

const ERROR_VECLOAD_MEMORY_ERROR: u32 = 1;
const ERROR_VECSTORE_MEMORY_ERROR: u32 = 2;
const ERROR_UNKNOWN_REG: u32 = 3;
const ERROR_UNKNOWN_OP: u32 = 4;

fn read_v2(from: &[f32]) -> (f32, f32) {
	(from[0], from[1])
}

fn read_v3(from: &[f32]) -> (f32, f32, f32) {
	(from[0], from[1], from[2])
}

fn read_v4(from: &[f32]) -> (f32, f32, f32, f32) {
	(from[0], from[1], from[2], from[3])
}

fn write_v2(to: &mut [f32], v: (f32, f32)) {
	let (x, y) = v;
	to[0] = x;
	to[1] = y;
}

fn write_v3(to: &mut [f32], v: (f32, f32, f32)) {
	let (x, y, z) = v;
	to[0] = x;
	to[1] = y;
	to[2] = z;
}

fn write_v4(to: &mut [f32], v: (f32, f32, f32, f32)) {
	let (x, y, z, w) = v;
	to[0] = x;
	to[1] = y;
	to[2] = z;
	to[3] = w;
}

fn add_v2(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
	let (a_x, a_y) = a;
	let (b_x, b_y) = b;
	(a_x + b_x, a_y + b_y)
}

fn add_v3(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
	let (a_x, a_y, a_z) = a;
	let (b_x, b_y, b_z) = b;
	(a_x + b_x, a_y + b_y, a_z + b_z)
}

fn add_v4(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
	let (a_x, a_y, a_z, a_w) = a;
	let (b_x, b_y, b_z, b_w) = b;
	(a_x + b_x, a_y + b_y, a_z + b_z, a_w + b_w)
}

fn sub_v2(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
	let (a_x, a_y) = a;
	let (b_x, b_y) = b;
	(a_x - b_x, a_y - b_y)
}

fn sub_v3(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
	let (a_x, a_y, a_z) = a;
	let (b_x, b_y, b_z) = b;
	(a_x - b_x, a_y - b_y, a_z - b_z)
}

fn sub_v4(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
	let (a_x, a_y, a_z, a_w) = a;
	let (b_x, b_y, b_z, b_w) = b;
	(a_x - b_x, a_y - b_y, a_z - b_z, a_w - b_w)
}

fn mul_v2(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
	let (a_x, a_y) = a;
	let (b_x, b_y) = b;
	(a_x * b_x, a_y * b_y)
}

fn mul_v3(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
	let (a_x, a_y, a_z) = a;
	let (b_x, b_y, b_z) = b;
	(a_x * b_x, a_y * b_y, a_z * b_z)
}

fn mul_v4(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
	let (a_x, a_y, a_z, a_w) = a;
	let (b_x, b_y, b_z, b_w) = b;
	(a_x * b_x, a_y * b_y, a_z * b_z, a_w * b_w)
}

fn div_v2(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
	let (a_x, a_y) = a;
	let (b_x, b_y) = b;
	(a_x / b_x, a_y / b_y)
}

fn div_v3(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
	let (a_x, a_y, a_z) = a;
	let (b_x, b_y, b_z) = b;
	(a_x / b_x, a_y / b_y, a_z / b_z)
}

fn div_v4(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
	let (a_x, a_y, a_z, a_w) = a;
	let (b_x, b_y, b_z, b_w) = b;
	(a_x / b_x, a_y / b_y, a_z / b_z, a_w / b_w)
}

fn rem_v2(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
	let (a_x, a_y) = a;
	let (b_x, b_y) = b;
	(a_x % b_x, a_y % b_y)
}

fn rem_v3(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
	let (a_x, a_y, a_z) = a;
	let (b_x, b_y, b_z) = b;
	(a_x % b_x, a_y % b_y, a_z % b_z)
}

fn rem_v4(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
	let (a_x, a_y, a_z, a_w) = a;
	let (b_x, b_y, b_z, b_w) = b;
	(a_x % b_x, a_y % b_y, a_z % b_z, a_w % b_w)
}

fn pow_v2(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
	let (a_x, a_y) = a;
	let (b_x, b_y) = b;
	(a_x.pow(b_x), a_y.pow(b_y))
}

fn pow_v3(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
	let (a_x, a_y, a_z) = a;
	let (b_x, b_y, b_z) = b;
	(a_x.pow(b_x), a_y.pow(b_y), a_z.pow(b_z))
}

fn pow_v4(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
	let (a_x, a_y, a_z, a_w) = a;
	let (b_x, b_y, b_z, b_w) = b;
	(a_x.pow(b_x), a_y.pow(b_y), a_z.pow(b_z), a_w.pow(b_w))
}

fn length_v2(v: (f32, f32)) -> f32 {
	let (x, y) = v;
	f32::sqrt(x * x + y * y)
}

fn length_v3(v: (f32, f32, f32)) -> f32 {
	let (x, y, z) = v;
	f32::sqrt(x * x + y * y + z * z)
}

fn length_v4(v: (f32, f32, f32, f32)) -> f32 {
	let (x, y, z, w) = v;
	f32::sqrt(x * x + y * y + z * z + w * w)
}

fn length_sq_v2(v: (f32, f32)) -> f32 {
	let (x, y) = v;
	x * x + y * y
}

fn length_sq_v3(v: (f32, f32, f32)) -> f32 {
	let (x, y, z) = v;
	x * x + y * y + z * z
}

fn length_sq_v4(v: (f32, f32, f32, f32)) -> f32 {
	let (x, y, z, w) = v;
	x * x + y * y + z * z + w * w
}

fn dot_v2(a: (f32, f32), b: (f32, f32)) -> f32 {
	let (a_x, a_y) = a;
	let (b_x, b_y) = b;
	a_x * b_x + a_y * b_y
}

fn dot_v3(a: (f32, f32, f32), b: (f32, f32, f32)) -> f32 {
	let (a_x, a_y, a_z) = a;
	let (b_x, b_y, b_z) = b;
	a_x * b_x + a_y * b_y + a_z * b_z
}

fn dot_v4(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> f32 {
	let (a_x, a_y, a_z, a_w) = a;
	let (b_x, b_y, b_z, b_w) = b;
	a_x * b_x + a_y * b_y + a_z * b_z + a_w * b_w
}

fn cross(a: (f32, f32, f32), b: (f32, f32, f32)) -> (f32, f32, f32) {
	let (a_x, a_y, a_z) = a;
	let (b_x, b_y, b_z) = b;
	(
		a_y * b_z - a_z * b_y,
		a_z * b_x - a_x * b_z,
		a_x * b_y - a_y * b_x
	)
}

fn inverse_q(q: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
	let length = length_v4(q);
	let (r, i, j, k) = q;
	let q_p = (r, -i, -j, -k);
	scale_v4(q_p, 1.0 / length)
}

fn mul_q(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
	let (a_r, a_i, a_j, a_k) = a;
	let (b_r, b_i, b_j, b_k) = b;
	let a_dir = (a_i, a_j, a_k);
	let b_dir = (b_i, b_j, b_k);
	let (result_i, result_j, result_k) = add_v3(
		cross(a_dir, b_dir), 
		add_v3(
			scale_v3(b_dir, a_r),
			scale_v3(a_dir, b_r
		)
	));
	let result_r = a_r * b_r - dot_v3(a_dir, b_dir);
	(result_r, result_i, result_j, result_k)
}

fn rotate_v2(v: (f32, f32), r: f32) -> (f32, f32) {
	let (x, y) = v;
	let r_sin = r.sin();
	let r_cos = r.cos();
	(r_cos * x + r_sin * y, r_cos * y - r_sin * x)
}

fn rotate_v3(v: (f32, f32, f32), q: (f32, f32, f32, f32)) -> (f32, f32, f32) {
	let (s, d_x, d_y, d_z) = q;
	let u = (d_x, d_y, d_z);
	add_v3(
		add_v3(
			scale_v3(u, 2.0 * dot_v3(u, v)),
			scale_v3(v, s * s - length_sq_v3(u))
		),
		scale_v3(cross(u, v), 2.0 * s)
	)
}

fn q_from_angle_axis(angle: f32, axis: (f32, f32, f32)) -> (f32, f32, f32, f32) {
	let r = angle * 0.5;
	let r_sin = r.sin();
	let r_cos = r.cos();
	let (i, j, k) = scale_v3(axis, r_sin);
	let q = (r_cos, i, j, k);
	let scale = 1.0 / length_v4(q);
	scale_v4(q, scale)
}

fn scale_v2(v: (f32, f32), scale: f32) -> (f32, f32) {
	let (x, y) = v;
	(x * scale, y * scale)
}

fn scale_v3(v: (f32, f32, f32), scale: f32) -> (f32, f32, f32) {
	let (x, y, z) = v;
	(x * scale, y * scale, z * scale)
}

fn scale_v4(v: (f32, f32, f32, f32), scale: f32) -> (f32, f32, f32, f32) {
	let (x, y, z, w) = v;
	(x * scale, y * scale, z * scale, w * scale)
}

impl MathAcceleratorData {
	pub fn new() -> Self {
		Self {
			regs: vec![0.0; 64].into_boxed_slice(),
			error: ERROR_NONE,
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		if (offset & 0x03) != 0 {
			return MemReadResult::ErrAlignment;
		}
		let reg_num = offset >> 2;
		match reg_num >> 2 {
			REG_NUM_0 ..= REG_NUM_63 => MemReadResult::Ok(self.regs[reg_num as usize].to_bits()),
			REG_NUM_ERROR => MemReadResult::Ok(self.error),
			_ => MemReadResult::Ok(0),
		}
	}
	
	pub fn write_32(&mut self, mio: &mut FmMemoryIO, offset: u32, data: u32) -> MemWriteResult {
		if (offset & 0x03) != 0 {
			return MemWriteResult::ErrAlignment;
		}
		let reg_num = offset >> 2;
		match reg_num {
			REG_NUM_0 ..= REG_NUM_63 => {
				self.regs[reg_num as usize] = f32::from_bits(data);
				MemWriteResult::Ok
			},
			REG_NUM_COMMAND => {
				if self.do_command(data) {
					MemWriteResult::Ok
				} else {
					MemWriteResult::PeripheralError
				}
			},
			REG_NUM_ERROR => {
				self.error = 0;
				MemWriteResult::Ok
			},
			REG_LOAD_VEC2_0 ..= REG_LOAD_VEC2_15 => {
				let vec_num = reg_num - REG_LOAD_VEC2_0;
				let reg_start = vec_num << 2;
				let address = data;
				for i in 0 .. 2 {
					let component_address = address + (i << 2);
					match mio.read_32(component_address) {
						MemReadResult::Ok(value) => {
							self.regs[(reg_start + i) as usize] = f32::from_bits(value);
						},
						_ => {
							self.error = ERROR_VECLOAD_MEMORY_ERROR;
							return MemWriteResult::PeripheralError;
						}
					}
				}
				MemWriteResult::Ok
			},
			REG_STORE_VEC2_0 ..= REG_STORE_VEC2_15 => {
				let reg_start = (reg_num - REG_STORE_VEC2_0) << 2;
				let address = data;
				for i in 0 .. 2 {
					let component_address = address + (i << 2);
					match mio.write_32(component_address, self.regs[(reg_start + i) as usize].to_bits()) {
						MemWriteResult::Ok => {},
						_ => {
							self.error = ERROR_VECSTORE_MEMORY_ERROR;
							return MemWriteResult::PeripheralError;
						}
					}
				}
				MemWriteResult::Ok
			},
			REG_LOAD_VEC3_0 ..= REG_LOAD_VEC3_15 => {
				let reg_start = (reg_num - REG_LOAD_VEC3_0) << 2;
				let address = data;
				for i in 0 .. 3 {
					let component_address = address + (i << 2);
					match mio.read_32(component_address) {
						MemReadResult::Ok(value) => {
							self.regs[(reg_start + i) as usize] = f32::from_bits(value);
						},
						_ => {
							self.error = ERROR_VECLOAD_MEMORY_ERROR;
							return MemWriteResult::PeripheralError;
						}
					}
				}
				MemWriteResult::Ok
			},
			REG_STORE_VEC3_0 ..= REG_STORE_VEC3_15 => {
				let reg_start = (reg_num - REG_STORE_VEC3_0) << 2;
				let address = data;
				for i in 0 .. 3 {
					let component_address = address + (i << 2);
					match mio.write_32(component_address, self.regs[(reg_start + i) as usize].to_bits()) {
						MemWriteResult::Ok => {},
						_ => {
							self.error = ERROR_VECSTORE_MEMORY_ERROR;
							return MemWriteResult::PeripheralError;
						}
					}
				}
				MemWriteResult::Ok
			},
			REG_LOAD_VEC4_0 ..= REG_LOAD_VEC4_15 => {
				let reg_start = (reg_num - REG_LOAD_VEC4_0) << 2;
				let address = data;
				for i in 0 .. 4 {
					let component_address = address + (i << 2);
					match mio.read_32(component_address) {
						MemReadResult::Ok(value) => {
							self.regs[(reg_start + i) as usize] = f32::from_bits(value);
						},
						_ => {
							self.error = ERROR_VECLOAD_MEMORY_ERROR;
							return MemWriteResult::PeripheralError;
						}
					}
				}
				MemWriteResult::Ok
			},
			REG_STORE_VEC4_0 ..= REG_STORE_VEC4_15 => {
				let reg_start = (reg_num - REG_STORE_VEC4_0) << 2;
				let address = data;
				for i in 0 .. 4 {
					let component_address = address + (i << 2);
					match mio.write_32(component_address, self.regs[(reg_start + i) as usize].to_bits()) {
						MemWriteResult::Ok => {},
						_ => {
							self.error = ERROR_VECSTORE_MEMORY_ERROR;
							return MemWriteResult::PeripheralError;
						}
					}
				}
				MemWriteResult::Ok
			},
			_ => {
				self.error = ERROR_UNKNOWN_REG;
				MemWriteResult::PeripheralError
			}
		}
	}
	
	fn vec_vec_vec_refs(&mut self, command: u32) -> (&'static mut [f32], &'static mut [f32], &'static mut [f32]) {
		let (src_a_index, src_b_index, dest_index) = (
			(command >> 8) & 0x0F,
			(command >> 12) & 0x0F,
			(command >> 16) & 0x0F
		);
		unsafe {
			(
				slice::from_raw_parts_mut::<'static>(&mut self.regs[(src_a_index << 2) as usize] as *mut f32, 4),
				slice::from_raw_parts_mut::<'static>(&mut self.regs[(src_b_index << 2) as usize] as *mut f32, 4),
				slice::from_raw_parts_mut::<'static>(&mut self.regs[(dest_index << 2) as usize] as *mut f32, 4)
			)
		}
	}
	
	fn vec_vec_r_refs(&mut self, command: u32) -> (&'static mut [f32], &'static mut [f32], &'static mut f32) {
		let (src_a_index, src_b_index, dest_index) = (
			(command >> 8) & 0x0F,
			(command >> 12) & 0x0F,
			(command >> 16) & 0x3F
		);
		unsafe {
			(
				slice::from_raw_parts_mut::<'static>(&mut self.regs[(src_a_index << 2) as usize] as *mut f32, 4),
				slice::from_raw_parts_mut::<'static>(&mut self.regs[(src_b_index << 2) as usize] as *mut f32, 4),
				&mut *(&mut self.regs[dest_index as usize] as *mut f32)
			)
		}
	}
	
	fn vec_r_refs(&mut self, command: u32) -> (&'static mut [f32], &'static mut f32) {
		let (src_index, dest_index) = (
			(command >> 8) & 0x0F,
			(command >> 12) & 0x3F
		);
		unsafe {
			(
				slice::from_raw_parts_mut::<'static>(&mut self.regs[(src_index << 2) as usize] as *mut f32, 4),
				&mut *(&mut self.regs[dest_index as usize] as *mut f32)
			)
		}
	}
	
	fn vec_r_vec_refs(&mut self, command: u32) -> (&'static mut [f32], &'static mut f32, &'static mut [f32]) {
		let (src_v_index, src_r_index, dest_index) = (
			(command >> 8) & 0x0F,
			(command >> 12) & 0x3F,
			(command >> 18) & 0x0F
		);
		unsafe {
			(
				slice::from_raw_parts_mut::<'static>(&mut self.regs[(src_v_index << 2) as usize] as *mut f32, 4),
				&mut *(&mut self.regs[src_r_index as usize] as *mut f32),
				slice::from_raw_parts_mut::<'static>(&mut self.regs[(dest_index << 2) as usize] as *mut f32, 4)
			)
		}
	}
	
	fn r_r_r_refs(&mut self, command: u32) -> (&'static mut f32, &'static mut f32, &'static mut f32) {
		let (src_a_index, src_b_index, dest_index) = (
			(command >> 8) & 0x3F,
			(command >> 14) & 0x3F,
			(command >> 20) & 0x3F
		);
		unsafe {
			(
				&mut *(&mut self.regs[src_a_index as usize] as *mut f32),
				&mut *(&mut self.regs[src_b_index as usize] as *mut f32),
				&mut *(&mut self.regs[dest_index as usize] as *mut f32),
			)
		}
	}
	
	fn r_r_refs(&mut self, command: u32) -> (&'static mut f32, &'static mut f32) {
		let (src_index, dest_index) = (
			(command >> 8) & 0x3F,
			(command >> 14) & 0x3F,
		);
		unsafe {
			(
				&mut *(&mut self.regs[src_index as usize] as *mut f32),
				&mut *(&mut self.regs[dest_index as usize] as *mut f32),
			)
		}
	}
	
	fn vec_vec_refs(&mut self, command: u32) -> (&'static mut [f32], &'static mut [f32]) {
		let (src_index, dest_index) = (
			(command >> 8) & 0x0F,
			(command >> 12) & 0x0F
		);
		unsafe {
			(
				slice::from_raw_parts_mut::<'static>(&mut self.regs[(src_index << 2) as usize] as *mut f32, 4),
				slice::from_raw_parts_mut::<'static>(&mut self.regs[(dest_index << 2) as usize] as *mut f32, 4),
			)
		}
	}
	
	fn do_command(&mut self, command: u32) -> bool {
		let op = command & 0xFF;
		match op {
			COMMAND_OP_VEC_VEC_ADD2_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v2(src_a);
				let v_b = read_v2(src_b);
				let result = add_v2(v_a, v_b);
				write_v2(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_ADD3_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v3(src_a);
				let v_b = read_v3(src_b);
				let result = add_v3(v_a, v_b);
				write_v3(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_ADD4_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v4(src_a);
				let v_b = read_v4(src_b);
				let result = add_v4(v_a, v_b);
				write_v4(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_SUB2_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v2(src_a);
				let v_b = read_v2(src_b);
				let result = sub_v2(v_a, v_b);
				write_v2(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_SUB3_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v3(src_a);
				let v_b = read_v3(src_b);
				let result = sub_v3(v_a, v_b);
				write_v3(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_SUB4_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v4(src_a);
				let v_b = read_v4(src_b);
				let result = sub_v4(v_a, v_b);
				write_v4(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_MUL2_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v2(src_a);
				let v_b = read_v2(src_b);
				let result = mul_v2(v_a, v_b);
				write_v2(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_MUL3_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v3(src_a);
				let v_b = read_v3(src_b);
				let result = mul_v3(v_a, v_b);
				write_v3(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_MUL4_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v4(src_a);
				let v_b = read_v4(src_b);
				let result = mul_v4(v_a, v_b);
				write_v4(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_DIV2_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v2(src_a);
				let v_b = read_v2(src_b);
				let result = div_v2(v_a, v_b);
				write_v2(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_DIV3_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v3(src_a);
				let v_b = read_v3(src_b);
				let result = div_v3(v_a, v_b);
				write_v3(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_DIV4_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v4(src_a);
				let v_b = read_v4(src_b);
				let result = div_v4(v_a, v_b);
				write_v4(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_REM2_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v2(src_a);
				let v_b = read_v2(src_b);
				let result = rem_v2(v_a, v_b);
				write_v2(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_REM3_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v3(src_a);
				let v_b = read_v3(src_b);
				let result = rem_v3(v_a, v_b);
				write_v3(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_REM4_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v4(src_a);
				let v_b = read_v4(src_b);
				let result = rem_v4(v_a, v_b);
				write_v4(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_POW2_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v2(src_a);
				let v_b = read_v2(src_b);
				let result = pow_v2(v_a, v_b);
				write_v2(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_POW3_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v3(src_a);
				let v_b = read_v3(src_b);
				let result = pow_v3(v_a, v_b);
				write_v3(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_POW4_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v4(src_a);
				let v_b = read_v4(src_b);
				let result = pow_v4(v_a, v_b);
				write_v4(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_PROJECT2_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v2(src_a);
				let v_b = read_v2(src_b);
				let scale = dot_v2(v_a, v_b) / length_sq_v2(v_a);
				let result = scale_v2(v_a, scale);
				write_v2(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_PROJECT3_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v3(src_a);
				let v_b = read_v3(src_b);
				let scale = dot_v3(v_a, v_b) / length_sq_v3(v_a);
				let result = scale_v3(v_a, scale);
				write_v3(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_PROJECT4_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v4(src_a);
				let v_b = read_v4(src_b);
				let scale = dot_v4(v_a, v_b) / length_sq_v4(v_a);
				let result = scale_v4(v_a, scale);
				write_v4(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_CROSS_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v_a = read_v3(src_a);
				let v_b = read_v3(src_b);
				let result = cross(v_a, v_b);
				write_v3(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_QROTATE_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let v = read_v3(src_a);
				let q = read_v4(src_b);
				let result = rotate_v3(v, q);
				write_v3(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_QMUL_VEC => {
				let (src_a, src_b, dest) = self.vec_vec_vec_refs(command);
				let q_a = read_v4(src_a);
				let q_b = read_v4(src_b);
				let result = mul_q(q_a, q_b);
				write_v4(dest, result);
				true
			},
			COMMAND_OP_VEC_VEC_DOT2_R => {
				let (src_a, src_b, dest) = self.vec_vec_r_refs(command);
				let v_a = read_v2(src_a);
				let v_b = read_v2(src_b);
				let result = dot_v2(v_a, v_b);
				*dest = result;
				true
			},
			COMMAND_OP_VEC_VEC_DOT3_R => {
				let (src_a, src_b, dest) = self.vec_vec_r_refs(command);
				let v_a = read_v3(src_a);
				let v_b = read_v3(src_b);
				let result = dot_v3(v_a, v_b);
				*dest = result;
				true
			},
			COMMAND_OP_VEC_VEC_DOT4_R => {
				let (src_a, src_b, dest) = self.vec_vec_r_refs(command);
				let v_a = read_v4(src_a);
				let v_b = read_v4(src_b);
				let result = dot_v4(v_a, v_b);
				*dest = result;
				true
			},
			COMMAND_OP_VEC_LENGTH2_R => {
				let (src, dest) = self.vec_r_refs(command);
				let v = read_v2(src);
				let result = length_v2(v);
				*dest = result;
				true
			},
			COMMAND_OP_VEC_LENGTH3_R => {
				let (src, dest) = self.vec_r_refs(command);
				let v = read_v3(src);
				let result = length_v3(v);
				*dest = result;
				true
			},
			COMMAND_OP_VEC_LENGTH4_R => {
				let (src, dest) = self.vec_r_refs(command);
				let v = read_v4(src);
				let result = length_v4(v);
				*dest = result;
				true
			},
			COMMAND_OP_VEC_NORM2_VEC => {
				let (src, dest) = self.vec_vec_refs(command);
				let v = read_v2(src);
				let scale = 1.0 / length_v2(v);
				let result = scale_v2(v, scale);
				write_v2(dest, result);
				true
			},
			COMMAND_OP_VEC_NORM3_VEC => {
				let (src, dest) = self.vec_vec_refs(command);
				let v = read_v3(src);
				let scale = 1.0 / length_v3(v);
				let result = scale_v3(v, scale);
				write_v3(dest, result);
				true
			},
			COMMAND_OP_VEC_NORM4_VEC => {
				let (src, dest) = self.vec_vec_refs(command);
				let v = read_v4(src);
				let scale = 1.0 / length_v4(v);
				let result = scale_v4(v, scale);
				write_v4(dest, result);
				true
			},
			COMMAND_OP_VEC_R_SCALE2_VEC => {
				let (src_v, src_r, dest) = self.vec_r_vec_refs(command);
				let v = read_v2(src_v);
				let result = scale_v2(v, *src_r);
				write_v2(dest, result);
				true
			},
			COMMAND_OP_VEC_R_SCALE3_VEC => {
				let (src_v, src_r, dest) = self.vec_r_vec_refs(command);
				let v = read_v3(src_v);
				let result = scale_v3(v, *src_r);
				write_v3(dest, result);
				true
			},
			COMMAND_OP_VEC_R_SCALE4_VEC => {
				let (src_v, src_r, dest) = self.vec_r_vec_refs(command);
				let v = read_v4(src_v);
				let result = scale_v4(v, *src_r);
				write_v4(dest, result);
				true
			},
			COMMAND_OP_VEC_R_ANGLEAXISQUAT_VEC => {
				let (src_v, src_r, dest) = self.vec_r_vec_refs(command);
				let v = read_v3(src_v);
				let result = q_from_angle_axis(*src_r, v);
				write_v4(dest, result);
				true
			},
			COMMAND_OP_VEC_R_ROTATE_VEC => {
				let (src_v, src_r, dest) = self.vec_r_vec_refs(command);
				let v = read_v2(src_v);
				let result = rotate_v2(v, *src_r);
				write_v2(dest, result);
				true
			}
			COMMAND_OP_R_R_ADD_R => {
				let (src_a, src_b, dest) = self.r_r_r_refs(command);
				let result = *src_a + *src_b;
				*dest = result;
				true
			},
			COMMAND_OP_R_R_SUB_R => {
				let (src_a, src_b, dest) = self.r_r_r_refs(command);
				let result = *src_a - *src_b;
				*dest = result;
				true
			},
			COMMAND_OP_R_R_MUL_R => {
				let (src_a, src_b, dest) = self.r_r_r_refs(command);
				let result = *src_a * *src_b;
				*dest = result;
				true
			},
			COMMAND_OP_R_R_DIV_R => {
				let (src_a, src_b, dest) = self.r_r_r_refs(command);
				let result = *src_a / *src_b;
				*dest = result;
				true
			},
			COMMAND_OP_R_R_REM_R => {
				let (src_a, src_b, dest) = self.r_r_r_refs(command);
				let result = *src_a % *src_b;
				*dest = result;
				true
			},
			COMMAND_OP_R_R_POW_R => {
				let (src_a, src_b, dest) = self.r_r_r_refs(command);
				let result = (*src_a).pow(*src_b);
				*dest = result;
				true
			},
			COMMAND_OP_R_R_ATAN2_R => {
				let (src_a, src_b, dest) = self.r_r_r_refs(command);
				let result = f32::atan2(*src_a, *src_b);
				*dest = result;
				true
			},
			COMMAND_OP_R_R_LOG_R => {
				let (src_a, src_b, dest) = self.r_r_r_refs(command);
				let result = src_a.log(*src_b);
				*dest = result;
				true
			},
			COMMAND_OP_R_SIN_R => {
				let (src, dest) = self.r_r_refs(command);
				let result = src.sin();
				*dest = result;
				true
			},
			COMMAND_OP_R_COS_R => {
				let (src, dest) = self.r_r_refs(command);
				let result = src.cos();
				*dest = result;
				true
			},
			COMMAND_OP_R_TAN_R => {
				let (src, dest) = self.r_r_refs(command);
				let result = src.tan();
				*dest = result;
				true
			},
			COMMAND_OP_R_ARCCOS_R => {
				let (src, dest) = self.r_r_refs(command);
				let result = src.acos();
				*dest = result;
				true
			},
			COMMAND_OP_R_ARCSIN_R => {
				let (src, dest) = self.r_r_refs(command);
				let result = src.asin();
				*dest = result;
				true
			},
			COMMAND_OP_R_ARCTAN_R => {
				let (src, dest) = self.r_r_refs(command);
				let result = src.atan();
				*dest = result;
				true
			},
			COMMAND_OP_R_EXP_R => {
				let (src, dest) = self.r_r_refs(command);
				let result = src.exp();
				*dest = result;
				true
			},
			COMMAND_OP_R_LN_R => {
				let (src, dest) = self.r_r_refs(command);
				let result = src.log(std::f32::consts::E);
				*dest = result;
				true
			},
			COMMAND_OP_R_INV_R => {
				let (src, dest) = self.r_r_refs(command);
				let result = src.exp();
				*dest = result;
				true
			},
			_ => {
				self.error = ERROR_UNKNOWN_OP;
				false
			}
		}
	}
}

impl MathAccelerator {
	pub fn new() -> Self {
		Self {
			data: Mutex::new(MathAcceleratorData::new())
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		self.data.lock().read_32(offset)
	}
	
	pub fn write_32(&self, mio: &mut FmMemoryIO, offset: u32, data: u32) -> MemWriteResult {
		self.data.lock().write_32(mio, offset, data)
	}
}