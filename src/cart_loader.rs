use std::{io, path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicBool, AtomicU32, Ordering}}};
use bytemuck::offset_of;
use json::JsonValue;
use parking_lot::{Condvar, Mutex};
use regex::Regex;
use rv_vsys::{Cpu, CpuKillHandle, MemIO, MemReadResult, MemWriteResult};
use std::sync::mpsc;

use crate::{fm_mio::FmMemoryIO, fm_interrupt_bus::FmInterruptBus, mtimer::MTimerPeripheral};

enum CartData {
	None,
	FsR(PathBuf),
	FsRW(PathBuf),
	BinaryR(PathBuf),
	BinaryRW(PathBuf),
}

struct Cart {
	name: String,
	version: (u32, u32, u32),
	binary: PathBuf,
	data: CartData,
	developer: String,
	developer_url: String,
	icon: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct CartLoaderWaitState {
	pub wait: bool,
	pub start_pc: u32
}

#[derive(Clone, Debug)]
pub struct CartLoaderCpuBarrier {
	wait_lock: Arc<Mutex<CartLoaderWaitState>>,
	wait_cond: Arc<Condvar>,
}

impl CartLoaderCpuBarrier {
	pub fn wait_barrier(&self) -> u32 {
		let mut gaurd = self.wait_lock.lock();
		while gaurd.wait {
			self.wait_cond.wait(&mut gaurd);
		}
		gaurd.start_pc
	}
}

#[derive(Clone, Debug)]
enum CartLoaderCmd {
	EnumerateCarts(u32),
	ReadCartMetadata{index: u32, data_write_addr: u32},
	LoadCart{index: u32, error_write_addr: u32},
}

pub struct CartLoader {
	library_dir: PathBuf,
	mio: FmMemoryIO,
	wait_lock: Arc<Mutex<CartLoaderWaitState>>,
	wait_cond: Arc<Condvar>,
	cpu0_kill: CpuKillHandle,
	cpu1_kill: CpuKillHandle,
	command_channel: mpsc::Receiver<CartLoaderCmd>,
	carts: Vec<Cart>,
	cart_count: Arc<AtomicU32>,
}

const COMPLETION_RESULT_NONE: u32 = 0;
const COMPLETION_RESULT_OK: u32 = 1;
const COMPLETION_RESULT_ERROR_READING_DIR: u32 = 1;

impl CartLoader {
	pub fn start(mut mio: FmMemoryIO, cpu0: &Cpu<MTimerPeripheral, FmMemoryIO, FmInterruptBus>, cpu1: &Cpu<MTimerPeripheral, FmMemoryIO, FmInterruptBus>) -> CartLoaderCpuBarrier {
		let (cmd_tx, cmd_rx) = mpsc::channel();
		let cart_count = Arc::new(AtomicU32::new(0));
		let peripheral = CartLoaderPeripheral {
			cmd_tx,
			cart_count: cart_count.clone(),
			param0: Arc::new(AtomicU32::new(0)),
			param1: Arc::new(AtomicU32::new(0)),
			param2: Arc::new(AtomicU32::new(0)),
			param3: Arc::new(AtomicU32::new(0)),
		};
		mio.set_cart_loader(peripheral);
		let loader = Self {
			library_dir: std::env::current_dir().unwrap(),
			mio,
			wait_lock: Arc::new(Mutex::new(CartLoaderWaitState {
				wait: false,
				start_pc: 0
			})),
			wait_cond: Arc::new(Condvar::new()),
			cpu0_kill: cpu0.get_kill_handle(),
			cpu1_kill: cpu1.get_kill_handle(),
			command_channel: cmd_rx,
			carts: Vec::new(),
			cart_count
		};
		let barrier = loader.make_cpu_barrier();
		std::thread::spawn(move || {
			loader.run();
		});
		barrier
	}
	
	pub fn make_cpu_barrier(&self) -> CartLoaderCpuBarrier {
		CartLoaderCpuBarrier {
			wait_lock: self.wait_lock.clone(),
			wait_cond: self.wait_cond.clone()
		}
	}
	
	fn load_cart(&mut self, cart_index: u32, error_write_addr: u32) {
		if cart_index >= self.carts.len() as u32 {
			self.mio.write_32(error_write_addr, 1); // ignore memory failure. this is how we signal an error
			self.mio.access_break();
		} else {
			let cart = &self.carts[cart_index as usize];
			{
				let mut wait_gaurd = self.wait_lock.lock();
				wait_gaurd.wait = true;
				self.cpu1_kill.kill();
				self.cpu0_kill.kill();
			}
			// actually load cart
			// todo...
			let start_pc = 0;
			{
				let mut wait_gaurd = self.wait_lock.lock();
				wait_gaurd.start_pc = start_pc;
				wait_gaurd.wait = false;
			}
			self.wait_cond.notify_all();
		}
	}
	
	fn enumerate_carts(&mut self, completion_signal_addr: u32) {
		self.mio.write_32(completion_signal_addr, COMPLETION_RESULT_NONE);
		self.mio.access_break();
		let cart_paths = match std::fs::read_dir(self.library_dir.as_path()) {
			Ok(x) => {
				x.filter_map(|entry| {
					match entry {
						Ok(entry) => {
							Some(entry.path())
						},
						Err(..) => None
					}
				})
			},
			Err(..) => {
				self.mio.write_32(completion_signal_addr, COMPLETION_RESULT_ERROR_READING_DIR);
				self.mio.access_break();
				return;
			}
		};
		let mut carts = Vec::new();
		for cart_path in cart_paths {
			let cart_path_str = cart_path.to_string_lossy().to_string();
			let mut cart_info_path = cart_path.clone();
			cart_info_path.push("cart.json");
			if let Ok(cart_info_json) = std::fs::read_to_string(cart_info_path) {
				if let Ok(info) = json::parse(cart_info_json.as_str()) {
					match info {
						json::JsonValue::Object(info_fields) => {
							let name = match info_fields.get("name") {
								Some(json::JsonValue::String(name)) => name.clone(),
								_ => {
									println!("CartLoader warning: Cart at {} has no name.", cart_path_str);
									"untitled ".to_string() + format!("{}", carts.len()).as_str()
								}
							};
							let version = match info_fields.get("version") {
								Some(json::JsonValue::String(version)) => {
									let re = Regex::new("(\\d+).(\\d+).(\\d)").unwrap();
									re.captures(version.as_str()).map_or_else(|| {
										println!("CartLoader warning: Cart at {} has invalid version string: \"{}\".", cart_path_str, version);
										(0, 0, 0)
									}, |captures| {
										(captures[1].parse().unwrap(), captures[2].parse().unwrap(), captures[3].parse().unwrap())
									})
								},
								_ => {
									println!("CartLoader warning: Cart at {} has no version!", cart_path_str);
									(0, 0, 0)
								}
							};
							let developer = match info_fields.get("developer") {
								Some(json::JsonValue::String(developer)) => developer.clone(),
								_ => String::from("")
							};
							let developer_url = match info_fields.get("developer_url") {
								Some(json::JsonValue::String(developer_url)) => developer_url.clone(),
								_ => String::from("")
							};
							let icon = match info_fields.get("icon") {
								Some(json::JsonValue::String(icon)) => Some(PathBuf::from(icon)),
								_ => None
							};
							let data = match info_fields.get("data") {
								Some(json::JsonValue::Object(data_fields)) => {
									if let Some(json::JsonValue::String(format)) = data_fields.get("format") {
										match format.as_str() {
											"none" => CartData::None,
											"fs-ro" => {
												if let Some(json::JsonValue::String(root_dir_name)) = data_fields.get("root_dir") {
													let mut root_dir = cart_path.clone();
													root_dir.push(root_dir_name);
													if ! root_dir.exists() {
														println!("CartLoader warning: Cart at {} data root_dir does not exist!", cart_path_str);
														CartData::None
													} else {
														if ! root_dir.metadata().unwrap().is_dir() {
															println!("CartLoader warning: Cart at {} data root_dir {} is not a directory!", cart_path_str, root_dir.to_string_lossy());
															CartData::None
														} else {
															CartData::FsR(PathBuf::from(root_dir))
														}
													}
												} else {
													println!("CartLoader warning: Cart at {} specifies fs-ro data but has no root_dir!", cart_path_str);
													CartData::None
												}
											},
											"fs-rw" => {
												if let Some(json::JsonValue::String(root_dir_name)) = data_fields.get("root_dir") {
													let mut root_dir = cart_path;
													root_dir.push(root_dir_name);
													if ! root_dir.exists() {
														println!("CartLoader warning: Cart at {} data root_dir does not exist!", cart_path_str);
														CartData::None
													} else {
														if ! root_dir.metadata().unwrap().is_dir() {
															println!("CartLoader warning: Cart at {} data root_dir {} is not a directory!", cart_path_str, root_dir.to_string_lossy());
															CartData::None
														} else {
															CartData::FsRW(PathBuf::from(root_dir))
														}
													}
												} else {
													println!("CartLoader warning: Cart at {} does not define a root_dir field!", cart_path_str);
													CartData::None
												}
											},
											"binary-ro" => {
												if let Some(json::JsonValue::String(data_file_name)) = data_fields.get("data_file") {
													let mut data_file = cart_path;
													data_file.push(data_file_name);
													if ! data_file.exists() {
														println!("CartLoader warning: Cart at {} data data_file does not exist!", cart_path_str);
														CartData::None
													} else {
														if ! data_file.metadata().unwrap().is_file() {
															println!("CartLoader warning: Cart at {} data data_file {} is not a file!", cart_path_str, data_file.to_string_lossy());
															CartData::None
														} else {
															CartData::BinaryR(PathBuf::from(data_file))
														}
													}
												} else {
													println!("CartLoader warning: Cart at {} does not define a data_file field!", cart_path_str);
													CartData::None
												}
											},
											"binary-rw" => {
												if let Some(json::JsonValue::String(data_file_name)) = data_fields.get("data_file") {
													let mut data_file = cart_path;
													data_file.push(data_file_name);
													if ! data_file.exists() {
														println!("CartLoader warning: Cart at {} data data_file does not exist!", cart_path_str);
														CartData::None
													} else {
														if ! data_file.metadata().unwrap().is_file() {
															println!("CartLoader warning: Cart at {} data data_file {} is not a file!", cart_path_str, data_file.to_string_lossy());
															CartData::None
														} else {
															CartData::BinaryRW(PathBuf::from(data_file))
														}
													}
												} else {
													println!("CartLoader warning: Cart at {} does not define a data_file field!", cart_path_str);
													CartData::None
												}
											},
											other => {
												println!("CartLoader warning: Cart at {} data format unknown: {}!", cart_path_str, other);
												CartData::None
											}
										}
									} else {
										println!("CartLoader warning: Cart at {} has no data definition, or is invalid!", cart_path_str);
										CartData::None
									}
								},
								_ => CartData::None
							};
							if let Some(json::JsonValue::String(binary_path)) = info_fields.get("binary") {
								let binary = PathBuf::from(binary_path);
								carts.push(Cart {
									name,
									version,
									binary,
									data,
									developer,
									developer_url,
									icon
								});
							} else {
								println!("CartLoader warning: Cart at {} has no binary!", cart_path_str);
							}
						},
						_ => {
							println!("CartLoader warning: Cart at {} has invalid file format.", cart_path_str);
						}
					}
				}
			}
		}
		self.mio.write_32(completion_signal_addr, COMPLETION_RESULT_OK);
		self.mio.access_break();
	}
	
	fn read_cart_metadata(&mut self, index: u32, data_write_addr: u32) {
		
	}
	
	pub fn run(mut self) {
		loop {
			match self.command_channel.recv() {
				Ok(command) => {
					match command {
						CartLoaderCmd::EnumerateCarts(completion_signal_addr) => {
							self.enumerate_carts(completion_signal_addr)
						},
						CartLoaderCmd::ReadCartMetadata {
							index, 
							data_write_addr
						} => {
							self.read_cart_metadata(index, data_write_addr);
						},
						CartLoaderCmd::LoadCart {
							index, 
							error_write_addr
						} => {
							self.load_cart(index, error_write_addr);
						}
					}
				},
				Err(..) => {
					panic!("Cart loader thread receive error");
				}
			}
		}
	}
}

#[derive(Debug, Clone)]
pub struct CartLoaderPeripheral {
	cmd_tx: mpsc::Sender<CartLoaderCmd>,
	cart_count: Arc<AtomicU32>,
	param0: Arc<AtomicU32>,
	param1: Arc<AtomicU32>,
	param2: Arc<AtomicU32>,
	param3: Arc<AtomicU32>,
}

const REG_COMMAND: u32 = 0;
const REG_PARAM0: u32 = 4;
const REG_PARAM1: u32 = 8;
const REG_PARAM2: u32 = 12;
const REG_PARAM3: u32 = 16;
const REG_CART_COUNT: u32 = 20;

const COMMAND_ENUMERATE_CARTS: u32 = 0;
const COMMAND_READ_CART_METADATA: u32 = 1;
const COMMAND_LOAD_CART: u32 = 2;

impl CartLoaderPeripheral {
	pub fn write_32(&self, offset: u32, value: u32) -> MemWriteResult {
		if (offset & 0x03) != 0 {
			return MemWriteResult::ErrAlignment;
		}
		match offset {
			REG_COMMAND => {
				if self.command(value) {
					MemWriteResult::Ok
				} else {
					MemWriteResult::PeripheralError
				}
			},
			REG_PARAM0 => {
				self.param0.store(value, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			REG_PARAM1 => {
				self.param1.store(value, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			REG_PARAM2 => {
				self.param2.store(value, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			REG_PARAM3 => {
				self.param3.store(value, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			_ => {
				MemWriteResult::PeripheralError
			}
		}
	}
	
	fn command(&self, command: u32) -> bool {
		match command {
			COMMAND_ENUMERATE_CARTS => {
				let completion_addr = self.param0.load(Ordering::SeqCst);
				self.cmd_tx.send(CartLoaderCmd::EnumerateCarts(completion_addr)).unwrap();
				true
			},
			COMMAND_LOAD_CART => {
				let index: u32 = self.param0.load(Ordering::SeqCst);
				let error_write_addr: u32 = self.param1.load(Ordering::SeqCst);
				self.cmd_tx.send(CartLoaderCmd::LoadCart {
					index,
					error_write_addr
				}).unwrap();
				true
			},
			_ => false,
		}
	}
	
	pub fn read_32(&self, offset: u32) -> MemReadResult<u32> {
		if (offset & 0x03) != 0 {
			return MemReadResult::ErrAlignment;
		}
		match offset {
			REG_CART_COUNT => {
				MemReadResult::Ok(self.cart_count.load(Ordering::SeqCst))
			},
			_ => MemReadResult::Ok(0)
		}
	}
}

unsafe impl Send for CartLoaderPeripheral {}
unsafe impl Sync for CartLoaderPeripheral {}
