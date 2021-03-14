#ifndef RVFM_CORE2_H
#define RVFM_CORE2_H

#include "common.h"

C_START

#ifndef RVFM_CUSTOM_CORE2_START
extern uint32_t _core2_start;
#define RVFM_CORE2_START_PC ((uint32_t) & _core2_start)
#endif

#define CORE2_CONTROLLER_START_ADDRESS *((volatile uint32_t *) 0xF0040000)
#define CORE2_CONTROLLER_RUN *((volatile uint32_t *) 0xF0040004)
#define CORE2_CONTROLLER_STATUS *((volatile uint32_t *) 0xF0040008)

#ifndef RVFM_CUSTOM_CORE2_START
void start_core2() {
	CORE2_CONTROLLER_START_ADDRESS = RVFM_CORE2_START_PC;
	CORE2_CONTROLLER_RUN = 1;
}
#else
void start_core2(uint32_t start_address) {
	CORE2_CONTROLLER_START_ADDRESS = start_address;
	CORE2_CONTROLLER_RUN = 1;
}
#endif

static inline bool core2_started() {
	return CORE2_CONTROLLER_STATUS != 0;
}

C_END

#endif
