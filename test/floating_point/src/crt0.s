.extern _stack_top
.extern main

.equ CSR_PANIC, 0xBFF

.section .init_text
.global _start
_start:
	.cfi_startproc
	.cfi_undefined ra
	.option push
	.option norelax
	la gp, __global_pointer$
	.option pop
	la sp, _stack_top
	mv s0, sp
	li a0, 0xF0000000
	la a1, init_msg
	sw a1, 0(a0)
	li a1, 20
	sw a1, 4(a0)
	sw zero, 8(a0)
	call main
	j panic
	.cfi_endproc
	
.section .text
panic:
	csrrw zero, CSR_PANIC, zero
	
.section .rodata
init_msg:
	.ascii "RVFM cart running..."
	