#ifndef RVFM_BLIT_H
#define RVFM_BLIT_H

#include <common.h>
#include <dspdma.h>

typedef struct {
	volatile uint32_t * buffer;
	uint32_t width;
	uint32_t height;
} blit_buff_t;

inline static void blit_sprite(blit_buff_t * sprite_buffer, blit_buff_t * dest_buffer, int x, int y) {
	int32_t sprite_width = sprite_buffer->width;
	int32_t sprite_height = sprite_buffer->height;
	int32_t dest_width = dest_buffer->width;
	int32_t dest_height = dest_buffer->height;
	if (x <= -sprite_width || x >= dest_width|| y <= -sprite_height || y >= dest_height) {
		return;
	}
	int32_t src_start_row = 0;
	int32_t src_start_col = 0;
	int32_t width = sprite_width;
	int32_t height = sprite_height;
	if (x < 0) {
		src_start_col = - x;
		width += x;
		x = 0;
	} else if (x > (dest_width - sprite_width)) {
		width = (dest_width - x);
	}
	if (y < 0) {
		src_start_row = - y;
		height += y;
		y = 0;
	} else if (y > (dest_height - sprite_height)) {
		height = (dest_height - y);
	}
	dspdma_source_mem32_blit2d(0, (void *) & sprite_buffer->buffer[src_start_col + src_start_row * sprite_width], 4, width, sprite_width, 0xFFFFFFFF);
	dspdma_dest_mem32_blit2d(0, (void *) & dest_buffer->buffer[x + y * dest_width], 4, width, dest_width, 0xFFFFFFFF);
	dspdma_op_copy(0, dspdma_op_source_source(0), dspdma_op_dest_dest(0));
	dspdma_op_end(1);
	dspdma_run(width * height);
}

inline static void blit_sprite_cutout(blit_buff_t * sprite_buffer, blit_buff_t * dest_buffer, int x, int y) {
	int32_t sprite_width = sprite_buffer->width;
	int32_t sprite_height = sprite_buffer->height;
	int32_t dest_width = dest_buffer->width;
	int32_t dest_height = dest_buffer->height;
	if (x <= -sprite_width || x >= dest_width|| y <= -sprite_height || y >= dest_height) {
		return;
	}
	int32_t src_start_row = 0;
	int32_t src_start_col = 0;
	int32_t width = sprite_width;
	int32_t height = sprite_height;
	if (x < 0) {
		src_start_col = - x;
		width += x;
		x = 0;
	} else if (x > (dest_width - sprite_width)) {
		width = (dest_width - x);
	}
	if (y < 0) {
		src_start_row = - y;
		height += y;
		y = 0;
	} else if (y > (dest_height - sprite_height)) {
		height = (dest_height - y);
	}
	dspdma_source_mem32_blit2d(0, (void *) & sprite_buffer->buffer[src_start_col + src_start_row * sprite_width], 4, width, sprite_width, 0xFFFFFFFF);
	dspdma_dest_mem32_blit2d(0, (void *) & dest_buffer->buffer[x + y * dest_width], 4, width, dest_width, 0xFFFFFFFF);
	dspdma_op_and(0, dspdma_op_source_source(0), dspdma_op_source_const(0xFF000000), dspdma_op_dest_ibuff(0));
	dspdma_op_conditional_copy(1, dspdma_op_source_source(0), dspdma_op_source_ibuff(0), dspdma_op_dest_dest(0));
	dspdma_op_end(2);
	dspdma_run(width * height);
}

inline static void blit_sprite_alpha_blend(blit_buff_t * sprite_buffer, blit_buff_t * dest_buffer, int x, int y) {
	int32_t sprite_width = sprite_buffer->width;
	int32_t sprite_height = sprite_buffer->height;
	int32_t dest_width = dest_buffer->width;
	int32_t dest_height = dest_buffer->height;
	if (x <= -sprite_width || x >= dest_width|| y <= -sprite_height || y >= dest_height) {
		return;
	}
	int32_t src_start_row = 0;
	int32_t src_start_col = 0;
	int32_t width = sprite_width;
	int32_t height = sprite_height;
	if (x < 0) {
		src_start_col = - x;
		width += x;
		x = 0;
	} else if (x > (dest_width - sprite_width)) {
		width = (dest_width - x);
	}
	if (y < 0) {
		src_start_row = - y;
		height += y;
		y = 0;
	} else if (y > (dest_height - sprite_height)) {
		height = (dest_height - y);
	}
	dspdma_source_mem32_blit2d(0, (void *) & sprite_buffer->buffer[src_start_col + src_start_row * sprite_width], 4, width, sprite_width, 0xFFFFFFFF);
	dspdma_dest_mem32_blit2d(0, (void *) & dest_buffer->buffer[x + y * dest_width], 4, width, dest_width, 0xFFFFFFFF);
	dspdma_op_and(0, dspdma_op_source_source(0), dspdma_op_source_const(0xFF000000), dspdma_op_dest_ibuff(0));
	dspdma_op_conditional_copy(1, dspdma_op_source_source(0), dspdma_op_source_ibuff(0), dspdma_op_dest_dest(0));
	dspdma_op_end(2);
	dspdma_run(width * height);
}

#endif
