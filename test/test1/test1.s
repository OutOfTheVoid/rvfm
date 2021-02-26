
.extern _stack_top
.global _start

.equ DEBUG_BASE, 0xF0000000
.equ DEBUG_OFF_MSG_ADDR, 0
.equ DEBUG_OFF_MSG_SIZE, 4
.equ DEBUG_OFF_WRITE, 8
.equ DEBUG_OFF_STATUS, 12

.section .init_text
_start:
	la t0, message
	li t1, DEBUG_BASE
	sw t0, DEBUG_OFF_MSG_ADDR(t1)
	li t0, 13
	sw t0, DEBUG_OFF_MSG_SIZE(t1)
	sw t0, DEBUG_OFF_WRITE(t1)
	j panic
	
.section .text
panic:
	csrrw x0, 0xBFF, x0
	
.section .data
message:
	.string "hello, world!\n"
	