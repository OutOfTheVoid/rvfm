#include <stdint.h>

#define DEBUG_IO_MSG_ADDRESS *((volatile uint32_t *)0xF0000000)
#define DEBUG_IO_MSG_LENGTH *((volatile uint32_t *)0xF0000004)
#define DEBUG_IO_WRITE *((volatile uint32_t *)0xF0000008)

#define GPU_RAW_FRAMEBUFFER ((volatile uint32_t *) 0x2000000)

#define GPU_MODE_SET *((volatile uint32_t *) 0xF0010000)
#define GPU_PRESENT_MMFB *((volatile uint32_t *) 0xF0010004)
#define GPU_VSYNC_INT_ENABLE *((volatile uint32_t *) 0xF0010008)

#define GPU_MODE_RAW_FRAMEBUFFER 1

#define GPU_SYNC_INTERRUPT_STATE *((volatile uint32_t *) 0xF0030000)

#include "dspdma.h"

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

void enable_interrupts() {
	__asm__ volatile("csrrsi zero, mstatus, 0x08");
}

void enable_external_interrupts() {
	__asm__ volatile(
		"li t0, 0x800\n"
		"csrrs zero, mie, t0" ::: "x5"
	);
}

void disable_interrupts() {
	__asm__ volatile("csrrsi zero, mstatus, 0x08");
}

void set_interrupt_handler(void (* __attribute__((interrupt("machine"))) handler)()) {
	__asm__ volatile("csrw 0x305, %0" :: "r"(handler));
}

void set_mstatus(void (* __attribute__((interrupt("machine"))) handler)()) {
	__asm__ volatile("csrw 0x300, %0" :: "r"(handler));
}

void clear_pending_interrupts() {
	__asm__ volatile("csrw 0x344, 0");
}

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

void dspdma_set_dest32(void * dest, int index) {
	DSPDMA_TYPE = DSPDMA_DEST_TYPE_MEM32;
	DSPDMA_INDEX = index;
	DSPDMA_PARAM0 = (uint32_t) dest;
	DSPDMA_PARAM1 = 4;
	DSPDMA_PARAM2 = 0xFFFFFFFF;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_DEST;
}

void dspdma_op_copy_const32(uint32_t op_index, uint32_t constant, uint32_t dest) {
	DSPDMA_TYPE = DSPDMA_OP_TYPE_COPY;
	DSPDMA_INDEX = op_index;
	DSPDMA_PARAM0 = DSPDMA_IOP_SOURCE_TYPE_CONST;
	DSPDMA_PARAM1 = constant;
	DSPDMA_PARAM2 = DSPDMA_IOP_DEST_TYPE_DEST;
	DSPDMA_PARAM3 = dest;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_PROGRAM_OP;
}

void dspdma_op_end(uint32_t op_index) {
	DSPDMA_TYPE = DSPDMA_OP_TYPE_END;
	DSPDMA_INDEX = op_index;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_PROGRAM_OP;
}

void dspdma_trigger() {
	DSPDMA_COMMAND = DSPDMA_COMMAND_TRIGGER;
}

void dma_clear_framebuffer(uint32_t value) {
	DSPDMA_TRANSFER_SIZE = 256*192;
	dspdma_set_dest32((void *) GPU_RAW_FRAMEBUFFER, 0);
	dspdma_op_copy_const32(0, value, 0);
	dspdma_op_end(1);
	dspdma_trigger();
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