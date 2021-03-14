#ifndef RVFM_MTIMER_H
#define RVFM_MTIMER_H

#include "common.h"
#include "interrupt.h"

C_START

#define MTIMER_MTIME *((volatile uint32_t *)0xF0060000)
#define MTIMER_MTIME_H *((volatile uint32_t *)0xF0060004)
#define MTIMER_MTIME_ATOMIC_BUFF *((volatile uint32_t *)0xF0060008)
#define MTIMER_MTIME_H_ATOMIC_BUFF *((volatile uint32_t *)0xF006000C)
#define MTIMER_MTIME_ATOMIC_READ_TRIGGER *((volatile uint32_t *)0xF0060010)
#define MTIMER_MTIME_ATOMIC_WRITE_TRIGGER *((volatile uint32_t *)0xF0060014)
#define MTIMER_MTIME_ATOMIC_SWAP_TRIGGER *((volatile uint32_t *)0xF0060018)

#define MTIMER_MTIMECMP *((volatile uint32_t *)0xF0060020)
#define MTIMER_MTIMECMP_H *((volatile uint32_t *)0xF0060024)
#define MTIMER_MTIMECMP_ATOMIC_BUFF *((volatile uint32_t *)0xF0060028)
#define MTIMER_MTIMECMP_H_ATOMIC_BUFF *((volatile uint32_t *)0xF006002C)
#define MTIMER_MTIMECMP_ATOMIC_READ_TRIGGER *((volatile uint32_t *)0xF0060030)
#define MTIMER_MTIMECMP_ATOMIC_WRITE_TRIGGER *((volatile uint32_t *)0xF0060034)
#define MTIMER_MTIMECMP_ATOMIC_SWAP_TRIGGER *((volatile uint32_t *)0xF0060038)

#define MTIMER_DUAL_ATOMIC_WRITE_TRIGGER *((volatile uint32_t *)0xF0060040)
#define MTIMER_DUAL_ATOMIC_SWAP_TRIGGER *((volatile uint32_t *)0xF0060044)

static inline void mtimer_schedule(uint32_t time_ms) {
	MTIMER_MTIME_H_ATOMIC_BUFF = 0;
	MTIMER_MTIME_ATOMIC_BUFF = 0;
	MTIMER_MTIMECMP_ATOMIC_BUFF = time_ms;
	MTIMER_MTIMECMP_H_ATOMIC_BUFF = 0;
	MTIMER_DUAL_ATOMIC_WRITE_TRIGGER = 1;
}

static inline void mtimer_schedule64(uint64_t time_ms) {
	MTIMER_MTIME_H_ATOMIC_BUFF = 0;
	MTIMER_MTIME_ATOMIC_BUFF = 0;
	MTIMER_MTIMECMP_ATOMIC_BUFF = (uint32_t) time_ms;
	MTIMER_MTIMECMP_H_ATOMIC_BUFF = (uint32_t) (time_ms >> 32);
	MTIMER_DUAL_ATOMIC_WRITE_TRIGGER = 1;
}

static inline void mtimer_enable_interrupt() {
	__asm__ volatile(
		"li t0, 0x80\n"
		"csrrs zero, mie, t0" ::: "t0"
	);
}

static inline void mtimer_disable_interrupt() {
	__asm__ volatile(
		"li t0, 0x80\n"
		"csrrc zero, mie, t0" ::: "t0"
	);
}

static inline bool mtimer_interrupt_pending() {
	return (get_mip() & MIP_MTIP) != 0;
}

static inline void mtimer_interrupt_ack() {
	clear_mip_bits(MIP_MTIP);
}

C_END

#endif
