use std::{fs::File, os::unix::prelude::FileExt, path::{PathBuf}, sync::{Arc, atomic::{AtomicU32, Ordering}}};
use image::EncodableLayout;
use parking_lot::{Condvar, Mutex};
use regex::Regex;
use rv_vsys::{Cpu, CpuKillHandle, MemIO, MemReadResult, MemWriteResult};
use std::sync::mpsc;

use crate::{elf_loader, fm_interrupt_bus::FmInterruptBus, fm_mio::FmMemoryIO, gpu::GpuResetHandle, mtimer::MTimerPeripheral};

#[derive(Debug, Clone)]
enum CartData {
	None,
	FsR(PathBuf),
	FsRW(PathBuf),
	BinaryR(PathBuf),
	BinaryRW(PathBuf),
}

#[derive(Debug, Clone)]
struct Cart {
	pub path: PathBuf,
	pub name: String,
	pub version: (u32, u32, u32),
	pub binary: PathBuf,
	pub data: CartData,
	pub developer: String,
	pub developer_url: String,
	pub source: String,
	pub icon: Option<PathBuf>,
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
	ReadCartMetadata{index: u32, metadata_struct_addr: u32, completion_addr: u32},
	LoadCart{index: u32, error_write_addr: u32},
	SetupDataAccessFs{slot: u32, file_name: PathBuf, completion_addr: u32, flags: u32},
	SetupDataAccessBinary{slot: u32, completion_addr: u32, offset: u32, length: u32},
	CloseDataAccess{slot: u32, completion_addr: u32},
	ReadData{slot: u32, offset: u32, length: u32, buffer_addr: u32, read_size_addr: u32, completion_addr: u32},
	GetDataExtents{slot: u32, extents_addr: u32, completion_addr: u32},
}

enum DataAccessSlot {
	FileAccess {
		file_name: std::path::PathBuf,
		file: std::fs::File,
		write: bool,
	},
	BinaryAccess {
		binary_file_name: Arc<std::path::PathBuf>,
		binary_file: Arc<std::fs::File>,
		write: bool,
		offset: u32,
		length: Option<u32>
	},
	None
}

const DATA_SLOT_COUNT: u32 = 8;

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
	gpu_reset_handle: GpuResetHandle,
	
	current_cart: Option<Cart>,
	data_access_slots: Box<[DataAccessSlot]>,
	binary_file_name: Option<Arc<std::path::PathBuf>>,
	binary_file: Option<Arc<std::fs::File>>,
}

const COMPLETION_RESULT_NONE: u32 = 0;
const COMPLETION_RESULT_OK: u32 = 1;
const COMPLETION_RESULT_ERROR_READING_DIR: u32 = 2;
const COMPLETION_RESULT_CART_INDEX_OUT_OF_BOUNDS: u32 = 3;
const COMPLETION_RESULT_FAILED_READING_BINARY: u32 = 4;
const COMPLETION_RESULT_DATA_SLOT_INDEX_OUT_OF_BOUNDS: u32 = 5;
const COMPLETION_RESULT_NO_CART_LOADED: u32 = 6;
const COMPLETION_RESULT_FAILED_OPENING_FILE: u32 = 7;
const COMPLETION_RESULT_BAD_OPERATION_FOR_DATA_FORMAT: u32 = 8;
const COMPLETION_RESULT_FILENAME_READ_ERROR: u32 = 9;
const COMPLETION_RESULT_DATA_SLOT_NOT_OPEN: u32 = 10;
const COMPLETION_RESULT_FAILED_READING_FILE: u32 = 11;

fn get_json_string(value: Option<&json::JsonValue>) -> Option<String> {
	match value {
		Some(json::JsonValue::String(string)) => Some(string.clone()),
		Some(json::JsonValue::Short(short)) => Some(short.to_string()),
		_ => None,
	}
}

impl CartLoader {
	pub fn start(mut mio: FmMemoryIO, cpu0: &Cpu<MTimerPeripheral, FmMemoryIO, FmInterruptBus>, cpu1: &Cpu<MTimerPeripheral, FmMemoryIO, FmInterruptBus>, gpu_reset_handle: GpuResetHandle) -> CartLoaderCpuBarrier {
		let (cmd_tx, cmd_rx) = mpsc::channel();
		let cart_count = Arc::new(AtomicU32::new(0));
		let peripheral = CartLoaderPeripheral {
			cmd_tx,
			cart_count: cart_count.clone(),
			param0: Arc::new(AtomicU32::new(0)),
			param1: Arc::new(AtomicU32::new(0)),
			param2: Arc::new(AtomicU32::new(0)),
			param3: Arc::new(AtomicU32::new(0)),
			param4: Arc::new(AtomicU32::new(0)),
			param5: Arc::new(AtomicU32::new(0)),
		};
		mio.set_cart_loader(peripheral);
		let mut cart_dir = std::env::current_dir().unwrap();
		cart_dir.push("test");
		cart_dir.push("cart_test");
		let mut data_access_slots = Vec::new();
		for _ in 0 .. DATA_SLOT_COUNT {
			data_access_slots.push(DataAccessSlot::None);
		}
		let loader = Self {
			library_dir: cart_dir,
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
			cart_count,
			gpu_reset_handle,
			
			current_cart: None,
			data_access_slots: data_access_slots.into_boxed_slice(),
			binary_file_name: None,
			binary_file: None,
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
			self.mio.write_32(error_write_addr, COMPLETION_RESULT_CART_INDEX_OUT_OF_BOUNDS);
			self.mio.access_break();
		} else {
			let cart = &self.carts[cart_index as usize];
			let mut binary_path = cart.path.clone();
			binary_path.push(&cart.binary);
			println!("Loading cart binary: {}", binary_path.to_str().unwrap());
			let elf_bytes = match std::fs::read(&binary_path) {
				Ok(elf_bytes) => elf_bytes,
				Err(..) => {
					self.mio.write_32(error_write_addr, COMPLETION_RESULT_FAILED_READING_BINARY);
					self.mio.access_break();
					return;
				}
			};
			{
				let mut wait_gaurd = self.wait_lock.lock();
				wait_gaurd.wait = true;
				self.cpu1_kill.kill();
				self.cpu0_kill.kill();
			}
			self.gpu_reset_handle.reset_gpu().wait();
			let start_pc = elf_loader::load_elf(elf_bytes.as_bytes(), &mut self.mio, 0x0000_0000).unwrap();
			{
				let mut wait_gaurd = self.wait_lock.lock();
				wait_gaurd.start_pc = start_pc;
				wait_gaurd.wait = false;
			}
			self.binary_file = None;
			self.binary_file_name = None;
			self.current_cart = Some(cart.clone());
			self.wait_cond.notify_all();
		}
	}
	
	fn enumerate_carts(&mut self, completion_signal_addr: u32) {
		self.mio.write_32(completion_signal_addr, COMPLETION_RESULT_NONE);
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
							let name = get_json_string(info_fields.get("name")).unwrap_or_else(|| {
								println!("CartLoader warning: Cart at {} has no name.", cart_path_str);
									"untitled ".to_string() + format!("{}", carts.len()).as_str()
							});
							let version_string = get_json_string(info_fields.get("version")).unwrap_or_else(|| {
								println!("CartLoader warning: Cart at {} has no version!", cart_path_str);
									"(0, 0, 0)".to_string()
							});
							let version =  {
								let re = Regex::new("(\\d+).(\\d+).(\\d)").unwrap();
								re.captures(version_string.as_str()).map_or_else(|| {
									println!("CartLoader warning: Cart at {} has invalid version string: \"{}\".", cart_path_str, version_string);
									(0, 0, 0)
								}, |captures| {
									(captures[1].parse().unwrap(), captures[2].parse().unwrap(), captures[3].parse().unwrap())
								})
							};
							let developer = get_json_string(info_fields.get("developer")).unwrap_or(String::from(""));
							let developer_url = get_json_string(info_fields.get("developer_url")).unwrap_or(String::from(""));
							let source = get_json_string(info_fields.get("source")).unwrap_or(String::from(""));
							let icon = match info_fields.get("icon") {
								Some(json::JsonValue::String(icon)) => {
									if icon.len() > 0 {
										Some(PathBuf::from(icon))
									} else {
										None
									}
								},
								Some(json::JsonValue::Short(icon)) => {
									if icon.as_str().len() > 0 {
										Some(PathBuf::from(icon.as_str()))
									} else {
										None
									}
								},
								_ => None
							};
							let data = match info_fields.get("data") {
								Some(json::JsonValue::Object(data_fields)) => {
									let format_str = get_json_string(data_fields.get("format"));
									if let Some(format) = format_str {
										match format.as_str() {
											"none" => CartData::None,
											"fs-ro" => {
												if let Some(root_dir_name) = get_json_string(data_fields.get("root_dir")) {
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
												if let Some(root_dir_name) = get_json_string(data_fields.get("root_dir")) {
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
															CartData::FsRW(PathBuf::from(root_dir))
														}
													}
												} else {
													println!("CartLoader warning: Cart at {} does not define a root_dir field!", cart_path_str);
													CartData::None
												}
											},
											"binary-ro" => {
												if let Some(data_file_name) = get_json_string(data_fields.get("data_file")) {
													let mut data_file = cart_path.clone();
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
												if let Some(data_file_name) = get_json_string(data_fields.get("data_file")) {
													let mut data_file = cart_path.clone();
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
							let binary_path_string = get_json_string(info_fields.get("binary"));
							if let Some(binary_path) = binary_path_string {
								let binary = PathBuf::from(binary_path);
								let new_cart = Cart {
									path: cart_path.clone(),
									name,
									version,
									binary,
									data,
									developer,
									developer_url,
									source,
									icon
								};
								carts.push(new_cart);
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
		self.carts = carts;
		self.cart_count.store(self.carts.len() as u32, Ordering::SeqCst);
		self.mio.write_32(completion_signal_addr, COMPLETION_RESULT_OK);
	}
	
	fn write_cart_metadata_string(&mut self, string: &String, string_addr: u32) {
		let str_len = string.as_bytes().len().min(255) as u32;
		for i in 0 .. str_len {
			self.mio.write_8(string_addr + i, string.as_bytes()[i as usize]);
		}
		self.mio.write_8(string_addr + str_len, 0);
	}
	
	fn read_cart_metadata(&mut self, index: u32, metadata_struct_addr: u32, completion_addr: u32) {
		if index >= self.carts.len() as u32 {
			self.mio.write_32(completion_addr, COMPLETION_RESULT_CART_INDEX_OUT_OF_BOUNDS);
			self.mio.access_break();
			return;
		}
		let cart = self.carts[index as usize].clone();
		self.write_cart_metadata_string(&cart.name, metadata_struct_addr);
		self.write_cart_metadata_string(&cart.developer, metadata_struct_addr + 0x100);
		self.write_cart_metadata_string(&cart.developer_url, metadata_struct_addr + 0x200);
		self.write_cart_metadata_string(&cart.source, metadata_struct_addr + 0x300);
		let icon_data = {
			if let Some(icon_filename) = cart.icon {
				let mut icon_path = cart.path.clone();
				icon_path.push(icon_filename);
				println!("Icon path: {:?}!", icon_path);
				match image::open(icon_path) {
					Ok(icon_image) => {
						println!("Found icon image for cart {}!", cart.name);
						let icon_image = icon_image.into_rgba8();
						let (w, h) = icon_image.dimensions();
						if w != 64 || h != 64 {
							println!("Warning: icon image for cart {} is not 64x64!", cart.name);
							None
						} else {
							let mut image_data = vec![0u32; 64 * 64].into_boxed_slice();
							for y in 0 .. 64 as u32 {
								for x in 0 .. 64 as u32 {
									image_data[(x + y * 64) as usize] = 
										(icon_image.get_pixel(x, y)[0] as u32) |
										(icon_image.get_pixel(x, y)[1] as u32) << 8 |
										(icon_image.get_pixel(x, y)[2] as u32) << 16 |
										(icon_image.get_pixel(x, y)[3] as u32) << 24;
								}
							}
							Some(image_data)
						}
					},
					Err(..) => None
				}
			} else {
				None
			}
		};
		if let Some(image_data) = icon_data {
			for i in 0 .. 64 * 64 {
				self.mio.write_32(metadata_struct_addr + 0x400 + (i * 4) as u32, image_data[i]);
			}
		} else {
			for i in 0 .. 64 * 64 {
				self.mio.write_32(metadata_struct_addr + 0x400 + (i * 4) as u32, 0xFFFFFFFF);
			}
		}
		let (major, minor, rev) = cart.version;
		self.mio.write_32(metadata_struct_addr + 0x40400, rev);
		self.mio.write_32(metadata_struct_addr + 0x40404, minor);
		self.mio.write_32(metadata_struct_addr + 0x40408, major);
		self.mio.write_32(completion_addr, COMPLETION_RESULT_OK);
	}
	
	pub fn setup_data_access_fs(&mut self, completion_addr: u32, index: u32, file_name: PathBuf, flags: u32) {
		if index >= DATA_SLOT_COUNT {
			self.mio.write_32(completion_addr, COMPLETION_RESULT_DATA_SLOT_INDEX_OUT_OF_BOUNDS);
			return;
		}
		if let Some(current_cart) = &self.current_cart {
			match &current_cart.data {
				CartData::FsR(root_path) => {
					if flags & SETUP_DATA_ACCESS_FS_FLAG_WRITE != 0 {
						self.mio.write_32(completion_addr, COMPLETION_RESULT_BAD_OPERATION_FOR_DATA_FORMAT);
						return;
					}
					let mut file_path = root_path.clone();
					file_path.push(file_name);
					let file = match std::fs::File::open(&file_path) {
						Ok(file) => file,
						Err(..) => {
							self.mio.write_32(completion_addr, COMPLETION_RESULT_FAILED_OPENING_FILE);
							return;
						}
					};
					self.data_access_slots[index as usize] = DataAccessSlot::FileAccess {
						file_name: file_path,
						file,
						write: false
					};
					self.mio.write_32(completion_addr, COMPLETION_RESULT_OK);
				},
				CartData::FsRW(root_path) => {
					let mut file_path = root_path.clone();
					file_path.push(file_name);
					let file = match std::fs::File::open(&file_path) {
						Ok(file) => file,
						Err(..) => {
							self.mio.write_32(completion_addr, COMPLETION_RESULT_FAILED_OPENING_FILE);
							return;
						}
					};
					self.data_access_slots[index as usize] = DataAccessSlot::FileAccess {
						file_name: file_path,
						file,
						write: flags & SETUP_DATA_ACCESS_FS_FLAG_WRITE != 0
					};
					self.mio.write_32(completion_addr, COMPLETION_RESULT_OK);
				},
				_ => {
					self.mio.write_32(completion_addr, COMPLETION_RESULT_BAD_OPERATION_FOR_DATA_FORMAT);
					return;
				}
			}
		} else {
			self.mio.write_32(completion_addr, COMPLETION_RESULT_NO_CART_LOADED);
		}
	}
	
	fn close_data_access(&mut self, slot: u32, completion_addr: u32) {
		if slot >= DATA_SLOT_COUNT {
			self.mio.write_32(completion_addr, COMPLETION_RESULT_DATA_SLOT_INDEX_OUT_OF_BOUNDS);
			return;
		}
		self.data_access_slots[slot as usize] = DataAccessSlot::None;
		self.mio.write_32(completion_addr, COMPLETION_RESULT_OK);
	}
	
	fn read_file_at_up_to(file: &File, mut buf: &mut [u8], offset: u64) -> Result<usize, std::io::Error> {
		let buf_len = buf.len();
		let mut offset = offset;
	
		while !buf.is_empty() {
			match file.read_at(buf, offset) {
				Ok(0) => break,
				Ok(n) => {
					let tmp = buf;
					buf = &mut tmp[n..];
					offset = offset + n as u64;
				}
				Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
				Err(e) => return Err(e),
			}
		}
		Ok(buf_len - buf.len())
	}
	
	fn read_data(&mut self, slot: u32, offset: u32, length: u32, buffer_addr: u32, read_size_addr: u32, completion_addr: u32) {
		if slot >= DATA_SLOT_COUNT {
			self.mio.write_32(completion_addr, COMPLETION_RESULT_DATA_SLOT_INDEX_OUT_OF_BOUNDS);
			return;
		}
		match &self.data_access_slots[slot as usize] {
			DataAccessSlot::FileAccess {
				file,
				..
			} => {
				let mut total_read = 0;
				let mut eof = false;
				while !eof && total_read < length {
					let left_to_read = length - total_read;
					let mut read_size = left_to_read.min(0x1000);
					let mut read_vec = vec![0u8; left_to_read as usize].into_boxed_slice();
					match Self::read_file_at_up_to(file, &mut read_vec, (offset + total_read) as u64) {
						Ok(actually_read_size) => {
							if read_size as usize != actually_read_size {
								read_size = actually_read_size as u32;
								eof = true;
							}
						},
						_ => {
							self.mio.write_32(completion_addr, COMPLETION_RESULT_FAILED_READING_FILE);
							return;
						}
					}
					for i in 0 .. read_size {
						self.mio.write_8(buffer_addr + total_read + i, read_vec[i as usize]);
					}
					total_read += read_size;
				}
				self.mio.write_32(read_size_addr, total_read);
				self.mio.write_32(completion_addr, COMPLETION_RESULT_OK);
			},
			DataAccessSlot::BinaryAccess {
				/*binary_file_name,
				binary_file,
				offset,
				length,*/
				..
			} => {
				// todo
				self.mio.write_32(completion_addr, COMPLETION_RESULT_FAILED_READING_BINARY);
			},
			_ => {
				self.mio.write_32(completion_addr, COMPLETION_RESULT_DATA_SLOT_NOT_OPEN);
			}
		}
	}
	
	fn get_data_extents(&mut self, slot: u32, extents_addr: u32, completion_addr: u32) {
		if slot >= DATA_SLOT_COUNT {
			self.mio.write_32(completion_addr, COMPLETION_RESULT_DATA_SLOT_INDEX_OUT_OF_BOUNDS);
			return;
		}
		match &self.data_access_slots[slot as usize] {
			DataAccessSlot::FileAccess {
				file,
				..
			} => {
				match file.sync_all() {
					Ok(..) => {},
					Err(..) => {
						self.mio.write_32(completion_addr, COMPLETION_RESULT_FAILED_READING_FILE);
						return;
					}
				};
				match file.metadata() {
					Ok(metadata) => {
						let length = metadata.len();
						self.mio.write_32(extents_addr, length as u32);
						self.mio.write_32(completion_addr, COMPLETION_RESULT_OK);
					},
					_ => {
						self.mio.write_32(completion_addr, COMPLETION_RESULT_FAILED_READING_FILE);
					}
				}
			},
			DataAccessSlot::BinaryAccess {
				/*binary_file_name,
				binary_file,
				offset,
				length,*/
				..
			} => {
				// todo
				self.mio.write_32(completion_addr, COMPLETION_RESULT_FAILED_READING_BINARY);
			},
			_ => {
				self.mio.write_32(completion_addr, COMPLETION_RESULT_DATA_SLOT_NOT_OPEN);
			}
		}
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
							metadata_struct_addr,
							completion_addr
						} => {
							self.read_cart_metadata(index, metadata_struct_addr, completion_addr);
						},
						CartLoaderCmd::LoadCart {
							index, 
							error_write_addr
						} => {
							self.load_cart(index, error_write_addr);
						},
						CartLoaderCmd::SetupDataAccessFs {
							slot,
							file_name,
							completion_addr,
							flags
						} => {
							self.setup_data_access_fs(completion_addr, slot, file_name, flags);
						},
						CartLoaderCmd::SetupDataAccessBinary {
							/*slot,
							completion_addr,
							offset,
							length*/
							..
						} => {
							
						},
						CartLoaderCmd::CloseDataAccess{slot, completion_addr} => {
							self.close_data_access(slot, completion_addr);
						},
						CartLoaderCmd::ReadData{
							slot,
							offset,
							length,
							buffer_addr,
							read_size_addr,
							completion_addr
						} => {
							self.read_data(slot, offset, length, buffer_addr, read_size_addr, completion_addr);
						},
						CartLoaderCmd::GetDataExtents{
							slot,
							extents_addr,
							completion_addr
						} => {
							self.get_data_extents(slot, extents_addr, completion_addr);
						}
					}
				},
				Err(..) => {
					panic!("Cart loader thread receive error");
				}
			}
			self.mio.access_break();
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
	param4: Arc<AtomicU32>,
	param5: Arc<AtomicU32>
}

const REG_COMMAND: u32 = 0;
const REG_PARAM0: u32 = 4;
const REG_PARAM1: u32 = 8;
const REG_PARAM2: u32 = 12;
const REG_PARAM3: u32 = 16;
const REG_PARAM4: u32 = 20;
const REG_PARAM5: u32 = 24;
const REG_CART_COUNT: u32 = 28;

const COMMAND_ENUMERATE_CARTS: u32 = 0;
const COMMAND_READ_CART_METADATA: u32 = 1;
const COMMAND_LOAD_CART: u32 = 2;
const COMMAND_SETUP_DATA_ACCESS_FS: u32 = 3;
//const COMMAND_SETUP_DATA_ACCESS_BINARY: u32 = 4;
const COMMAND_CLOSE_DATA_ACCESS: u32 = 5;
const COMMAND_READ_DATA: u32 = 6;
//const COMMAND_WRITE_DATA: u32 = 7;
const COMMAND_GET_DATA_EXTENTS: u32 = 8;

const SETUP_DATA_ACCESS_FS_FLAG_WRITE: u32 = 1 << 0;

impl CartLoaderPeripheral {
	fn read_filename(mio: &FmMemoryIO, addr: u32) -> Option<PathBuf> {
		let mut path_str = String::from("");
		let mut path_buf: Option<PathBuf> = None;
		let mut i = 0;
		loop {
			match mio.read_8(addr + i) {
				MemReadResult::Ok(c) => {
					if c == 0 {
						match &mut path_buf {
							Some(path_buf) => {
								if (path_str.len() != 0) {
									path_buf.push(path_str);
								}
								break;
							},
							None => {
								path_buf = Some(PathBuf::from(path_str));
								path_str = String::from("");
							}
						}
					} else if c as char == '/' {
						match &mut path_buf {
							Some(path_buf) => {
								path_buf.push(path_str);
							},
							None => {
								path_buf = Some(PathBuf::from(path_str));
							}
						}
						path_str = String::from("");
					} else {
						path_str += String::from(c as char).as_str();
					}
				},
				_ => return None
			}
			i = i + 1;
		}
		path_buf
	}
	
	pub fn write_32(&self, mio: &mut FmMemoryIO, offset: u32, value: u32) -> MemWriteResult {
		if (offset & 0x03) != 0 {
			return MemWriteResult::ErrAlignment;
		}
		match offset {
			REG_COMMAND => {
				if self.command(mio, value) {
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
			REG_PARAM4 => {
				self.param4.store(value, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			REG_PARAM5 => {
				self.param5.store(value, Ordering::SeqCst);
				MemWriteResult::Ok
			},
			_ => {
				MemWriteResult::PeripheralError
			}
		}
	}
	
	fn command(&self, mio: &mut FmMemoryIO, command: u32) -> bool {
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
			COMMAND_READ_CART_METADATA => {
				let index = self.param0.load(Ordering::SeqCst);
				let metadata_struct_addr = self.param1.load(Ordering::SeqCst);
				let completion_addr = self.param2.load(Ordering::SeqCst);
				self.cmd_tx.send(CartLoaderCmd::ReadCartMetadata {
					index,
					metadata_struct_addr,
					completion_addr
				}).unwrap();
				true
			},
			COMMAND_SETUP_DATA_ACCESS_FS => {
				let index = self.param0.load(Ordering::SeqCst);
				let file_name_addr = self.param1.load(Ordering::SeqCst);
				let completion_addr = self.param2.load(Ordering::SeqCst);
				let flags = self.param3.load(Ordering::SeqCst);
				let file_name = Self::read_filename(mio, file_name_addr);
				if let Some(file_name) = file_name {
					self.cmd_tx.send(CartLoaderCmd::SetupDataAccessFs {
						slot: index,
						file_name,
						completion_addr,
						flags
					}).unwrap();
					true
				} else {
					mio.write_32(completion_addr, COMPLETION_RESULT_FILENAME_READ_ERROR);
					false
				}
			},
			COMMAND_CLOSE_DATA_ACCESS => {
				let index = self.param0.load(Ordering::SeqCst);
				let completion_addr = self.param1.load(Ordering::SeqCst);
				self.cmd_tx.send(CartLoaderCmd::CloseDataAccess {
					slot: index,
					completion_addr
				}).unwrap();
				true
			},
			// slot: u32, offset: u32, length: u32, buffer_addr: u32, completion_addr: u32
			COMMAND_READ_DATA => {
				let slot = self.param0.load(Ordering::SeqCst);
				let offset = self.param1.load(Ordering::SeqCst);
				let length = self.param2.load(Ordering::SeqCst);
				let buffer_addr = self.param3.load(Ordering::SeqCst);
				let read_size_addr = self.param4.load(Ordering::SeqCst);
				let completion_addr = self.param5.load(Ordering::SeqCst);
				self.cmd_tx.send(CartLoaderCmd::ReadData {
					slot,
					offset,
					length,
					buffer_addr,
					read_size_addr,
					completion_addr
				}).unwrap();
				true
			},
			COMMAND_GET_DATA_EXTENTS => {
				let slot = self.param0.load(Ordering::SeqCst);
				let extents_addr = self.param1.load(Ordering::SeqCst);
				let completion_addr = self.param2.load(Ordering::SeqCst);
				self.cmd_tx.send(CartLoaderCmd::GetDataExtents {
					slot,
					extents_addr,
					completion_addr
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
