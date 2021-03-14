#include <stdint.h>

#include <interrupt.h>
#include <mtimer.h>
#include <debug_print.h>

volatile int timer_interrupt_fired = 0;

void ATTR_INTERRUPT interrupt_handler() {
	if (mtimer_interrupt_pending()) {
		timer_interrupt_fired = 1;
	}
	clear_pending_interrupts();
}

void timer_init() {
	set_interrupt_handler(& interrupt_handler);
}

void timer_delay(int ms) {
	disable_interrupts();
	timer_interrupt_fired = 0;
	mtimer_schedule(ms);
	mtimer_enable_interrupt();
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