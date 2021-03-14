#ifndef RVFM_INTERRUPT_H
#define RVFM_INTERRUPT_H

#include "common.h"

C_START

#define ATTR_INTERRUPT __attribute__((interrupt("machine")))

static inline void wfi() {
	__asm__ volatile("wfi");
}

static inline bool get_interrupt_state() {
	int32_t mstatus;
	__asm__ volatile("csrr mstatus, %0" : "=r"(mstatus) :: "memory");
	return (mstatus & 0x08) != 0;
}

static inline void enable_interrupts() {
	__asm__ volatile("csrrsi zero, mstatus, 0x08");
}

static inline void enable_external_interrupts() {
	__asm__ volatile(
		"li t0, 0x800\n"
		"csrrs zero, mie, t0" ::: "t0"
	);
}

static inline void enable_timer_interrupt() {
	__asm__ volatile(
		"li t0, 0x80\n"
		"csrrs zero, mie, t0" ::: "t0"
	);
}

static inline void disable_interrupts() {
	__asm__ volatile("csrrci zero, mstatus, 0x08");
}

static inline void set_interrupt_handler(void (* __attribute__((interrupt("machine"))) handler)()) {
	__asm__ volatile("csrw 0x305, %0" :: "r"(handler));
}

static inline void clear_pending_interrupts() {
	__asm__ volatile("csrw 0x344, 0");
}

static inline bool block_interrupts() {
	bool old_int_state = get_interrupt_state();
	disable_interrupts();
}

static inline void unblock_interrupts(bool old_state) {
	if (old_state) {
		enable_interrupts();
	}
}

static inline uint32_t get_mip() {
	uint32_t val = 0;
	__asm__ __volatile__("csrr %0, mip" : "=r"(val) :);
	return val;
}

static inline void clear_mip_bits(uint32_t bits) {
	__asm__ __volatile__("csrrc zero, mip, %0" :: "r"(bits) :);
}

#define MIP_MSIP (1 << 3)
#define MIP_MTIP (1 << 7)
#define MIP_MEIP (1 << 11)

C_END

#endif
