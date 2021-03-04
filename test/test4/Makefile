LD = riscv32-unknown-elf-ld
CC = riscv32-unknown-elf-gcc
AS = riscv32-unknown-elf-gcc
RM = rm -rf
MKDIR = mkdir -p

C_OPT = -O2
C_FLAGS = -ffreestanding -march=rv32im -mabi=ilp32 -c $(C_OPT)
AS_FLAGS = -ffreestanding -march=rv32im  -mabi=ilp32 -c 

LINKER_SCRIPT = link.ld

all: cart.elf

cart.elf: obj/main.o obj/crt0.o init
	$(LD) -T $(LINKER_SCRIPT) -nostartfiles obj/crt0.o obj/main.o -o cart.elf

obj/crt0.o: src/crt0.s init
	$(AS) src/crt0.s $(AS_FLAGS) -o obj/crt0.o
	
obj/main.o: src/main.c init
	$(CC) src/main.c $(C_FLAGS) -o obj/main.o

clean:
	-$(RM) obj/*
	-$(RM) cart.elf
	
obj:
	-$(MKDIR) obj
	
.PHONY: all clean init
