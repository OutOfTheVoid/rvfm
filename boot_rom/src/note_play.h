#ifndef NOTE_PLAY_H
#define NOTE_PLAY_H

#include <stdint.h>
#include <sound.h>

#define NOTE_VOICE_COUNT 4

typedef enum {
	NoteEvent_On,
	NoteEvent_Off,
	NoteEvent_Delay
} node_event_type_t;

typedef struct {
	int frequency;
	int channel;
} note_params_t;

typedef struct {
	int delay_ms;
} delay_params_t;

typedef struct {
	node_event_type_t type; // NoteOn     NoteOff  Delay
	int param0;             // channel    channel  delay_ms
	int param1;             // frequency  -        -
} note_event_t;

#define NOTE_ON(channel, frequency) {NoteEvent_On, channel, frequency}
#define NOTE_OFF(channel) {NoteEvent_Off, channel, 0}
#define NOTE_DELAY(delay_ms) {NoteEvent_Delay, delay_ms, 0}

typedef struct {
	int phase;
	int frequency;
	bool on;
} note_voice_t;

typedef struct {
	const note_event_t * events;
	int event_count;
	int event_index;
	int end_sample;
	int current_sample;
	note_voice_t voices[NOTE_VOICE_COUNT];
} note_play_state_t;

// generates a triangle wave based on a phase variable and frequency
static int gen_triangle_wave(int * ph, int frequency) {
	int phase = *ph;
	phase += frequency;
	phase %= SOUND_SAMPLE_RATE;
	*ph = phase;
	int t = phase / (SOUND_SAMPLE_RATE/2000);
	if (t > 1000) {
		return 1500 - t;
	} else {
		return t - 500;
	}
}

static inline void note_play_init(note_play_state_t * state, const note_event_t * events, int event_count) {
	state->events = events;
	state->event_count = event_count;
	state->event_index = 0;
	state->current_sample = 0;
	state->end_sample = -1;
	for (int i = 0; i < NOTE_VOICE_COUNT; i ++) {
		state->voices[i].on = false;
		state->voices[i].phase = 0;
	}
}

static inline int16_t note_play_sample(note_play_state_t * state, int * done) {
	if (state->event_index == -1) {
		*done = 1;
		return 0;
	}
	bool events_done = false;
	while (! events_done) {
		const note_event_t * event = &state->events[state->event_index];
		switch (event->type) {
			case NoteEvent_On: {
				int voice = event->param0;
				int frequency = event->param1;
				state->voices[voice].on = true;
				state->voices[voice].phase = 0;
				state->voices[voice].frequency = frequency;
				state->event_index ++;
			} break;
			case NoteEvent_Off: {
				int voice = event->param0;
				int frequency = event->param1;
				state->voices[voice].on = false;
				state->voices[voice].phase = 0;
				state->voices[voice].frequency = frequency;
				state->event_index ++;
			} break;
			case NoteEvent_Delay: {
				if (state->end_sample != -1) {
					if (state->current_sample >= state->end_sample) {
						state->event_index ++;
						state->end_sample = -1;
					} else {
						events_done = true;
					}
				} else {
					int delay_ms = event->param0;
					int delay_samples = delay_ms * (SOUND_SAMPLE_RATE / 1000);
					state->end_sample = state->current_sample + delay_samples;
					events_done = true;
				}
			} break;
		}
		if (state->event_index >= state->event_count) {
			state->event_index = -1;
			*done = 1;
			return 0;
		}
	}
	state->current_sample ++;
	
	int16_t sample = 0;
	for (int i = 0; i < NOTE_VOICE_COUNT; i ++) {
		if (state->voices[i].on) {
			sample += gen_triangle_wave(& state->voices[i].phase, state->voices[i].frequency);
		}
	}
	return sample;
}

#endif
