#ifndef RVFM_GPU_MMFB_H
#define RVFM_GPU_MMFB_H

#include <common.h>
#include <dspdma.h>

#include <gpu/gpu.h>

inline static void gpu_mmfb_clear(volatile uint32_t * fb_ptr, uint32_t color) {
	dspdma_dest_mem32(0, (void *) fb_ptr, 4, DSPDMA_LOOP_INDEX_NEVER);
	dspdma_op_copy(0, dspdma_op_source_const(color), dspdma_op_dest_dest(0));
	dspdma_op_end(1);
	dspdma_run(GPU_OUTPUT_RESOLUTION_W * GPU_OUTPUT_RESOLUTION_H);
}

inline static void gpu_mmfb_set_ptr(volatile uint32_t * mmfb_ptr) {
	GPU_RAW_FRAMEBUFFER_PTR = (uint32_t) mmfb_ptr;
}

inline static void gpu_mmfb_present() {
	GPU_PRESENT_MMFB = 1;
}

#endif
