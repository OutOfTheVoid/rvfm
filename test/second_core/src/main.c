#include <stdint.h>
#include <debug_print.h>
#include <interrupt.h>
#include <core2.h>

void main() {
	start_core2();
	wfi();
}

void core2_main() {
	const char * message = "Hello world from core 2!";
	debug_print_msg(message, str_len(message));
	wfi();
}