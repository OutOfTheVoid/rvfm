# RVFM: Risc-V Fun Machine

RVFM is a virtual console, including an RV32IMACF emulator written entirely in Rust.

## Specs

- Dual core Risc-V CPU
  - 20 Mips per hart
  - RV32IMACF
  - Machine mode only
  - Flat memory model
- Hardware-accelerated GPU
  - 256 x 192 native resoltution
  - Raw Framebuffer mode for cpu rendering
- Elf based "cartridges"
  - Binary + JSON Metadata + Data store
  - Save file/directory per cartridge
- Coherent RAM
  - All individual memory access in RAM is gaurenteed to be atomic
- Memory-mapped peripherals
  - see memory_map.txt for details
- Cartridge hot-swapping thro
- C based HAL, but any language which can compile to RV32IMACF is supported

## RVFM Cartridge Format

RVFM Catridges are the basic form of a program for RVFM. They consist of a cartridge directory, which contains everything used by that program, along with a json file which tells RVFM how to use the cartridge.

The basic directory structure is as follows:
```
cart
├── cart.elf         # program binary - required
├── cart.json        # metadata file - required
├── cart.png         # icon file - optional, but should be a 64x64 png or jpg
├── data             # data folder - optional, see data format section
│   └── <data files>
└── data-file        # data file - optional, see data format section
```

### Example cart.json

```json
{
    "name": "my_cart",                                  # cartridge name - pick something unique
    "version": "0.1.0",                                 # version - semver style version
    "developer": "Liam Taylor",                         # developer (optional) - developer name
    "developer_url": "https://github.com/OutOfTheVoid", # developer url (optional) - I use my github user page
    "source": "https://github.com/OutOfTheVoid/rvfm",   # source url (optional) - source code for the cart
    "binary": "cart.elf",                               # cart directory relative path to the binary
    "data": {                                           # data definition, see below
        "format": "fs-ro",
        "root_dir": "data"
    },
    "icon": "cart.png"                                  # cart directory relative path to the icon
}
```

### Data format

The `data` field of the cart json accepts a few different kinds of data stores, specified by different values of the `format` field. While read-write is supported for cartridge data, it is recommended that cartridges use the cartridge-save peripheral for save-states rather than the cartrige data store for save data, as this keeps RVFM saves in once place and is easier to work with for the end-user of your cartridge:

- `none`: No cart data
- `fs-ro`: Read-only filesystem
  - `root_dir` specifies the cart-relative path to the data filesystem root path
  - Useful if you want to keep your assets as separate files
- `fs-rw`: Read-Write filesystem
  - `root_dir`: same as for `fs-ro`
- `binary-ro`: Read-only binary blob
  - `data_file`: specifies the cart-relative path to the binary data file
- `binary-rw`: Read-write binary blob
  - `data_file`: specifies the cart-relative path to the binary data file

## Examples

- test/audio_synthesis
  - Shows how to use the sound output peripheral with basic fixed-point audio synthesis
- test/cart_test
  - Shows the basic structure of a "cart", along with a basic program
- test/floating_point
  - Demos the F extension working
- test/math_accel
  - Shows how to use the math accelerator (currently not implemented in the HAL, but raw MMIO works)
- test/mmfb
  - Shows how to set up the memory-mapped framebuffer for basic CPU-based drawing
- test/mtimer
  - Shows how to use the mtimer to delay the program
- test/second_core
  - Shows how to use the second core
- boot_rom/boot_rom
  - An example of using many features at once, including cartridge hot-swapping
  
## RVFM Usage

Currently, RVFM is launched via the command line, with the program binary being the single argument:

`target/release/rvfm_main boot_rom/boot_rom.elf`

When fully implemented however, RVFM will start as a normal GUI app, and automatically load the boot rom program. The boot rom will then enumerate cartridges in the RVFM catridge directory, and allow for graphical cartridge selection.