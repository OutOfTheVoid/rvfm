#include <stdint.h>
#include <debug_print.h>
#include <interrupt.h>

void main() {
	debug_print_string("hello world!");
	wfi();
}