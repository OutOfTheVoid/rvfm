#include <stdint.h>
#include <dspdma.h>
#include <interrupt.h>
#include <debug_print.h>

#define GPU_RAW_FRAMEBUFFER ((volatile uint32_t *) 0x2000000)

#define GPU_MODE_SET *((volatile uint32_t *) 0xF0010000)
#define GPU_PRESENT_MMFB *((volatile uint32_t *) 0xF0010004)
#define GPU_VSYNC_INT_ENABLE *((volatile uint32_t *) 0xF0010008)

#define GPU_MODE_RAW_FRAMEBUFFER 1

#define GPU_SYNC_INTERRUPT_STATE *((volatile uint32_t *) 0xF0030000)

uint32_t get_gpu_sync_interrupt_state() {
	return GPU_SYNC_INTERRUPT_STATE;
}

void clear_gpu_sync_interrupt() {
	GPU_SYNC_INTERRUPT_STATE = 0;
}

volatile int frame = 0;

void __attribute__((interrupt("machine"))) interrupt_handler () {
	clear_pending_interrupts();
	if (get_gpu_sync_interrupt_state()) {
		clear_gpu_sync_interrupt();
		frame ++;
	}
}

uint32_t get_time() {
	uint32_t val = 0;
	__asm__ __volatile__("csrr %0, 0xC01" : "=r"(val) :);
	return val;
}

void draw_square(int x) {
	for (int y = 118; y < 138; y ++) {
		for (int x_off = 0; x_off < 20; x_off ++) {
			GPU_RAW_FRAMEBUFFER[(y << 8) + x + x_off] = 0x0000FFFF;
		}
	}
}

void setup_vsync_interrupt() {
	disable_interrupts();
	set_interrupt_handler(interrupt_handler);
	clear_pending_interrupts();
	enable_interrupts();
	enable_external_interrupts();
	GPU_VSYNC_INT_ENABLE = 1;
}

int get_frame() {
	disable_interrupts();
	int f = frame;
	enable_interrupts();
	return f;
}

int last_frame = 0;

void vsync_interrupt_wait() {
	int current_frame = get_frame();
	while(current_frame == last_frame) {
		wfi();
		current_frame = get_frame();
	}
	last_frame = current_frame;
}

void dma_clear_framebuffer(uint32_t value) {
	dspdma_dest_mem32(0, (void *) GPU_RAW_FRAMEBUFFER, 4, DSPDMA_LOOP_INDEX_NEVER);
	dspdma_op_copy(0, dspdma_op_source_const(value), dspdma_op_dest_dest(0));
	dspdma_op_end(1);
	dspdma_run(256*192);
}

void loop_clear_framebuffer(uint32_t value) {
	for (int i = 0; i < 256*192; i ++) {
		GPU_RAW_FRAMEBUFFER[i] = value;
	}
}

void main() {
	GPU_MODE_SET = GPU_MODE_RAW_FRAMEBUFFER;
	setup_vsync_interrupt();
	
	while (1) {
		// wait for vsync
		vsync_interrupt_wait();
		// clear framebuffer
		dma_clear_framebuffer(0);
		//loop_clear_framebuffer(0);
		// draw yellow square
		draw_square(frame % 236);
		// present mmfb
		GPU_PRESENT_MMFB = 1;
	}
}