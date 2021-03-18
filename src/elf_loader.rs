use mem::size_of;
use simple_endian::{u32le, u16le};
use rv_vsys::{MemIO, MTimer};
use std::{collections::HashMap, fmt::Display, fmt, mem};
use bytemuck::{Pod, Zeroable, from_bytes};

const EI_NIDENT: usize = 16;
const ELF_CLASS_32: u8 = 1;
const ELF_DATA2_LSB: u8 = 1;
const ELF_VERSION_CURRENT: u8 = 1;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct ElfHeaderRaw {
	ident: [u8; EI_NIDENT],
	object_type: u16le,
	machine: u16le,
	version: u32le,
	entry: u32le,
	program_header_offset: u32le,
	section_header_offset: u32le,
	flags: u32le,
	header_size: u16le,
	program_header_entry_size: u16le,
	program_header_entry_count: u16le,
	section_header_entry_size: u16le,
	section_header_entry_count: u16le,
	section_name_table_section_index: u16le,
}

unsafe impl Zeroable for ElfHeaderRaw {}
unsafe impl Pod for ElfHeaderRaw {}

#[derive(Debug)]
enum ElfObjectType {
	None,
	Relocatable,
	Executable,
	Shared,
	Core,
	Unknown
}

impl ElfObjectType {
	pub fn from_u16(val: u16) -> Self {
		match val {
			0 => ElfObjectType::None,
			1 => ElfObjectType::Relocatable,
			2 => ElfObjectType::Executable,
			3 => ElfObjectType::Shared,
			4 => ElfObjectType::Core,
			_ => ElfObjectType::Unknown
		}
	}
}

#[derive(Debug)]
enum ElfMachineType {
	None,
	M32,
	Sparc,
	I386,
	M68000,
	M88000,
	I860,
	Mips,
	RiscV,
	Unknown
}

impl ElfMachineType {
	pub fn from_u16(val: u16) -> Self {
		match val {
			0 => ElfMachineType::None,
			1 => ElfMachineType::M32,
			2 => ElfMachineType::Sparc,
			3 => ElfMachineType::I386,
			4 => ElfMachineType::M68000,
			5 => ElfMachineType::M88000,
			7 => ElfMachineType::I860,
			8 => ElfMachineType::Mips,
			243 => ElfMachineType::RiscV,
			_ => ElfMachineType::Unknown
		}
	}
}

#[derive(Debug)]
enum ElfVersion {
	None,
	Current,
	Unknown,
}

impl ElfVersion {
	pub fn from_u32(val: u32) -> Self {
		match val {
			0 => ElfVersion::None,
			1 => ElfVersion::Current,
			_ => ElfVersion::Unknown,
		}
	}
}

#[derive(Debug)]
struct ElfHeader {
	ident: [u8; EI_NIDENT],
	object_type: ElfObjectType,
	machine_type: ElfMachineType,
	version: ElfVersion,
	entry: u32,
	program_header_offset: u32,
	section_header_offset: u32,
	flags: u32,
	header_size: u16,
	program_header_entry_size: u16,
	program_header_entry_count: u16,
	section_header_entry_size: u16,
	section_header_entry_count: u16,
	section_name_table_section_index: u16,
}

impl ElfHeader {
	pub fn from_raw(raw: &ElfHeaderRaw) -> Self {
		unsafe {
			ElfHeader {
				ident: raw.ident,
				object_type: ElfObjectType::from_u16(raw.object_type.to_native()),
				machine_type: ElfMachineType::from_u16(raw.machine.to_native()),
				version: ElfVersion::from_u32(raw.version.to_native()),
				entry: raw.entry.to_native(),
				program_header_offset: raw.program_header_offset.to_native(),
				section_header_offset: raw.section_header_offset.to_native(),
				flags: raw.flags.to_native(),
				header_size: raw.header_size.to_native(),
				program_header_entry_size: raw.program_header_entry_size.to_native(),
				program_header_entry_count: raw.program_header_entry_count.to_native(),
				section_header_entry_size: raw.section_header_entry_size.to_native(),
				section_header_entry_count: raw.section_header_entry_count.to_native(),
				section_name_table_section_index: raw.section_name_table_section_index.to_native(),
			}
		}
	}
	
	pub fn has_program_headers(&self) -> bool {
		self.program_header_offset != 0
	}
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct ElfSectionHeaderRaw {
	name_index: u32le,
	section_type: u32le,
	flags: u32le,
	address: u32le,
	data_offset: u32le,
	size: u32le,
	link_index: u32le,
	info: u32le,
	address_align: u32le,
	entry_size: u32le,
}

unsafe impl Zeroable for ElfSectionHeaderRaw {}
unsafe impl Pod for ElfSectionHeaderRaw {}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
enum ElfSectionType {
	Null,
	ProgramBits,
	SymbolTable,
	StringTable,
	RelocationAddend,
	SymbolHash,
	Dynamic,
	Note,
	NoBits,
	Relocation,
	ReservedShLib,
	DynamicSymbolTable,
	Processor(u32),
	User(u32),
	Unknown
}

impl Display for ElfSectionType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			ElfSectionType::Null               => write!(f, "Null           "),
			ElfSectionType::ProgramBits        => write!(f, "ProgBits       "),
			ElfSectionType::SymbolTable        => write!(f, "Sym Table      "),
			ElfSectionType::StringTable        => write!(f, "String Table   "),
			ElfSectionType::RelocationAddend   => write!(f, "Relocation (A) "),
			ElfSectionType::SymbolHash         => write!(f, "Sym Hash Table "),
			ElfSectionType::Dynamic            => write!(f, "Dynamic        "),
			ElfSectionType::Note               => write!(f, "Note           "),
			ElfSectionType::NoBits             => write!(f, "NoBits         "),
			ElfSectionType::Relocation         => write!(f, "Relocation     "),
			ElfSectionType::ReservedShLib      => write!(f, "Reserved(SHLIB)"),
			ElfSectionType::DynamicSymbolTable => write!(f, "Dyn Sym Table  "),
			ElfSectionType::Processor(val)=> write!(f, "Proc {:#010x}", val),
			ElfSectionType::User(val)     => write!(f, "User {:#010x}", val),
			ElfSectionType::Unknown            => write!(f, "Unknown        "),
		}
    }
}

#[allow(dead_code)]
impl ElfSectionType {
	pub fn from_u32(val: u32) -> Self {
		match val {
			0 => ElfSectionType::Null,
			1 => ElfSectionType::ProgramBits,
			2 => ElfSectionType::SymbolTable,
			3 => ElfSectionType::StringTable,
			4 => ElfSectionType::RelocationAddend,
			5 => ElfSectionType::SymbolHash,
			6 => ElfSectionType::Dynamic,
			7 => ElfSectionType::Note,
			8 => ElfSectionType::NoBits,
			9 => ElfSectionType::Relocation,
			10 => ElfSectionType::ReservedShLib,
			11 => ElfSectionType::DynamicSymbolTable,
			_ => {
				if val < 0x70000000 {
					ElfSectionType::Unknown
				} else if val < 0x80000000 {
					ElfSectionType::Processor(val)
				} else {
					ElfSectionType::User(val)
				}
			}
		}
	}
}

#[derive(Debug)]
struct ElfSectionHeader {
	name_index: u32,
	section_type: ElfSectionType,
	flags: u32,
	address: u32,
	data_offset: u32,
	size: u32,
	link_index: u32,
	info: u32,
	address_align: u32,
	entry_size: u32,
}

#[allow(dead_code)]
impl ElfSectionHeader {
	pub fn from_raw(raw: &ElfSectionHeaderRaw) -> Self {
		unsafe {
				ElfSectionHeader {
				name_index: raw.name_index.to_native(),
				section_type: ElfSectionType::from_u32(raw.section_type.to_native()),
				flags: raw.flags.to_native(),
				address: raw.address.to_native(),
				data_offset: raw.data_offset.to_native(),
				size: raw.size.to_native(),
				link_index: raw.link_index.to_native(),
				info: raw.info.to_native(),
				address_align: raw.address_align.to_native(),
				entry_size: raw.entry_size.to_native(),
			}
		}
	}
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct ElfProgramHeaderEntryRaw {
	header_type: u32le,
	offset: u32le,
	virt_addr: u32le,
	phys_addr: u32le,
	file_size: u32le,
	mem_size: u32le,
	flags: u32le,
	align: u32le,
}

unsafe impl Zeroable for ElfProgramHeaderEntryRaw {}
unsafe impl Pod for ElfProgramHeaderEntryRaw {}

#[allow(dead_code)]
struct ElfStringTable {
	data: Vec<u8>
}

#[allow(dead_code)]
impl ElfStringTable {
	pub fn new(file_data: &[u8], string_table_header: &ElfSectionHeader) -> Result<ElfStringTable, String> {
		let mut data = Vec::new();
		if (string_table_header.data_offset + string_table_header.size) as usize >= file_data.len() {
			return Err("Elf string table references data beyond end of file".to_string());
		}
		for i in 0 .. string_table_header.size {
			data.push(file_data[(string_table_header.data_offset + i) as usize]);
		}
		Ok(ElfStringTable {
			data: data
		})
	}
	
	pub fn get_string(&self, index: u32) -> Option<String> {
		if index as usize >= self.data.len() {
			None
		} else {
			let mut len = 0;
			while self.data[(index + len) as usize] != 0 {
				len = len + 1;
			}
			Some(String::from_utf8_lossy(&self.data[index as usize .. (index + len) as usize]).to_string())
		}
	}
}

#[derive(Debug)]
enum ElfProgramHeaderType {
	Null,
	Load,
	Dynamic,
	Interpreter,
	Note,
	ReservedShLib,
	ProgramHeader,
	Processor(u32),
	Unknown
}

impl ElfProgramHeaderType {
	pub fn from_u32(val: u32) -> Self {
		match val {
			0 => ElfProgramHeaderType::Null,
			1 => ElfProgramHeaderType::Load,
			2 => ElfProgramHeaderType::Dynamic,
			3 => ElfProgramHeaderType::Interpreter,
			4 => ElfProgramHeaderType::Note,
			5 => ElfProgramHeaderType::ReservedShLib,
			6 => ElfProgramHeaderType::ProgramHeader,
			_ => {
				if val >= 0x70000000 && val < 0x80000000 {
					ElfProgramHeaderType::Processor(val)
				} else {
					ElfProgramHeaderType::Unknown
				}
			}
		}
	}
}

#[allow(dead_code)]
struct ElfProgramHeaderEntry {
	header_type: ElfProgramHeaderType,
	offset: u32,
	virt_addr: u32,
	phys_addr: u32,
	file_size: u32,
	mem_size: u32,
	flags: u32,
	align: u32
}

#[allow(dead_code)]
impl ElfProgramHeaderEntry {
	pub fn from_raw(raw: &ElfProgramHeaderEntryRaw) -> ElfProgramHeaderEntry {
		unsafe {
			ElfProgramHeaderEntry {
				header_type: ElfProgramHeaderType::from_u32(raw.header_type.to_native()),
				offset: raw.offset.to_native(),
				virt_addr: raw.virt_addr.to_native(),
				phys_addr: raw.phys_addr.to_native(),
				file_size: raw.file_size.to_native(),
				mem_size: raw.mem_size.to_native(),
				flags: raw.flags.to_native(),
				align: raw.align.to_native()
			}
		}
	}
}


#[allow(dead_code)]
pub const ELF_SECTION_FLAGS_WRITE: u32 = 0x1;
#[allow(dead_code)]
pub const ELF_SECTION_FLAGS_ALLOC: u32 = 0x2;
#[allow(dead_code)]
pub const ELF_SECTION_FLAGS_EXEC: u32 = 0x4;
#[allow(dead_code)]
pub const ELF_SECTION_FLAGS_PROC_MASK: u32 = 0xF0000000;

fn check_header(header: &ElfHeader) -> Result<(), String> {
	match &header.ident[0..4] {
		&[0x7F, b'E', b'L', b'F'] => {},
		_ => {
			return Err(format!("Magic number wasn't [0x7F, E, L, F], ({:?}) invalid elf file", &header.ident[0..4]).to_string());
		}
	}
	if header.ident[4] != ELF_CLASS_32 {
		return Err("Elf file is not 32-bits".to_string());
	}
	if header.ident[5] != ELF_DATA2_LSB {
		return Err("Elf file is not little-endian".to_string());
	}
	if header.ident[6] != ELF_VERSION_CURRENT {
		return Err("Elf magic version wrong".to_string());
	};
	Ok(())
}

fn load_program_headers(file_data: &[u8], elf_header: &ElfHeader) -> Result<Vec<ElfProgramHeaderEntry>, String> {
	let file_header_size = elf_header.program_header_entry_size as u32;
	let mem_header_size = size_of::<ElfProgramHeaderEntryRaw>() as u32;
	if file_header_size < mem_header_size {
		return Err("Elf file declares program header size to be smaller than minimum elf program header size".to_string());
	}
	let mut program_headers = Vec::new();
	for h in 0 .. elf_header.program_header_entry_count as u32 {
		let header_offset = elf_header.program_header_offset + file_header_size * h;
		if (file_data.len() as u32) < header_offset + mem_header_size {
			return Err("Elf file declares program header beyond end of file".to_string());
		}
		let raw_header: ElfProgramHeaderEntryRaw = *from_bytes(&file_data[header_offset as usize..header_offset as usize + mem_header_size as usize]);
		program_headers.push(ElfProgramHeaderEntry::from_raw(&raw_header));
	}
	Ok(program_headers)
}

#[allow(dead_code)]
fn load_section_headers(file_data: &[u8], elf_header: &ElfHeader) -> Result<Vec<ElfSectionHeader>, String> {
	let mut header_size = elf_header.section_header_entry_size as u32;
	if (header_size as usize) < size_of::<ElfSectionHeaderRaw>() {
		return Err("Elf file declares section header size to be smaller than minimum elf section header size".to_string());
	} else {
		header_size = size_of::<ElfSectionHeaderRaw>() as u32;
	}
	let mut section_headers = Vec::new();
	for h in 0 .. elf_header.section_header_entry_count as u32 {
		let header_offset = elf_header.section_header_offset + header_size * h;
		if (file_data.len() as u32) < header_offset + header_size {
			return Err("Elf file declares section header beyond end of file".to_string());
		}
		let raw_header: ElfSectionHeaderRaw = *from_bytes(&file_data[header_offset as usize..header_offset as usize + header_size as usize]);
		section_headers.push(ElfSectionHeader::from_raw(&raw_header));
	}
	Ok(section_headers)
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct ElfSymbolRaw {
	name_index: u32le,
	value: u32le,
	size: u32le,
	info: u8,
	other: u8,
	section_index: u16le
}

unsafe impl Zeroable for ElfSymbolRaw {}
unsafe impl Pod for ElfSymbolRaw {}

#[allow(dead_code)]
#[derive(Debug)]
enum ElfSymbolBinding {
	Local,
	Global,
	Weak,
	Proc(u32),
	Unknown
}

#[allow(dead_code)]
impl ElfSymbolBinding {
	pub fn from_u32(val: u32) -> Self {
		match val {
			0 => ElfSymbolBinding::Local,
			1 => ElfSymbolBinding::Global,
			2 => ElfSymbolBinding::Weak,
			13 ..= 15 => ElfSymbolBinding::Proc(val),
			_ => ElfSymbolBinding::Unknown
		}
	}
}

#[allow(dead_code)]
#[derive(Debug)]
enum ElfSymbolType {
	None,
	Object,
	Function,
	Section,
	File,
	Proc(u32),
	Unknown
}

#[allow(dead_code)]
impl ElfSymbolType {
	pub fn from_u32(val: u32) -> Self {
		match val {
			0 => ElfSymbolType::Object,
			1 => ElfSymbolType::Function,
			2 => ElfSymbolType::Section,
			3 => ElfSymbolType::File,
			4 => ElfSymbolType::Proc(val),
			13 ..= 15 => ElfSymbolType::None,
			_ => ElfSymbolType::Unknown,
		}
	}
}

#[allow(dead_code)]
#[derive(Debug)]
enum ElfSymbolSection {
	Absolute,
	Common,
	Undefined,
	Section(u32),
	Proc(u32),
	Unknown,
}

#[allow(dead_code)]
impl ElfSymbolSection { 
	pub fn from_u32(val: u32) -> Self {
		match val {
			0 => ElfSymbolSection::Undefined,
			1 ..= 0xFEFF => ElfSymbolSection::Section(val),
			0xFF00 ..= 0xFFF0 => ElfSymbolSection::Proc(val),
			0xFFF1 => ElfSymbolSection::Absolute,
			0xFFF2 => ElfSymbolSection::Common,
			_ => ElfSymbolSection::Unknown
		}
	}
}

#[derive(Debug)]
struct ElfSymbol {
	binding: ElfSymbolBinding,
	sym_type: ElfSymbolType,
	section: ElfSymbolSection,
	value: u32,
	size: u32,
	name: String
}

#[allow(dead_code)]
impl ElfSymbol {
	pub fn from_raw(raw: &ElfSymbolRaw, name: String) -> Self {
		unsafe {
			ElfSymbol {
				binding: ElfSymbolBinding::from_u32((raw.info >> 4) as u32),
				sym_type: ElfSymbolType::from_u32((raw.info & 0x0F) as u32),
				section: ElfSymbolSection::from_u32(raw.section_index.to_native() as u32),
				value: raw.value.to_native(),
				size: raw.size.to_native(),
				name: name
			}
		}
	}
}

#[allow(dead_code)]
fn symbol_temporary_name(sym_raw: &ElfSymbolRaw) -> String {
	let sym_type_name = match ElfSymbolType::from_u32((sym_raw.info & 0x0F) as u32) {
		ElfSymbolType::None => "$N_".to_string(),
		ElfSymbolType::Object => "$O_".to_string(),
		ElfSymbolType::Function => "$F_".to_string(),
		ElfSymbolType::Section => "$S_".to_string(),
		ElfSymbolType::File => "$E_".to_string(),
		ElfSymbolType::Proc(val) => format!("$P{:04x}_", val).to_string(),
		ElfSymbolType::Unknown => "$U_".to_string(),
	};
	let sym_section_name = match ElfSymbolSection::from_u32(unsafe {sym_raw.section_index.to_native()} as u32) {
		ElfSymbolSection::Absolute => "abs_".to_string(),
		ElfSymbolSection::Common => "com_".to_string(),
		ElfSymbolSection::Undefined => "undef_".to_string(),
		ElfSymbolSection::Section(sect) => format!("s{:04x}_", sect).to_string(),
		ElfSymbolSection::Proc(val) => format!("p{:04x}_", val).to_string(),
		ElfSymbolSection::Unknown => "unknown_".to_string()
	};
	unsafe {
		format!("{}{}{:08x}", sym_type_name, sym_section_name, sym_raw.value).to_string()
	}
}

#[allow(dead_code)]
fn load_symbol_tables(file_data: &[u8], section_headers: &Vec<ElfSectionHeader>, string_tables: &ElfStringTables) -> Result<HashMap<String, ElfSymbol>, String> {
	let mut symbol_map: HashMap<String, ElfSymbol> = HashMap::new();
	for h in 0 .. section_headers.len() {
		let s_hdr = &section_headers[h];
		match s_hdr.section_type {
			ElfSectionType::SymbolTable => {
				let file_entry_size = s_hdr.entry_size;
				let mem_entry_size = size_of::<ElfSymbolRaw>() as u32;
				if file_entry_size < mem_entry_size {
					return Err("Elf symbol table declares symbol to be smaller than minimum elf symbol size".to_string());
				}
				let symbol_count = s_hdr.size / file_entry_size;
				for s in 0 .. symbol_count {
					let symbol_address = s_hdr.data_offset + file_entry_size * s;
					let symbol_raw: ElfSymbolRaw = *from_bytes(&file_data[symbol_address as usize..(symbol_address + mem_entry_size) as usize]);
					let symbol_name = string_tables.get_symbol_name(s_hdr.link_index, unsafe {symbol_raw.name_index.to_native()}).unwrap_or(symbol_temporary_name(&symbol_raw));
					let symbol = ElfSymbol::from_raw(&symbol_raw, symbol_name.clone());
					symbol_map.insert(symbol_name.clone(), symbol);
				}
			},
			ElfSectionType::DynamicSymbolTable => {
				let file_entry_size = s_hdr.entry_size;
				let mem_entry_size = size_of::<ElfSymbolRaw>() as u32;
				if file_entry_size < mem_entry_size {
					return Err("Elf symbol table declares symbol to be smaller than minimum elf symbol size".to_string());
				}
				let symbol_count = s_hdr.size / file_entry_size;
				for s in 0 .. symbol_count {
					let symbol_address = s_hdr.data_offset + file_entry_size * s;
					let symbol_raw: ElfSymbolRaw = *from_bytes(&file_data[symbol_address as usize..(symbol_address + mem_entry_size) as usize]);
					let symbol_name = string_tables.get_symbol_name(s_hdr.link_index, unsafe {symbol_raw.name_index.to_native()}).unwrap_or(symbol_temporary_name(&symbol_raw));
					let symbol = ElfSymbol::from_raw(&symbol_raw, symbol_name.clone());
					symbol_map.insert(symbol_name.clone(), symbol);
				}
			},
			_ => {}
		}
	}
	Ok(symbol_map)
}

#[allow(dead_code)]
struct ElfStringTables {
	section_name_table: Option<ElfStringTable>,
	symbol_name_tables: HashMap<u32, ElfStringTable>
}

#[allow(dead_code)]
impl ElfStringTables {
	fn get_section_name(&self, section_header: &ElfSectionHeader) -> Option<String> {
		match &self.section_name_table {
			None => None,
			Some(name_table) => {
				name_table.get_string(section_header.name_index)
			}
		}
	}
	
	fn get_symbol_name(&self, table_id: u32, name_id: u32) -> Option<String> {
		if name_id == 0 {
			return None;
		}
		match self.symbol_name_tables.get(&table_id) {
			Some(name_table) => {
				name_table.get_string(name_id)
			},
			None => {
				None
			}
		}
	}
}

#[allow(dead_code)]
fn collect_string_tables(file_data: &[u8], elf_header: &ElfHeader, section_headers: &Vec<ElfSectionHeader>) -> Result<ElfStringTables, String> {
	if elf_header.section_name_table_section_index as usize >= section_headers.len() {
		return Err("Elf header specifies out-of-range string table for section header names".to_string())
	}
	let section_name_table = if elf_header.section_name_table_section_index == 0 {
		None
	} else {
		let string_table_header = &section_headers[elf_header.section_name_table_section_index as usize];
		match ElfStringTable::new(file_data, string_table_header) {
			Ok(table) => Some(table),
			Err(error_string) => return Err(error_string)
		}
	};
	let mut symbol_name_tables = HashMap::<u32, ElfStringTable>::new();
	for i in 1 .. section_headers.len() {
		if i != elf_header.section_name_table_section_index as usize {
			if section_headers[i].section_type == ElfSectionType::StringTable {
				match ElfStringTable::new(file_data, &section_headers[i]) {
					Ok(table) => {
						symbol_name_tables.insert(i as u32, table);
					},
					Err(error_string) => return Err(error_string)
				}
			}
		}
	}
	Ok(ElfStringTables{
		section_name_table: section_name_table,
		symbol_name_tables: symbol_name_tables
	})
}

/*
fn align_to(start: u32, align: u32) -> u32 {
	let align = if align == 0 {
		1
	} else {
		align
	};
	let x = start + align - 1;
	let y = x % align;
	x - y
}
fn load_section<Mem: MemIO>(_file_data: &[u8], _elf_header: &ElfHeader, _section: &ElfSectionHeader, _string_tables: &ElfStringTables, _symbol_table: &HashMap<String, ElfSymbol>, _mio: &mut Mem, _image_counter: &mut u32) -> Result<(), String> {
	Ok(())
}
*/

fn load_program_header<Timer: MTimer, Mem: MemIO<Timer>>(file_data: &[u8], program_header: &ElfProgramHeaderEntry, mio: &mut Mem, base_addr: u32) -> Result<(), String> {
	match program_header.header_type {
		ElfProgramHeaderType::Load => {
			let v_addr = program_header.virt_addr;
			let m_addr = v_addr + base_addr;
			let offset = program_header.offset;
			let m_size = program_header.mem_size;
			let f_size = program_header.file_size;
			let z_size = m_size - f_size;
			let zm_addr = m_addr + f_size;
			if (offset + f_size) as usize > file_data.len() {
				return Err("program header specifies data outside of elf file range".to_string());
			}
			for i in 0 .. f_size {
				match mio.write_8(m_addr + i, file_data[(offset + i) as usize]) {
					rv_vsys::MemWriteResult::Ok => {},
					_ => return Err(format!("failed to write to address {:#010x}", m_addr + i).to_string())
				}
			}
			for i in 0 .. z_size {
				match mio.write_8(zm_addr + i, 0) {
					rv_vsys::MemWriteResult::Ok => {},
					_ => return Err(format!("failed to write zero at address {:#010x}", m_addr + i).to_string())
				}
			}
		},
		_ => {}
	}
	Ok(())
}

pub fn load_elf<Timer: MTimer, Mem: MemIO<Timer>>(file_data: &[u8], mio: &mut Mem, image_base: u32) -> Result<u32, String> {
	let header_size = mem::size_of::<ElfHeaderRaw>();
	if file_data.len() < header_size {
		return Err("Elf file too small to be Elf format".to_string())
	};
	let raw_header: ElfHeaderRaw = *from_bytes(&file_data[0..header_size]);
	let header: ElfHeader = ElfHeader::from_raw(&raw_header);
	match check_header(&header) {
		Ok(()) => {},
		Err(error_string) => return Err(error_string)
	}
	if ! header.has_program_headers() {
		return Err("Elf file has no program header".to_string());
	}
	let program_headers = match load_program_headers(file_data, &header) {
		Ok(headers) => headers,
		Err(error_string) => return Err(error_string)
	};
	//println!("program header count: {}", program_headers.len());
	for i in 0 .. program_headers.len() {
		let p_hdr = &program_headers[i];
		//println!("* {}: {:?} - offset: {:#010x}, virt addr: {:#010x}, phys addr: {:#010x}, file_size: {:#010x}, mem size: {:#010x}, flags: {}{}{}, align: {:#010x}", i + 1, p_hdr.header_type, p_hdr.offset, p_hdr.virt_addr, p_hdr.phys_addr, p_hdr.file_size, p_hdr.mem_size, if (p_hdr.flags & ELF_SECTION_FLAGS_WRITE) != 0 {"W"} else {"-"}, if (p_hdr.flags & ELF_SECTION_FLAGS_ALLOC) != 0 {"A"} else {"-"}, if (p_hdr.flags & ELF_SECTION_FLAGS_EXEC) != 0 {"E"} else {"-"}, p_hdr.align);
		match load_program_header(file_data, p_hdr, mio, image_base) {
			Ok(()) => {},
			Err(error) => return Err(error)
		}
	}
	/*let section_headers = match load_section_headers(file_data, &header) {
		Ok(headers) => headers,
		Err(error_string) => return Err(error_string)
	};
	//println!("section header count: {}", section_headers.len());
	let string_tables = match collect_string_tables(file_data, &header, &section_headers) {
		Ok(string_tables) => string_tables,
		Err(error_string) => return Err(error_string)
	};
	let symbol_table = match load_symbol_tables(file_data, &section_headers, &string_tables) {
		Ok(symbol_table) => symbol_table,
		Err(error_string) => return Err(error_string)
	};
	/*println!("symbol count: {}", symbol_table.len());
	for (name, sym) in &symbol_table {
		println!("* {}: {:?}", name, sym);
	}*/
	let mut image_counter = image_base;
	for i in 0 .. section_headers.len() {
		let s_hdr = &section_headers[i];
		if let Some(table_name) = string_tables.get_section_name(s_hdr) {
			println!("* {}, {:17} - {}, flags: {}{}{}, address: {:#010x}, data offset: {:#010x}, size: {:#010x}, link index: {:#010x}, address_align: {:#010x}, entry_size: {:#010x}, info: {:#010x}", i, table_name, s_hdr.section_type, if (s_hdr.flags & ELF_SECTION_FLAGS_WRITE) != 0 {"W"} else {"-"}, if (s_hdr.flags & ELF_SECTION_FLAGS_ALLOC) != 0 {"A"} else {"-"}, if (s_hdr.flags & ELF_SECTION_FLAGS_EXEC) != 0 {"E"} else {"-"}, s_hdr.address, s_hdr.data_offset, s_hdr.size, s_hdr.link_index, s_hdr.address_align, s_hdr.entry_size, s_hdr.info);
		} else {
			println!("* {} - {}, flags: {}{}{}, address: {:#010x}, data offset: {:#010x}, size: {:#010x}, link index: {:#010x}, address_align: {:#010x}, entry_size: {:#010x}, info: {:#010x}", i, s_hdr.section_type, if (s_hdr.flags & ELF_SECTION_FLAGS_WRITE) != 0 {"W"} else {"-"}, if (s_hdr.flags & ELF_SECTION_FLAGS_ALLOC) != 0 {"A"} else {"-"}, if (s_hdr.flags & ELF_SECTION_FLAGS_EXEC) != 0 {"E"} else {"-"}, s_hdr.address, s_hdr.data_offset, s_hdr.size, s_hdr.link_index, s_hdr.address_align, s_hdr.entry_size, s_hdr.info);
		}
		match load_section(file_data, &header, s_hdr, &string_tables, &symbol_table, mio, &image_counter) {
			Ok(()) => {},
			Err(error_string) => return Err(error_string)
		}
	}*/
	mio.access_break();
	Ok(header.entry)
}