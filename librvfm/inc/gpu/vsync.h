#ifndef RVFM_GPU_VSYNC_H
#define RVFM_GPU_VSYNC_H

#include <gpu/gpu.h>
#include <interrupt.h>

static volatile int vsync_wait;

inline static void gpu_vsync_wait_interrupt_callback() {
	if (gpu_vsync_interrupt_pending()) {
		vsync_wait = 0;
		gpu_clear_vsync_interrupt();
	}
}

inline static void gpu_vsync_wait_init() {
	gpu_clear_vsync_interrupt();
	gpu_enable_vsync_interrupt();
	vsync_wait = 0;
}

inline static void gpu_vsync_wait() {
	vsync_wait = 1;
	while (vsync_wait) {
		wfi();
	}
}

#endif
