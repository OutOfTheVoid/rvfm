#ifndef RVFM_DSPDMA_H
#define RVFM_DSPDMA_H

#include "common.h"

C_START

#define DSPDMA_TYPE *((volatile uint32_t *) 0xF0020000)
#define DSPDMA_INDEX *((volatile uint32_t *) 0xF0020004)
#define DSPDMA_PARAM0 *((volatile uint32_t *) 0xF0020008)
#define DSPDMA_PARAM1 *((volatile uint32_t *) 0xF002000C)
#define DSPDMA_PARAM2 *((volatile uint32_t *) 0xF0020010)
#define DSPDMA_PARAM3 *((volatile uint32_t *) 0xF0020014)
#define DSPDMA_PARAM4 *((volatile uint32_t *) 0xF0020018)
#define DSPDMA_PARAM5 *((volatile uint32_t *) 0xF002001C)
#define DSPDMA_COMMAND *((volatile uint32_t *) 0xF0020020)
#define DSPDMA_TRANSFER_SIZE *((volatile uint32_t *) 0xF0020024)
#define DSPDMA_ERROR *((volatile uint32_t *) 0xF0020028)

#define DSPDMA_SOURCE_TYPE_NONE 0
#define DSPDMA_SOURCE_TYPE_MEM8 1
#define DSPDMA_SOURCE_TYPE_MEM16 2
#define DSPDMA_SOURCE_TYPE_MEM32 3

#define DSPDMA_DEST_TYPE_NONE 0
#define DSPDMA_DEST_TYPE_MEM8 1
#define DSPDMA_DEST_TYPE_MEM16 2
#define DSPDMA_DEST_TYPE_MEM32 3

#define DSPDMA_OP_TYPE_END 0
#define DSPDMA_OP_TYPE_COPY 1
#define DSPDMA_OP_TYPE_ADD 2

#define DSPDMA_COMMAND_TRIGGER 0
#define DSPDMA_COMMAND_WRITE_SOURCE 1
#define DSPDMA_COMMAND_WRITE_DEST 2
#define DSPDMA_COMMAND_WRITE_PROGRAM_OP 3

#define DSPDMA_IOP_SOURCE_TYPE_SOURCE 0
#define DSPDMA_IOP_SOURCE_TYPE_IBUFFER 1
#define DSPDMA_IOP_SOURCE_TYPE_CONST 2

#define DSPDMA_IOP_DEST_TYPE_DEST 0
#define DSPDMA_IOP_DEST_TYPE_IBUFFER 1

#define DSPDMA_ERROR_NONE 0
#define DSPDMA_ERROR_INDEX_OUT_OF_RANGE 1
#define DSPDMA_ERROR_TYPE_OUT_OF_RANGE 2
#define DSPDMA_ERROR_PARAM0_OUT_OF_RANGE 3
#define DSPDMA_ERROR_PARAM1_OUT_OF_RANGE 4
#define DSPDMA_ERROR_PARAM2_OUT_OF_RANGE 5
#define DSPDMA_ERROR_SOURCE_OVERLAPS_PERIPHERAL 6
#define DSPDMA_ERROR_DEST_OVERLAPS_PERIPHERAL 7
#define DSPDMA_ERROR_TRANSFER_SIZE_TOO_LARGE 8
#define DSPDMA_ERROR_BAD_COMMAND 9
#define DSPDMA_ERROR_SOURCE_OUT_OF_RANGE 10
#define DSPDMA_ERROR_DEST_OUT_OF_RANGE 11
#define DSPDMA_ERROR_IOP_SOURCE_TYPE_OUT_OF_RANGE 12
#define DSPDMA_ERROR_IOP_DEST_TYPE_OUT_OF_RANGE 13
#define DSPDMA_ERROR_USAGE_OF_NULL_SOURCE 14
#define DSPDMA_ERROR_USAGE_OF_NULL_DEST 15
#define DSPDMA_ERROR_MEMORY_ACCESS 80

#define DSPDMA_MEM_ACCESS_ERROR_TYPE_READ 0
#define DSPDMA_MEM_ACCESS_ERROR_TYPE_WRITE 1

#define DSPDMA_LOOP_INDEX_NEVER 0xFFFFFFFF

typedef enum {
	DspDma_OpSource_Source = DSPDMA_IOP_SOURCE_TYPE_SOURCE,
	DspDma_OpSource_IBuff = DSPDMA_IOP_SOURCE_TYPE_IBUFFER,
	DspDma_OpSource_Constant = DSPDMA_IOP_SOURCE_TYPE_CONST,
} dspdma_op_src_type_t;

typedef enum {
	DspDma_OpDest_Dest = DSPDMA_IOP_DEST_TYPE_DEST,
	DspDma_OpDest_IBuff = DSPDMA_IOP_DEST_TYPE_IBUFFER,
} dspdma_op_dest_type_t;

typedef struct {
	dspdma_op_src_type_t type;
	union {
		uint32_t source;
		uint32_t ibuff;
		uint32_t constant;
	};
} dspdma_op_src_t;

typedef struct {
	dspdma_op_dest_type_t type;
	union {
		uint32_t dest;
		uint32_t ibuff;
	};
} dspdma_op_dest_t;

dspdma_op_src_t dspdma_op_source_const(uint32_t value) {
	dspdma_op_src_t val;
	val.type = DspDma_OpSource_Constant;
	val.constant = value;
	return val;
}

dspdma_op_src_t dspdma_op_source_ibuff(uint32_t ibuff_index) {
	dspdma_op_src_t val;
	val.type = DspDma_OpSource_IBuff;
	val.ibuff = ibuff_index;
	return val;
}

dspdma_op_src_t dspdma_op_source_source(uint32_t source_index) {
	dspdma_op_src_t val;
	val.type = DspDma_OpSource_Source;
	val.source = source_index;
	return val;
}

dspdma_op_dest_t dspdma_op_dest_ibuff(uint32_t ibuff_index) {
	dspdma_op_dest_t val;
	val.type = DspDma_OpDest_IBuff;
	val.ibuff = ibuff_index;
	return val;
}

dspdma_op_dest_t dspdma_op_dest_dest(uint32_t dest_index) {
	dspdma_op_dest_t val;
	val.type = DspDma_OpDest_Dest;
	val.dest = dest_index;
	return val;
}

inline static int dspdma_source_mem8(uint32_t index, void * address, uint32_t increment, uint32_t loop_index) {
	DSPDMA_TYPE = DSPDMA_SOURCE_TYPE_MEM8;
	DSPDMA_INDEX = index;
	DSPDMA_PARAM0 = (uint32_t) address;
	DSPDMA_PARAM1 = increment;
	DSPDMA_PARAM2 = loop_index;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_SOURCE;
	return DSPDMA_ERROR;
}

inline static int dspdma_source_mem16(uint32_t index, void * address, uint32_t increment, uint32_t loop_index) {
	DSPDMA_TYPE = DSPDMA_SOURCE_TYPE_MEM16;
	DSPDMA_INDEX = index;
	DSPDMA_PARAM0 = (uint32_t) address;
	DSPDMA_PARAM1 = increment;
	DSPDMA_PARAM2 = loop_index;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_SOURCE;
	return DSPDMA_ERROR;
}

inline static int dspdma_source_mem32(uint32_t index, void * address, uint32_t increment, uint32_t loop_index) {
	DSPDMA_TYPE = DSPDMA_SOURCE_TYPE_MEM32;
	DSPDMA_INDEX = index;
	DSPDMA_PARAM0 = (uint32_t) address;
	DSPDMA_PARAM1 = increment;
	DSPDMA_PARAM2 = loop_index;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_SOURCE;
	return DSPDMA_ERROR;
}

inline static int dspdma_source_none(uint32_t index) {
	DSPDMA_TYPE = DSPDMA_SOURCE_TYPE_NONE;
	DSPDMA_INDEX = index;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_SOURCE;
	return DSPDMA_ERROR;
}

inline static int dspdma_dest_mem8(uint32_t index, void * address, uint32_t increment, uint32_t loop_index) {
	DSPDMA_TYPE = DSPDMA_DEST_TYPE_MEM8;
	DSPDMA_INDEX = index;
	DSPDMA_PARAM0 = (uint32_t) address;
	DSPDMA_PARAM1 = increment;
	DSPDMA_PARAM2 = loop_index;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_DEST;
	return DSPDMA_ERROR;
}

inline static int dspdma_dest_mem16(uint32_t index, void * address, uint32_t increment, uint32_t loop_index) {
	DSPDMA_TYPE = DSPDMA_DEST_TYPE_MEM16;
	DSPDMA_INDEX = index;
	DSPDMA_PARAM0 = (uint32_t) address;
	DSPDMA_PARAM1 = increment;
	DSPDMA_PARAM2 = loop_index;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_DEST;
	return DSPDMA_ERROR;
}

inline static int dspdma_dest_mem32(uint32_t index, void * address, uint32_t increment, uint32_t loop_index) {
	DSPDMA_TYPE = DSPDMA_DEST_TYPE_MEM32;
	DSPDMA_INDEX = index;
	DSPDMA_PARAM0 = (uint32_t) address;
	DSPDMA_PARAM1 = increment;
	DSPDMA_PARAM2 = loop_index;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_DEST;
	return DSPDMA_ERROR;
}

inline static int dspdma_dest_none(uint32_t index) {
	DSPDMA_TYPE = DSPDMA_DEST_TYPE_NONE;
	DSPDMA_INDEX = index;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_DEST;
	return DSPDMA_ERROR;
}

static inline int dspdma_op_copy (uint32_t op_index, dspdma_op_src_t source, dspdma_op_dest_t dest) {
	DSPDMA_TYPE = DSPDMA_OP_TYPE_COPY;
	DSPDMA_INDEX = op_index;
	DSPDMA_PARAM0 = source.type;
	switch (source.type) {
		case DspDma_OpSource_Constant:
			DSPDMA_PARAM1 = source.constant;
			break;
		case DspDma_OpSource_IBuff:
			DSPDMA_PARAM1 = source.ibuff;
			break;
		case DspDma_OpSource_Source:
			DSPDMA_PARAM1 = source.source;
			break;
		default:
			return -1;
	}
	DSPDMA_PARAM2 = dest.type;
	switch (dest.type) {
		case DspDma_OpDest_IBuff:
			DSPDMA_PARAM3 = dest.ibuff;
			break;
		case DspDma_OpDest_Dest:
			DSPDMA_PARAM3 = dest.dest;
			break;
		default:
			return -1;
	}
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_PROGRAM_OP;
	return DSPDMA_ERROR;
}

static inline int dspdma_op_add (uint32_t op_index, dspdma_op_src_t source_a, dspdma_op_src_t source_b, dspdma_op_dest_t dest) {
	DSPDMA_TYPE = DSPDMA_OP_TYPE_ADD;
	DSPDMA_INDEX = op_index;
	DSPDMA_PARAM0 = source_a.type;
	switch (source_a.type) {
		case DspDma_OpSource_Constant:
			DSPDMA_PARAM1 = source_a.constant;
			break;
		case DspDma_OpSource_IBuff:
			DSPDMA_PARAM1 = source_a.ibuff;
			break;
		case DspDma_OpSource_Source:
			DSPDMA_PARAM1 = source_a.source;
			break;
		default:
			return -1;
	}
	DSPDMA_PARAM2 = source_b.type;
	switch (source_b.type) {
		case DspDma_OpSource_Constant:
			DSPDMA_PARAM3 = source_b.constant;
			break;
		case DspDma_OpSource_IBuff:
			DSPDMA_PARAM3 = source_b.ibuff;
			break;
		case DspDma_OpSource_Source:
			DSPDMA_PARAM3 = source_b.source;
			break;
		default:
			return -1;
	}
	DSPDMA_PARAM4 = dest.type;
	switch (dest.type) {
		case DspDma_OpDest_IBuff:
			DSPDMA_PARAM5 = dest.ibuff;
			break;
		case DspDma_OpDest_Dest:
			DSPDMA_PARAM5 = dest.dest;
			break;
		default:
			return -1;
	}
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_PROGRAM_OP;
	return DSPDMA_ERROR;
}

inline static int dspdma_op_end(uint32_t op_index) {
	DSPDMA_TYPE = DSPDMA_OP_TYPE_END;
	DSPDMA_INDEX = op_index;
	DSPDMA_COMMAND = DSPDMA_COMMAND_WRITE_PROGRAM_OP;
}

inline static int dspdma_run(uint32_t transfer_size) {
	DSPDMA_TRANSFER_SIZE = transfer_size;
	DSPDMA_COMMAND = DSPDMA_COMMAND_TRIGGER;
	return DSPDMA_ERROR;
}

C_END

#endif
