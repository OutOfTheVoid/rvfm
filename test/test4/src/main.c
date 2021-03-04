#include <stdint.h>

#define DEBUG_IO_MSG_ADDRESS *((volatile uint32_t *)0xF0000000)
#define DEBUG_IO_MSG_LENGTH *((volatile uint32_t *)0xF0000004)
#define DEBUG_IO_WRITE *((volatile uint32_t *)0xF0000008)
#define MESSAGE_IO_WRITE_TYPE_STRING 0
#define MESSAGE_IO_WRITE_TYPE_U32 1
#define MESSAGE_IO_WRITE_TYPE_F32 2

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
	DEBUG_IO_WRITE = MESSAGE_IO_WRITE_TYPE_STRING;
}

void debug_print_u32(uint32_t value) {
	DEBUG_IO_MSG_ADDRESS = value;
	DEBUG_IO_WRITE = MESSAGE_IO_WRITE_TYPE_U32;
}

void debug_print_f32(float value) {
	DEBUG_IO_MSG_ADDRESS = *((uint32_t *) &value);
	DEBUG_IO_WRITE = MESSAGE_IO_WRITE_TYPE_F32;
}

void wfi() {
	__asm__ volatile("wfi");
}

void main() {
	debug_print_f32(0.5f);
	wfi();
}