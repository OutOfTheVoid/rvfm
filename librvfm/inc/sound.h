#ifndef RVFM_SOUND_H
#define RVFM_SOUND_H

#include "common.h"

C_START

#define SOUND_BASE 0xF0050000
#define SOUND_ENABLE *((volatile uint32_t *) (SOUND_BASE | 0x0000))
#define SOUND_FIFO_LENGTH *((volatile uint32_t *) (SOUND_BASE | 0x0004))
#define SOUND_FIFO_INTERRUPT_ENABLE *((volatile uint32_t *) (SOUND_BASE | 0x0008))
#define SOUND_FIFO_FILL_PTR *((volatile uint32_t *) (SOUND_BASE | 0x000C))
#define SOUND_FIFO_FILL_TRIGGER *((volatile uint32_t *) (SOUND_BASE | 0x0010))
#define SOUND_FIFO_LAST_FILL_COUNT *((volatile uint32_t *) (SOUND_BASE | 0x0014))

#define SOUND_FIFO_INTERRUPT_STATE *((volatile uint32_t *) 0xF0030004)

#define SOUND_FIFO_SIZE 512
#define SOUND_FIFO_MIN_FILL 256
#define SOUND_SAMPLE_RATE 48000
#define SOUND_CHANNEL_COUNT 2

static inline void sound_enable() {
	SOUND_ENABLE = 1;
}

static inline void sound_disable() {
	SOUND_ENABLE = 0;
}

static inline void sound_fifo_interrupt_enable() {
	SOUND_FIFO_INTERRUPT_ENABLE = 1;
}

static inline void sound_fifo_interrupt_disable() {
	SOUND_FIFO_INTERRUPT_ENABLE = 0;
}

static inline bool sound_fifo_interrupt_state() {
	return SOUND_FIFO_INTERRUPT_STATE;
}

static inline void sound_fifo_interrupt_ack() {
	SOUND_FIFO_INTERRUPT_STATE = 0;
}

static inline int16_t sound_fill_fifo(int16_t * frame, uint32_t size) {
	SOUND_FIFO_FILL_PTR = (uint32_t) frame;
	SOUND_FIFO_FILL_TRIGGER = size;
}

C_END

#endif
