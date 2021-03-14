#include <stdint.h>
#include <interrupt.h>
#include <debug_print.h>
#include <core2.h>
#include <sound.h>
#include <interrupt.h>

#include "note_play.h"

void main() {
	start_core2();
	wfi();
}
