#ifndef RVFM_SOUND_H
#define RVFM_SOUND_H

#include "common.h"

C_START

#define SOUND_BASE 0xF0050000
#define SOUND_ENABLE *((volatile uint32_t *) (SOUND_BASE | 0x0000))
#define SOUND_FRAME_COUNT *((volatile uint32_t *) (SOUND_BASE | 0x0004))
#define SOUND_INTERRUPT_ENABLE *((volatile uint32_t *) (SOUND_BASE | 0x0008))
#define SOUND_FRAME_PTR *((volatile uint32_t *) (SOUND_BASE | 0x000C))
#define SOUND_TRIGGER_COPY *((volatile uint32_t *) (SOUND_BASE | 0x0010))

#define SOUND_INTERRUPT_STATE *((volatile uint32_t *) 0xF0030004)

#define SOUND_FRAME_SIZE 256
#define SOUND_SAMPLE_RATE 48000
#define SOUND_CHANNEL_COUNT 2

static inline void sound_enable() {
	SOUND_ENABLE = 1;
}

static inline void sound_disable() {
	SOUND_ENABLE = 0;
}

static inline void sound_interrupt_enable() {
	SOUND_INTERRUPT_ENABLE = 1;
}

static inline void sound_interrupt_disable() {
	SOUND_INTERRUPT_ENABLE = 0;
}

static inline bool sound_interrupt_state() {
	return SOUND_INTERRUPT_STATE;
}

static inline void sound_interrupt_ack() {
	SOUND_INTERRUPT_STATE = 0;
}

static inline int get_sound_frame_number() {
	return SOUND_FRAME_COUNT;
}

static inline int16_t sound_frame_submit(int16_t * frame) {
	SOUND_FRAME_PTR = (uint32_t) frame;
	SOUND_TRIGGER_COPY = 1;
}

C_END

#endif
