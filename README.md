# RVFM: Risc-V Fun Machine

RVFM is a virtual console, including an RV32IMACF emulator written entirely in Rust.

## Specs
- RV32IMACF (incomplete)
  - Dual core, eventually (seperate host threads)
  - Interpreted (though JIT is not out of the question)
- Sequentially consistent memory
  - Implemented through page-granularity locking on RAM
- Hardware accelerated GPU (incomplete)
  - Implemented using wgpu-rs
  - 256 x 192 native display framebuffer
  - Supports raw memory-mapped ARGB8888 framebuffer
- DSP-DMA Peripheral
  - supports simple, fast simd/memory transfer
- ELF based "catridges"
  
## Examples
- C examples in test directory
  - based on current implementation work
  - undocumented
  - include linker scripts and makefiles
