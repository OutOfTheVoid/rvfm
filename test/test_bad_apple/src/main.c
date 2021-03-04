#include <stdint.h>
#include "test_audio.h"
#include "interrupts.h"

#define DEBUG_IO_MSG_ADDRESS *((volatile uint32_t *)0xF0000000)
#define DEBUG_IO_MSG_LENGTH *((volatile uint32_t *)0xF0000004)
#define DEBUG_IO_WRITE *((volatile uint32_t *)0xF0000008)

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
	DEBUG_IO_MSG_LENGTH = value;
	DEBUG_IO_WRITE = 1;
}

#define CORE2_CONTROLLER_START_ADDRESS *((volatile uint32_t *) 0xF0040000)
#define CORE2_CONTROLLER_RUN *((volatile uint32_t *) 0xF0040004)
#define CORE2_CONTROLLER_STATUS *((volatile uint32_t *) 0xF0040008)

extern void core2_start();

#define SOUND_BASE 0xF0050000
#define SOUND_ENABLE *((volatile uint32_t *) (SOUND_BASE | 0x0000))
#define SOUND_FRAME_COUNT *((volatile uint32_t *) (SOUND_BASE | 0x0004))
#define SOUND_INTERRUPT_ENABLE *((volatile uint32_t *) (SOUND_BASE | 0x0008))
#define SOUND_FRAME_PTR *((volatile uint32_t *) (SOUND_BASE | 0x000C))
#define SOUND_TRIGGER_COPY *((volatile uint32_t *) (SOUND_BASE | 0x0010))

#define SOUND_INTERRUPT_STATE *((volatile uint32_t *) 0xF0030004)

volatile int sound_frame = 0;

void ATTR_INTERRUPT interrupt_handler() {
	if (SOUND_INTERRUPT_STATE != 0) {
		sound_frame = SOUND_FRAME_COUNT;
		SOUND_INTERRUPT_STATE = 0;
	}
	clear_pending_interrupts();
}

int get_sound_frame() {
	disable_interrupts();
	int frame = sound_frame;
	enable_interrupts();
	return frame;
}

void sound_interrupt_wait() {
	static int last_frame = 0;
	int current_frame = get_sound_frame();
	while(current_frame == last_frame) {
		wfi();
		current_frame = get_sound_frame();
	}
	last_frame = current_frame;
}

void init_sound_interrupt() {
	disable_interrupts();
	set_interrupt_handler(interrupt_handler);
	clear_pending_interrupts();
	enable_interrupts();
	enable_external_interrupts();
	SOUND_INTERRUPT_ENABLE = 1;
}

void main() {
	/*CORE2_CONTROLLER_START_ADDRESS = (uint32_t) &core2_start;
	CORE2_CONTROLLER_RUN = 1;*/
	wfi();
}

void core2_main() {
	int16_t silece[256];
	for (int i = 0; i < 256; i ++) {
		silece[i] = 0;
	} 
	const char * message = "Hello world from core 2!";
	debug_print_msg(message, str_len(message));
	init_sound_interrupt();
	SOUND_ENABLE = 0;
	/*while (1) {
		int frame = get_sound_frame();
		if (frame >= (test_audio_len / 1024)) {
			SOUND_ENABLE = 0;
			break;
		}
		SOUND_FRAME_PTR = (uint32_t) (void *) (&test_audio[frame * 1024]);
		SOUND_TRIGGER_COPY = 1;
		sound_interrupt_wait();
	}
	disable_interrupts();
	clear_pending_interrupts();*/
	wfi();
}