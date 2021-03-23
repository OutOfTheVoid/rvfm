#include <stdint.h>
#include <debug_print.h>
#include <interrupt.h>
#include <cart_loader.h>

void main() {
	volatile uint32_t cart_loader_completion;
	cart_loader_setup_data_slot_fs(0, "test.txt", false, &cart_loader_completion);
	while (! cart_loader_poll_completion(& cart_loader_completion)) {};
	if (cart_loader_completion_is_error(cart_loader_completion)) {
		debug_print_string("cart_loader_setup_data_slot_fs completed with error:");
		debug_print_u32(cart_loader_completion);
		wfi();
	}
	
	volatile uint32_t extents;
	cart_loader_get_data_extents(0, &extents, &cart_loader_completion);
	while (! cart_loader_poll_completion(& cart_loader_completion)) {};
	if (cart_loader_completion_is_error(cart_loader_completion)) {
		debug_print_string("cart_loader_get_data_extents completed with error:");
		debug_print_u32(cart_loader_completion);
		wfi();
	}
	
	debug_print_string("data.txt extents: ");
	debug_print_u32(extents);
	
	char buffer[32];
	volatile uint32_t read_size;
	cart_loader_read_data(0, 0, 32, (void *) buffer, & read_size, & cart_loader_completion);
	while (! cart_loader_poll_completion(& cart_loader_completion)) {};
	if (cart_loader_completion_is_error(cart_loader_completion)) {
		debug_print_string("cart_loader_read_data completed with error:");
		debug_print_u32(cart_loader_completion);
		wfi();
	}
	debug_print_string("data.txt read size: ");
	debug_print_u32(read_size);
	buffer[read_size] = 0;
	debug_print_string("data.txt contents: ");
	debug_print_string((const char *) buffer);
	
	wfi();
}