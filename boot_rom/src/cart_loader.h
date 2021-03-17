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

static inline void cart_loader_begin_enumerate(volatile uint32_t * completion) {
	*completion = CART_LOADER_COMPLETION_RESULT_NONE;
	CART_LOADER_PARAM0 = (uint32_t) completion;
	CART_LOADER_COMMAND = CART_LOADER_CMD_ENUMERATE_CARTS;
}

static inline bool cart_loader_poll_completion(volatile uint32_t * completion) {
	return *completion != CART_LOADER_COMPLETION_RESULT_NONE;
}

static inline bool cart_loader_completion_is_error(volatile uint32_t completion) {
	return completion != CART_LOADER_COMPLETION_RESULT_OK && completion != CART_LOADER_COMPLETION_RESULT_NONE;
}

#endif
