#ifndef RVFM_CART_LOADER_H
#define RVFM_CART_LOADER_H

#include <common.h>

#define CART_LOADER_COMMAND *((volatile uint32_t *) 0xF0080000)
#define CART_LOADER_PARAM0 *((volatile uint32_t *) 0xF0080004)
#define CART_LOADER_PARAM1 *((volatile uint32_t *) 0xF0080008)
#define CART_LOADER_PARAM2 *((volatile uint32_t *) 0xF008000C)
#define CART_LOADER_PARAM3 *((volatile uint32_t *) 0xF0080010)
#define CART_LOADER_PARAM4 *((volatile uint32_t *) 0xF0080014)
#define CART_LOADER_PARAM5 *((volatile uint32_t *) 0xF0080018)
#define CART_LOADER_CART_COUNT *((volatile uint32_t *) 0xF008001C)

#define CART_LOADER_CMD_ENUMERATE_CARTS 0
#define CART_LOADER_CMD_READ_CART_METADATA 1
#define CART_LOADER_CMD_LOAD_CART 2
#define CART_LOADER_CMD_SETUP_DATA_ACCESS_FS 3
#define CART_LOADER_CMD_SETUP_DATA_ACCESS_BIN 4
#define CART_LOADER_CMD_CLOSE_DATA_ACCESS 5
#define CART_LOADER_CMD_READ_DATA 6
#define CART_LOADER_CMD_WRITE_DATA 7
#define CART_LOADER_CMD_GET_DATA_EXTENTS 8

#define CART_LOADER_COMPLETION_RESULT_NONE 0
#define CART_LOADER_COMPLETION_RESULT_OK 1
#define CART_LOADER_COMPLETION_RESULT_ERROR_READING_DIR 2
#define CART_LOADER_COMPLETION_RESULT_CART_INDEX_OUT_OF_BOUNDS 3
#define CART_LOADER_COMPLETION_RESULT_FAILED_READING_BINARY 4
#define CART_LOADER_COMPLETION_RESULT_DATA_SLOT_INDEX_OUT_OF_BOUNDS 5
#define CART_LOADER_COMPLETION_RESULT_NO_CART_LOADED 6
#define CART_LOADER_COMPLETION_RESULT_FAILED_OPENING_FILE 7
#define CART_LOADER_COMPLETION_RESULT_BAD_OPERATION_FOR_DATA_FORMAT 8
#define CART_LOADER_COMPLETION_RESULT_FILENAME_READ_ERROR 9
#define CART_LOADER_COMPLETION_RESULT_DATA_SLOT_NOT_OPEN 10
#define CART_LOADER_COMPLETION_RESULT_FAILED_READING_FILE 11

#define CART_LOADER_SETUP_DATA_ACCESS_FS_FLAG_WRITE 1

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

static inline void cart_loader_setup_data_slot_fs(uint32_t slot_index, const char * filename, bool write, volatile uint32_t * completion) {
	*completion = CART_LOADER_COMPLETION_RESULT_NONE;
	CART_LOADER_PARAM0 = slot_index;
	CART_LOADER_PARAM1 = (uint32_t) filename;
	CART_LOADER_PARAM2 = (uint32_t) completion;
	CART_LOADER_PARAM3 = write ? CART_LOADER_SETUP_DATA_ACCESS_FS_FLAG_WRITE : 0;
	CART_LOADER_COMMAND = CART_LOADER_CMD_SETUP_DATA_ACCESS_FS;
}

static inline void cart_loader_get_data_extents(uint32_t slot_index, volatile uint32_t * extents, volatile uint32_t * completion) {
	*completion = CART_LOADER_COMPLETION_RESULT_NONE;
	CART_LOADER_PARAM0 = slot_index;
	CART_LOADER_PARAM1 = (uint32_t) extents;
	CART_LOADER_PARAM2 = (uint32_t) completion;
	CART_LOADER_COMMAND = CART_LOADER_CMD_GET_DATA_EXTENTS;
}

static inline void cart_loader_read_data(uint32_t slot_index, uint32_t offset, uint32_t length, void * buffer, volatile uint32_t * read_size, volatile uint32_t * completion) {
	*completion = CART_LOADER_COMPLETION_RESULT_NONE;
	CART_LOADER_PARAM0 = slot_index;
	CART_LOADER_PARAM1 = offset;
	CART_LOADER_PARAM2 = length;
	CART_LOADER_PARAM3 = (uint32_t) buffer;
	CART_LOADER_PARAM4 = (uint32_t) read_size;
	CART_LOADER_PARAM5 = (uint32_t) completion;
	CART_LOADER_COMMAND = CART_LOADER_CMD_READ_DATA;
}

#endif
