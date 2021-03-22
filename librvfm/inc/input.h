#ifndef RVFM_INPUT_H
#define RVFM_INPUT_H

#include "common.h"

#define INPUT_KEY_EVENTS_0_TO_31 *((volatile uint32_t *) 0xF0090000)
#define INPUT_KEY_EVENTS_32_TO_63 *((volatile uint32_t *) 0xF0090004)
#define INPUT_KEY_EVENTS_64_TO_95 *((volatile uint32_t *) 0xF0090008)
#define INPUT_KEY_STATES_0_TO_31 *((volatile uint32_t *) 0xF009000C)
#define INPUT_KEY_STATES_32_TO_63 *((volatile uint32_t *) 0xF0090010)
#define INPUT_KEY_STATES_64_TO_95 *((volatile uint32_t *) 0xF0090014)

#define INPUT_MOUSE_EVENTS *((volatile uint32_t *) 0xF0090020)
#define INPUT_MOUSE_X *((volatile uint32_t *) 0xF0090024)
#define INPUT_MOUSE_Y *((volatile uint32_t *) 0xF0090028)

typedef enum {
	InputKey_Escape = 0,
	InputKey_Back = 1,
	InputKey_Return = 3,
	InputKey_Up = 4,
	InputKey_Down = 5,
	InputKey_Left = 6,
	InputKey_Right = 7,
	InputKey_Delete = 8,
	InputKey_Tab = 9,
	InputKey_Space = 10,
	InputKey_Apostrophe = 11,
	InputKey_Semicolon = 12,
	InputKey_LBracket = 13,
	InputKey_RBracket = 14,
	InputKey_Backslash = 15,
	InputKey_Minus = 16,
	InputKey_Equals = 17,
	InputKey_Slash = 17,
	
	InputKey_F1 = 18,
	InputKey_F2 = 19,
	InputKey_F3 = 20,
	InputKey_F4 = 21,
	InputKey_F5 = 22,
	InputKey_F6 = 23,
	InputKey_F7 = 24,
	InputKey_F8 = 25,
	InputKey_F9 = 26,
	InputKey_F10 = 28,
	InputKey_F11 = 29,
	InputKey_F12 = 30,
	
	InputKey_A = 31,
	InputKey_B = 32,
	InputKey_C = 33,
	InputKey_D = 34,
	InputKey_E = 35,
	InputKey_F = 36,
	InputKey_G = 37,
	InputKey_H = 38,
	InputKey_I = 39,
	InputKey_J = 40,
	InputKey_K = 41,
	InputKey_L = 42,
	InputKey_M = 43,
	InputKey_N = 44,
	InputKey_O = 45,
	InputKey_P = 46,
	InputKey_Q = 47,
	InputKey_R = 48,
	InputKey_S = 49,
	InputKey_T = 50,
	InputKey_U = 51,
	InputKey_V = 52,
	InputKey_W = 53,
	InputKey_X = 54,
	InputKey_Y = 55,
	InputKey_Z = 56,
	
	InputKey_Key0 = 56,
	InputKey_Key1 = 57,
	InputKey_Key2 = 58,
	InputKey_Key3 = 59,
	InputKey_Key4 = 60,
	InputKey_Key5 = 61,
	InputKey_Key6 = 62,
	InputKey_Key7 = 63,
	InputKey_Key8 = 64,
	InputKey_Key9 = 65,
} InputKey;

bool input_key_down(InputKey key) {
	if (key < 32) {
		return (INPUT_KEY_STATES_0_TO_31 & (1 << key)) != 0;
	} else if (key < 64) {
		return (INPUT_KEY_STATES_32_TO_63 & (1 << (key - 32))) != 0;
	} else {
		return (INPUT_KEY_STATES_64_TO_95 & (1 << (key - 64))) != 0;
	}
}

#endif
