#include <stdint.h>
#include <interrupt.h>
#include <gpu/gpu.h>
#include <gpu/mmfb.h>
#include <gpu/vsync.h>

volatile int frame = 0;

void __attribute__((interrupt("machine"))) interrupt_handler () {
	gpu_vsync_wait_interrupt_callback();
	clear_pending_interrupts();
}

volatile uint32_t * mmfb;

void draw_square(int x) {
	for (int y = 118; y < 138; y ++) {
		for (int x_off = 0; x_off < 20; x_off ++) {
			mmfb[(y << 8) + x + x_off] = 0x0000FFFF;
		}
	}
}

#define CLEAR_COLOR 0x44AADD

void setup_gpu() {
	mmfb = (volatile uint32_t *) 0x0FFA0000;
	gpu_mmfb_set_ptr(mmfb);
	gpu_set_mode(GpuMode_RawFramebuffer);
	gpu_mmfb_clear(mmfb, CLEAR_COLOR);
	gpu_mmfb_present();
	gpu_vsync_wait_init();
}

void draw() {
	gpu_mmfb_clear(mmfb, CLEAR_COLOR);
}

void main() {
	set_interrupt_handler(&interrupt_handler);
	enable_external_interrupts();
	enable_interrupts();
	
	setup_gpu();
	
	while (1) {
		gpu_vsync_wait();
		draw();
		gpu_mmfb_present();
		frame ++;
	}
}