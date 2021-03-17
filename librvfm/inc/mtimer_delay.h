#ifndef MTIMER_DELAY_H
#define MTIMER_DELAY_H

#include "mtimer.h"

typedef struct {
	volatile int int_fired;
} mtimer_delay_context_t;

static inline void mtimer_delay_interrupt_call(volatile mtimer_delay_context_t * delay_context) {
	if (mtimer_interrupt_pending()) {
		delay_context->int_fired = 1;
		mtimer_interrupt_ack();
	}
}

static inline void mtimer_delay(volatile mtimer_delay_context_t * delay_context, int ms) {
	disable_interrupts();
	delay_context->int_fired = 0;
	mtimer_schedule(ms);
	mtimer_enable_interrupt();
	enable_interrupts();
	while (delay_context->int_fired == 0) {
		wfi();
	}
}

#endif
