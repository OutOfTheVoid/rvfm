#include <stdint.h>
#include <interrupt.h>
#include <debug_print.h>
#include <core2.h>
#include <sound.h>
#include <interrupt.h>
#include <mtimer.h>
#include <mtimer_delay.h>
#include <gpu/gpu.h>
#include <gpu/mmfb.h>
#include <gpu/vsync.h>
#include <blit.h>
#include <input.h>
#include <cart_loader.h>

volatile mtimer_delay_context_t delay_context;

void ATTR_INTERRUPT core1_interrupt_handler() {
	mtimer_delay_interrupt_call(& delay_context);
	gpu_vsync_wait_interrupt_callback();
	clear_pending_interrupts();
}

volatile uint32_t * mmfb;

#define CLEAR_COLOR 0x44AADD

void setup_gpu() {
	gpu_vsync_wait_init();
	gpu_set_mode(GpuMode_RawFramebuffer);
	
	mmfb = (volatile uint32_t *) 0x0FFA0000;
	gpu_mmfb_clear(mmfb, CLEAR_COLOR);
	gpu_mmfb_set_ptr(mmfb);
}

volatile cart_metadata_t cart_metadata;
volatile bool cart_metadata_loaded;

void draw_cart_icon(volatile cart_metadata_t * cart, int x, int y) {
	blit_buff_t cart_sprite_buff = {
		cart->icon_bitmap,
		64, 64
	};
	blit_buff_t mmfb_buff = {
		mmfb,
		256, 192
	};
	blit_sprite_cutout(& cart_sprite_buff, & mmfb_buff, x, y);
}

int x = 0;
int y = 0;

void draw() {
	gpu_mmfb_clear(mmfb, CLEAR_COLOR);
	if (cart_metadata_loaded) {
		draw_cart_icon(&cart_metadata, INPUT_MOUSE_X, INPUT_MOUSE_Y);
	}
}

void halt() {
	while(1) {
		disable_interrupts();
		wfi();
	}
}

uint32_t load_cart(uint32_t index) {
	volatile uint32_t load_error;
	cart_loader_load_cart(index, &load_error);
	while (! cart_loader_poll_completion(& load_error)) {
		mtimer_delay(&delay_context, 10);
	}
	return load_error;
}

void main() {
	cart_metadata_loaded = false;
	
	setup_gpu();
	draw();
	gpu_mmfb_present();
	
	start_core2();
	
	delay_context.int_fired = 0;
	set_interrupt_handler(&core1_interrupt_handler);
	enable_external_interrupts();
	enable_interrupts();
	
	volatile uint32_t enumerate_completion;
	cart_loader_begin_enumerate(& enumerate_completion);
	while(! cart_loader_poll_completion(& enumerate_completion)) {
		mtimer_delay(&delay_context, 1);
	}
	debug_print_string("Cart count: ");
	debug_print_u32(CART_LOADER_CART_COUNT);
	if (cart_loader_completion_is_error(enumerate_completion)) {
		debug_print_string("Cart enumerate produced an error!");
		halt();
	}
	
	volatile uint32_t metadata_completion;
	cart_loader_read_cart_metadata(0, & cart_metadata, & metadata_completion);
	while(! cart_loader_poll_completion(& metadata_completion)) {
		mtimer_delay(&delay_context, 1);
	}
	if (cart_loader_completion_is_error(metadata_completion)) {
		debug_print_string("Cart metadata load produced an error!");
		halt();
	}
	cart_metadata_loaded = true;
	
	debug_print_string("Cart 0 name: ");
	debug_print_string((const char *) cart_metadata.name);
	
	bool requested_load = false;
	volatile uint32_t load_error_completion = CART_LOADER_COMPLETION_RESULT_NONE;
	
	while(true) {
		if (input_key_down(InputKey_Space)) {
			load_cart(0);
		}
		draw();
		gpu_mmfb_present();
		gpu_vsync_wait();
	}
}
