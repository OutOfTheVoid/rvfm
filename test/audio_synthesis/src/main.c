#include <stdint.h>
#include <interrupt.h>
#include <debug_print.h>
#include <core2.h>
#include <sound.h>

volatile int sound_interrupt_waiting = 0;

void ATTR_INTERRUPT interrupt_handler() {
	if (sound_interrupt_state()) {
		sound_interrupt_waiting = 0;
		sound_interrupt_ack();
	}
	clear_pending_interrupts();
}

void sound_interrupt_wait() {
	sound_interrupt_waiting = 1;
	while(sound_interrupt_waiting) {
		wfi();
	}
}

void init_sound_interrupt() {
	disable_interrupts();
	set_interrupt_handler(interrupt_handler);
	clear_pending_interrupts();
	enable_interrupts();
	enable_external_interrupts();
	sound_interrupt_enable();
}

void main() {
	start_core2();
	while(1) {
		wfi();
		debug_print_msg("core1 wfi passed", 16);
	}
}

#define SAMPLE_RATE 48000
#define AMPLITUDE 5000

// generates a triangle wave based on a phase variable and frequency
int get_triangle_wave(int * ph, int frequency) {
	int phase = *ph;
	phase += frequency;
	phase %= SAMPLE_RATE;
	*ph = phase;
	int t = phase / (SAMPLE_RATE/2000);
	if (t > 1000) {
		return 1500 - t;
	} else {
		return t - 500;
	}
}

void core2_main() {
	const char * message = "Hello world from core 2!";
	debug_print_msg(message, str_len(message));
	
	int phase_1 = 0;
	int phase_2 = 0;
	int phase_3 = 0;
	
	int16_t buffer[SOUND_FRAME_SIZE * SOUND_CHANNEL_COUNT];
	for (int i = 0; i < SOUND_FRAME_SIZE * SOUND_CHANNEL_COUNT; i ++) {
		buffer[i] = 0;
	}
	
	init_sound_interrupt();
	sound_enable();
	while (1) {
		sound_interrupt_wait();
		sound_frame_submit(buffer);
		for (int i = 0; i < SOUND_FRAME_SIZE; i ++) {
			int s = 
				get_triangle_wave(& phase_1, 262) + // C4
				get_triangle_wave(& phase_2, 330) + // E4
				get_triangle_wave(& phase_3, 392);  // G4
			buffer[i * 2] = s;
			buffer[i * 2 + 1] = s;
		}
	}
	disable_interrupts();
	clear_pending_interrupts();
	wfi();
}