#include <stdint.h>
#include "interrupts.h"

#define DEBUG_IO_MSG_ADDRESS *((volatile uint32_t *)0xF0000000)
#define DEBUG_IO_MSG_LENGTH *((volatile uint32_t *)0xF0000004)
#define DEBUG_IO_WRITE *((volatile uint32_t *)0xF0000008)

#define MTIMER_MTIME *((volatile uint32_t *)0xF0060000)
#define MTIMER_MTIME_H *((volatile uint32_t *)0xF0060004)
#define MTIMER_MTIME_ATOMIC_BUFF *((volatile uint32_t *)0xF0060008)
#define MTIMER_MTIME_H_ATOMIC_BUFF *((volatile uint32_t *)0xF006000C)
#define MTIMER_MTIME_ATOMIC_READ_TRIGGER *((volatile uint32_t *)0xF0060010)
#define MTIMER_MTIME_ATOMIC_WRITE_TRIGGER *((volatile uint32_t *)0xF0060014)
#define MTIMER_MTIME_ATOMIC_SWAP_TRIGGER *((volatile uint32_t *)0xF0060018)

#define MTIMER_MTIMECMP *((volatile uint32_t *)0xF0060020)
#define MTIMER_MTIMECMP_H *((volatile uint32_t *)0xF0060024)
#define MTIMER_MTIMECMP_ATOMIC_BUFF *((volatile uint32_t *)0xF0060028)
#define MTIMER_MTIMECMP_H_ATOMIC_BUFF *((volatile uint32_t *)0xF006002C)
#define MTIMER_MTIMECMP_ATOMIC_READ_TRIGGER *((volatile uint32_t *)0xF0060030)
#define MTIMER_MTIMECMP_ATOMIC_WRITE_TRIGGER *((volatile uint32_t *)0xF0060034)
#define MTIMER_MTIMECMP_ATOMIC_SWAP_TRIGGER *((volatile uint32_t *)0xF0060038)

#define MTIMER_DUAL_ATOMIC_WRITE_TRIGGER *((volatile uint32_t *)0xF0060040)
#define MTIMER_DUAL_ATOMIC_SWAP_TRIGGER *((volatile uint32_t *)0xF0060044)


int32_t str_len(const char * string) {
	int32_t count = 0;
	while (string[count] != '\0') {
		count ++;
	}
	return count;
}

void debug_print_msg(const char * message, uint32_t length) {
	DEBUG_IO_MSG_ADDRESS = (uint32_t) message;
	DEBUG_IO_MSG_LENGTH = length;
	DEBUG_IO_WRITE = 0;
}

void debug_print_u32(uint32_t value) {
	DEBUG_IO_MSG_ADDRESS = value;
	DEBUG_IO_WRITE = 1;
}

volatile int timer_interrupt_fired = 0;

void ATTR_INTERRUPT interrupt_handler() {
	if (get_mip() & MIP_MTIP) {
		timer_interrupt_fired = 1;
		MTIMER_MTIMECMP_ATOMIC_BUFF = 0xFFFFFFFF;
		MTIMER_MTIMECMP_H_ATOMIC_BUFF = 0xFFFFFFFF;
		MTIMER_MTIMECMP_ATOMIC_WRITE_TRIGGER = 1;
	}
	clear_pending_interrupts();
}

void timer_init() {
	set_interrupt_handler(& interrupt_handler);
}

void timer_delay(int ms) {
	disable_interrupts();
	timer_interrupt_fired = 0;
	MTIMER_MTIME_H_ATOMIC_BUFF = 0;
	MTIMER_MTIME_ATOMIC_BUFF = 0;
	MTIMER_MTIMECMP_H_ATOMIC_BUFF = 0;
	MTIMER_MTIMECMP_ATOMIC_BUFF = ms;
	MTIMER_DUAL_ATOMIC_WRITE_TRIGGER = 1;
	timer_interrupt_fired = 0;
	enable_timer_interrupt();
	enable_interrupts();
	while (! timer_interrupt_fired) {
		wfi();
	}
}

void main() {
	timer_init();
	debug_print_msg("Hello, world!", 13);
	int i = 0;
	while (1) {
		debug_print_u32(i);
		i ++;
		timer_delay(1000);
	}
}