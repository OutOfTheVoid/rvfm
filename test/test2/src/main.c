#include <stdint.h>

#define DEBUG_IO_MSG_ADDRESS *((volatile uint32_t *)0xF0000000)
#define DEBUG_IO_MSG_LENGTH *((volatile uint32_t *)0xF0000004)
#define DEBUG_IO_WRITE *((volatile uint32_t *)0xF0000008)

#define GPU_RAW_FRAMEBUFFER ((volatile uint32_t *) 0x2000000)

#define GPU_MODE_SET *((volatile uint32_t *) 0xF0010000)

#define GPU_MODE_RAW_FRAMEBUFFER 1

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
		"csrrs zero, mie, t0" ::: "t0"
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

int interrupt_count = 0;

void __attribute__((interrupt("machine"))) interrupt_handler () {
	clear_pending_interrupts();
	interrupt_count ++;
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
	set_interrupt_handler(interrupt_handler);
	clear_pending_interrupts();
	enable_interrupts();
	enable_external_interrupts();
}

void vsync_interrupt_wait() {
	clear_pending_interrupts();
	wfi();
}

void main() {
	setup_vsync_interrupt();
	GPU_MODE_SET = GPU_MODE_RAW_FRAMEBUFFER;
	
	while (1) {
		// wait for vsync
		vsync_interrupt_wait();
		
		// clear framebuffer
		for (int i = 0; i < 256*192; i ++) {
			GPU_RAW_FRAMEBUFFER[i] = 0x00000000;
		}
		
		// draw yellow square
		draw_square(interrupt_count % 236);
	}
}