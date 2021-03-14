LD = riscv32-unknown-elf-ld
CC = riscv32-unknown-elf-gcc
AS = riscv32-unknown-elf-gcc
RM = rm -rf
MKDIR = mkdir -p

C_OPT = -O2
C_FLAGS = -ffreestanding -march=rv32im -mabi=ilp32 -c $(C_OPT)
AS_FLAGS = -ffreestanding -march=rv32im  -mabi=ilp32 -c 

librvfm: librvfm.o



clean:
	-$(RM) obj/*
	-$(RM) librvfm.o
	
obj:
	-$(MKDIR) obj
	
.PHONY: all clean init
