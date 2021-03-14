#include <stdint.h>
#include <debug_print.h>
#include <interrupt.h>

void test_add(float a, float b) {
	debug_print_string("adding: ");
	debug_print_f32(a);
	debug_print_f32(b);
	debug_print_string("result: ");
	float result = a + b;
	debug_print_f32(result);
}

void main() {
	test_add(0.5f, 1.0f);
	wfi();
}