#include <stdint.h>
#include <sound.h>
#include <interrupt.h>

#include "note_play.h"


volatile int sound_interrupt_waiting = 0;

void ATTR_INTERRUPT core2_interrupt_handler() {
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

void init_core2_interrupts() {
	set_interrupt_handler(&core2_interrupt_handler);
	enable_external_interrupts();
	sound_interrupt_enable();
	clear_pending_interrupts();
	enable_interrupts();
}


#define FREQ_C4 262
#define FREQ_E4 330
#define FREQ_G4 392
#define FREQ_C5 524

const note_event_t startup_melody[] = {
	NOTE_DELAY(100),
	NOTE_ON(0, FREQ_C4),
	NOTE_DELAY(100),
	NOTE_OFF(0),
	NOTE_DELAY(100),
	NOTE_ON(0, FREQ_E4),
	NOTE_DELAY(100),
	NOTE_OFF(0),
	NOTE_DELAY(100),
	NOTE_ON(0, FREQ_G4),
	NOTE_DELAY(100),
	NOTE_OFF(0),
	NOTE_DELAY(100),
	NOTE_ON(0, FREQ_C5),
	NOTE_DELAY(100),
	NOTE_OFF(0)
};

void core2_main() {
	int16_t buffer[1024];
	for (int i = 0; i < 1024; i ++) {
		buffer[i] = 0;
	}
	
	note_play_state_t note_play_state;
	note_play_init(& note_play_state, startup_melody, sizeof(startup_melody) / sizeof(note_event_t));
	
	init_core2_interrupts();
	sound_enable();
	int done = 0;
	while (! done) {
		sound_interrupt_wait();
		for (int i = 0; i < SOUND_FRAME_SIZE; i ++) {
			int s = note_play_sample(&note_play_state, &done);
			buffer[i * 2] = s;
			buffer[i * 2 + 1] = s;
		}
		sound_frame_submit(buffer);
	}
	sound_disable();
	disable_interrupts();
	clear_pending_interrupts(); 
	wfi();
}

