#ifndef CART_LOADER_H
#define CART_LOADER_H

#include "common.h"

#define CART_LOADER_COMMAND *((volatile uint32_t *) 0xF0080000)
#define CART_LOADER_PARAM0 *((volatile uint32_t *) 0xF0080004)
#define CART_LOADER_PARAM1 *((volatile uint32_t *) 0xF0080008)
#define CART_LOADER_PARAM2 *((volatile uint32_t *) 0xF008000C)
#define CART_LOADER_PARAM3 *((volatile uint32_t *) 0xF0080010)
#define CART_LOADER_CART_COUNT *((volatile uint32_t *) 0xF0080014)

#define CART_LOADER_CMD_ENUMERATE_CARTS 0
#define CART_LOADER_CMD_READ_CART_METADATA 1
#define CART_LOADER_CMD_LOAD_CART 2

#define CART_LOADER_COMPLETION_RESULT_NONE 0
#define CART_LOADER_COMPLETION_RESULT_OK 1

#define PACKED __attribute__((packed))

typedef struct {
	uint32_t revision;
	uint32_t minor;
	uint32_t major;
} PACKED semver_t;

typedef struct {
	char name[256];
	char dev[256];
	char dev_url[256];
	char source_url[256];
	uint32_t icon_bitmap[64*64];
	semver_t version;
} PACKED cart_metadata_t;

static inline void cart_loader_begin_enumerate(volatile uint32_t * completion) {
	*completion = CART_LOADER_COMPLETION_RESULT_NONE;
	CART_LOADER_PARAM0 = (uint32_t) completion;
	CART_LOADER_COMMAND = CART_LOADER_CMD_ENUMERATE_CARTS;
}

static inline bool cart_loader_poll_completion(volatile uint32_t * completion) {
	return *completion != CART_LOADER_COMPLETION_RESULT_NONE;
}

static inline void cart_loader_read_cart_metadata(uint32_t index, volatile cart_metadata_t * metadata, volatile uint32_t * completion) {
	*completion = CART_LOADER_COMPLETION_RESULT_NONE;
	CART_LOADER_PARAM0 = index;
	CART_LOADER_PARAM1 = (uint32_t) metadata;
	CART_LOADER_PARAM2 = (uint32_t) completion;
	CART_LOADER_COMMAND = CART_LOADER_CMD_READ_CART_METADATA;
}

static inline void cart_loader_load_cart(uint32_t index, volatile uint32_t * error_completion) {
	*error_completion = CART_LOADER_COMPLETION_RESULT_NONE;
	CART_LOADER_PARAM0 = index;
	CART_LOADER_PARAM1 = (uint32_t) error_completion;
	CART_LOADER_COMMAND = CART_LOADER_CMD_LOAD_CART;
}

static inline bool cart_loader_completion_is_error(volatile uint32_t completion) {
	return completion != CART_LOADER_COMPLETION_RESULT_OK && completion != CART_LOADER_COMPLETION_RESULT_NONE;
}

#endif
