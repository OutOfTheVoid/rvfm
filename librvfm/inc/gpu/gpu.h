#ifndef RVFM_GPU_H
#define RVFM_GPU_H

#include <common.h>

#define GPU_RAW_FRAMEBUFFER ((volatile uint32_t *) 0x2000000)

#define GPU_MODE_SET *((volatile uint32_t *) 0xF0010000)
#define GPU_PRESENT_MMFB *((volatile uint32_t *) 0xF0010004)
#define GPU_VSYNC_INT_ENABLE *((volatile uint32_t *) 0xF0010008)

#define GPU_MODE_DISABLED 0
#define GPU_MODE_RAW_FRAMEBUFFER 1

#define GPU_SYNC_INTERRUPT_STATE *((volatile uint32_t *) 0xF0030000)

#define GPU_OUTPUT_RESOLUTION_H 192
#define GPU_OUTPUT_RESOLUTION_W 256

typedef enum {
	Disabled = GPU_MODE_DISABLED,
	RawFramebuffer = GPU_MODE_RAW_FRAMEBUFFER,
} GpuMode;

inline static void gpu_set_mode(GpuMode mode) {
	GPU_MODE_SET = (uint32_t) mode;
}

inline static void gpu_enable_vsync_interrupt() {
	GPU_VSYNC_INT_ENABLE = 1;
}

inline static void gpu_disable_vsync_interrupt() {
	GPU_VSYNC_INT_ENABLE = 0;
}

inline static bool gpu_vsync_interrupt_pending() {
	return GPU_SYNC_INTERRUPT_STATE != 0;
}

inline static void gpu_clear_vsync_interrupt() {
	GPU_SYNC_INTERRUPT_STATE = 0;
}

#endif
