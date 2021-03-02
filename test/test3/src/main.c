#include <stdint.h>

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

void wfi() {
	__asm__ volatile("wfi");
}

#define CORE2_CONTROLLER_START_ADDRESS *((volatile uint32_t *) 0xF0040000)
#define CORE2_CONTROLLER_RUN *((volatile uint32_t *) 0xF0040004)
#define CORE2_CONTROLLER_STATUS *((volatile uint32_t *) 0xF0040008)

extern void core2_start();

void main() {
	CORE2_CONTROLLER_START_ADDRESS = (uint32_t) &core2_start;
	CORE2_CONTROLLER_RUN = 1;
	wfi();
}

void core2_main() {
	const char * message = "Hello world from core 2!";
	debug_print_msg(message, str_len(message));
	wfi();
}