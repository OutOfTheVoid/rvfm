riscv32-unknown-elf-g++ test/test1/test1.s -ffreestanding -c -o test/test1/out.o -march=rv32im -mno-relax -mexplicit-relocs
riscv32-unknown-elf-ld test/test1/out.o -T test/test1/link.ld -o test/test1/out.elf
riscv32-unknown-elf-elf2bin --input test/test1/out.elf --output test/test1/out.bin 