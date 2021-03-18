#include <stdint.h>
#include <interrupt.h>
#include <debug_print.h>
#include <core2.h>
#include <sound.h>
#include <interrupt.h>
#include <mtimer.h>
#include <mtimer_delay.h>
#include <gpu/gpu.h>

#include "cart_loader.h"

volatile mtimer_delay_context_t delay_context;

void ATTR_INTERRUPT core1_interrupt_handler() {
	mtimer_delay_interrupt_call(& delay_context);
	clear_pending_interrupts();
}

void main() {
	//start_core2();
	
	delay_context.int_fired = 0;
	set_interrupt_handler(&core1_interrupt_handler);
	
	volatile uint32_t enumerate_completion;
	cart_loader_begin_enumerate(& enumerate_completion);
	while(! cart_loader_poll_completion(& enumerate_completion)) {
		mtimer_delay(&delay_context, 1);
	}
	debug_print_string("Cart count: ");
	debug_print_u32(CART_LOADER_CART_COUNT);
	if (cart_loader_completion_is_error(enumerate_completion)) {
		debug_print_string("Cart enumerate produced an error!");
	}
	wfi();
}
