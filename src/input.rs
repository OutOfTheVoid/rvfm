use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

use crate::fm_mio::FmMemoryIO;
use rv_vsys::{MemReadResult, MemWriteResult};
use winit::event::VirtualKeyCode;

#[derive(Debug)]
pub struct InputPeripheral {
	key_change_events_0_31: Arc<AtomicU32>,
	key_change_events_32_63: Arc<AtomicU32>,
	key_change_events_64_95: Arc<AtomicU32>,
	key_states_0_31: Arc<AtomicU32>,
	key_states_32_63: Arc<AtomicU32>,
	key_states_64_95: Arc<AtomicU32>,
}

pub struct InputEventSink {
	key_change_events_0_31: Arc<AtomicU32>,
	key_change_events_32_63: Arc<AtomicU32>,
	key_change_events_64_95: Arc<AtomicU32>,
	key_states_0_31: Arc<AtomicU32>,
	key_states_32_63: Arc<AtomicU32>,
	key_states_64_95: Arc<AtomicU32>,
}

fn map_vkey(vkey: VirtualKeyCode) -> Option<u32> {
	match vkey {
		VirtualKeyCode::Escape => Some(0),
		VirtualKeyCode::Back => Some(1),
		VirtualKeyCode::Return => Some(3),
		VirtualKeyCode::Up => Some(4),
		VirtualKeyCode::Down => Some(5),
		VirtualKeyCode::Left => Some(6),
		VirtualKeyCode::Right => Some(7),
		VirtualKeyCode::Delete => Some(8),
		VirtualKeyCode::Tab => Some(9),
		VirtualKeyCode::Space => Some(10),
		VirtualKeyCode::Apostrophe => Some(11),
		VirtualKeyCode::Semicolon => Some(12),
		VirtualKeyCode::LBracket => Some(13),
		VirtualKeyCode::RBracket => Some(14),
		VirtualKeyCode::Backslash => Some(15),
		VirtualKeyCode::Minus => Some(16),
		VirtualKeyCode::Equals => Some(17),
		VirtualKeyCode::Slash => Some(17),
		
		VirtualKeyCode::F1 => Some(18),
		VirtualKeyCode::F2 => Some(19),
		VirtualKeyCode::F3 => Some(20),
		VirtualKeyCode::F4 => Some(21),
		VirtualKeyCode::F5 => Some(22),
		VirtualKeyCode::F6 => Some(23),
		VirtualKeyCode::F7 => Some(24),
		VirtualKeyCode::F8 => Some(25),
		VirtualKeyCode::F9 => Some(26),
		VirtualKeyCode::F10 => Some(28),
		VirtualKeyCode::F11 => Some(29),
		VirtualKeyCode::F12 => Some(30),
		
		VirtualKeyCode::A => Some(31),
		VirtualKeyCode::B => Some(32),
		VirtualKeyCode::C => Some(33),
		VirtualKeyCode::D => Some(34),
		VirtualKeyCode::E => Some(35),
		VirtualKeyCode::F => Some(36),
		VirtualKeyCode::G => Some(37),
		VirtualKeyCode::H => Some(38),
		VirtualKeyCode::I => Some(39),
		VirtualKeyCode::J => Some(40),
		VirtualKeyCode::K => Some(41),
		VirtualKeyCode::L => Some(42),
		VirtualKeyCode::M => Some(43),
		VirtualKeyCode::N => Some(44),
		VirtualKeyCode::O => Some(45),
		VirtualKeyCode::P => Some(46),
		VirtualKeyCode::Q => Some(47),
		VirtualKeyCode::R => Some(48),
		VirtualKeyCode::S => Some(49),
		VirtualKeyCode::T => Some(50),
		VirtualKeyCode::U => Some(51),
		VirtualKeyCode::V => Some(52),
		VirtualKeyCode::W => Some(53),
		VirtualKeyCode::X => Some(54),
		VirtualKeyCode::Y => Some(55),
		VirtualKeyCode::Z => Some(56),
		
		VirtualKeyCode::Key0 => Some(56),
		VirtualKeyCode::Key1 => Some(57),
		VirtualKeyCode::Key2 => Some(58),
		VirtualKeyCode::Key3 => Some(59),
		VirtualKeyCode::Key4 => Some(60),
		VirtualKeyCode::Key5 => Some(61),
		VirtualKeyCode::Key6 => Some(62),
		VirtualKeyCode::Key7 => Some(63),
		VirtualKeyCode::Key8 => Some(64),
		VirtualKeyCode::Key9 => Some(65),
		
		_ => None
	}
}

impl InputEventSink {
	pub fn vkey_event(&mut self, vkey: VirtualKeyCode, down: bool) {
		let key_index = map_vkey(vkey);
		if let Some(key_index) = key_index {
			if down {
				match key_index {
					0 ..= 31 => {
						self.key_states_0_31.fetch_or(1 << key_index, std::sync::atomic::Ordering::SeqCst);
						self.key_change_events_0_31.fetch_or(1 << key_index, std::sync::atomic::Ordering::SeqCst);
					},
					32 ..= 63 => {
						self.key_states_32_63.fetch_or(1 << (key_index - 32), std::sync::atomic::Ordering::SeqCst);
						self.key_change_events_32_63.fetch_or(1 << (key_index - 32), std::sync::atomic::Ordering::SeqCst);
					},
					64 ..= 95 => {
						self.key_states_64_95.fetch_or(1 << (key_index - 64), std::sync::atomic::Ordering::SeqCst);
						self.key_change_events_64_95.fetch_or(1 << (key_index - 64), std::sync::atomic::Ordering::SeqCst);
					},
					_ => panic!("unknown key index")
				}
			} else {
				match key_index {
					0 ..= 31 => {
						self.key_states_0_31.fetch_and(! (1 << key_index), std::sync::atomic::Ordering::SeqCst);
						self.key_change_events_0_31.fetch_or(1 << key_index, std::sync::atomic::Ordering::SeqCst);
					},
					32 ..= 63 => {
						self.key_states_32_63.fetch_and(! (1 << (key_index - 32)), std::sync::atomic::Ordering::SeqCst);
						self.key_change_events_32_63.fetch_or(1 << (key_index - 32), std::sync::atomic::Ordering::SeqCst);
					},
					64 ..= 95 => {
						self.key_states_64_95.fetch_and(! (1 << (key_index - 64)), std::sync::atomic::Ordering::SeqCst);
						self.key_change_events_64_95.fetch_or(1 << (key_index - 64), std::sync::atomic::Ordering::SeqCst);
					},
					_ => panic!("unknown key index")
				}
			}
		}
	}
}

const REG_KEY_EVENTS_0_31: u32 = 0;
const REG_KEY_EVENTS_32_63: u32 = 4;
const REG_KEY_EVENTS_64_95: u32 = 8;
const REG_KEY_STATES_0_31: u32 = 12;
const REG_KEY_STATES_32_63: u32 = 16;
const REG_KEY_STATES_64_95: u32 = 20;
const REG_CLEAR_KEY_EVENTS: u32 = 24;

impl InputPeripheral {
	pub fn new(mio: &mut FmMemoryIO) -> InputEventSink {
		let key_change_events_0_31 = Arc::new(AtomicU32::new(0));
		let key_change_events_32_63 = Arc::new(AtomicU32::new(0));
		let key_change_events_64_95 = Arc::new(AtomicU32::new(0));
		let key_states_0_31 = Arc::new(AtomicU32::new(0));
		let key_states_32_63 = Arc::new(AtomicU32::new(0));
		let key_states_64_95 = Arc::new(AtomicU32::new(0));
		let peripheral = InputPeripheral {
			key_change_events_0_31: key_change_events_0_31.clone(),
			key_change_events_32_63: key_change_events_32_63.clone(),
			key_change_events_64_95: key_change_events_64_95.clone(),
			key_states_0_31: key_states_0_31.clone(),
			key_states_32_63: key_states_32_63.clone(),
			key_states_64_95: key_states_64_95.clone(),
		};
		mio.set_input(peripheral);
		InputEventSink {
			key_change_events_0_31,
			key_change_events_32_63,
			key_change_events_64_95,
			key_states_0_31,
			key_states_32_63,
			key_states_64_95,
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		match offset {
			REG_KEY_EVENTS_0_31 => MemReadResult::Ok(self.key_change_events_0_31.load(Ordering::SeqCst)),
			REG_KEY_EVENTS_32_63 => MemReadResult::Ok(self.key_change_events_32_63.load(Ordering::SeqCst)),
			REG_KEY_EVENTS_64_95 => MemReadResult::Ok(self.key_change_events_64_95.load(Ordering::SeqCst)),
			REG_KEY_STATES_0_31 => MemReadResult::Ok(self.key_states_0_31.load(Ordering::SeqCst)),
			REG_KEY_STATES_32_63 => MemReadResult::Ok(self.key_states_32_63.load(Ordering::SeqCst)),
			REG_KEY_STATES_64_95 => MemReadResult::Ok(self.key_states_64_95.load(Ordering::SeqCst)),
			_ => MemReadResult::Ok(0),
		}
	}
	
	pub fn write_32(&self, offset: u32, value: u32) -> MemWriteResult {
		match offset {
			REG_KEY_EVENTS_0_31 => {
				self.key_change_events_0_31.fetch_and(! value, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			REG_KEY_EVENTS_32_63 => {
				self.key_change_events_32_63.fetch_and(! value, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			REG_KEY_EVENTS_64_95 => {
				self.key_change_events_64_95.fetch_and(! value, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			REG_CLEAR_KEY_EVENTS => {
				self.key_change_events_0_31.store(0, Ordering::SeqCst);
				self.key_change_events_32_63.store(0, Ordering::SeqCst);
				self.key_change_events_64_95.store(0, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			_ => MemWriteResult::ErrReadOnly
		}
	}
}
