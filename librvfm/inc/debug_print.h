#ifndef RVFM_DEBUG_PRINT_H
#define RVFM_DEBUG_PRINT_H

#include "common.h"

C_START

#define DEBUG_PRINT_WRITE_TYPE_STRING 0
#define DEBUG_PRINT_WRITE_TYPE_U32 1
#define DEBUG_PRINT_WRITE_TYPE_F32 2
#define DEBUG_PRINT_WRITE_TYPE_U32H 3

#define DEBUG_PRINT_MSG_ADDRESS *((volatile uint32_t *)0xF0000000)
#define DEBUG_PRINT_MSG_LENGTH *((volatile uint32_t *)0xF0000004)
#define DEBUG_PRINT_WRITE *((volatile uint32_t *)0xF0000008)

static inline int32_t str_len(const char * string) {
	int32_t count = 0;
	while (string[count] != '\0') {
		count ++;
	}
	return count;
}

static inline void debug_print_msg(const char * message, uint32_t length) {
	DEBUG_PRINT_MSG_ADDRESS = (uint32_t) message;
	DEBUG_PRINT_MSG_LENGTH = length;
	DEBUG_PRINT_WRITE = DEBUG_PRINT_WRITE_TYPE_STRING;
}

static inline void debug_print_string(const char * str) {
	debug_print_msg(str, str_len(str));
}

static inline void debug_print_u32(uint32_t value) {
	DEBUG_PRINT_MSG_ADDRESS = value;
	DEBUG_PRINT_WRITE = DEBUG_PRINT_WRITE_TYPE_U32;
}

static inline void debug_print_f32(float value) {
	DEBUG_PRINT_MSG_ADDRESS = *((uint32_t *) &value);
	DEBUG_PRINT_WRITE = DEBUG_PRINT_WRITE_TYPE_F32;
}

C_END

#endif
