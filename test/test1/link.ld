OUTPUT_ARCH("riscv")
ENTRY(_start)

MEMORY
{
	RAM (rwx) : ORIGIN = 0x00001000, LENGTH = 0x0FFFF000
}

SECTIONS
{
	.text :
	{
		_text_start = ABSOLUTE(.);
		KEEP(*(.init_text*))
		KEEP(*(.text*))
        KEEP(*(.rodata*))
		_text_end = ABSOLUTE(.);
	} > RAM
	
	.data ALIGN(4) :
	{
		_data_start = ABSOLUTE(.);
		KEEP(*(.data*))
		_data_end = ABSOLUTE(.);
	} > RAM
	
	.bss ALIGN(4) :
	{
		_bss_start = ABSOLUTE(.);
		KEEP(*(.bss*))
		_bss_end = ABSOLUTE(.);
	} > RAM

	. = ORIGIN(RAM) + LENGTH(RAM) - 4;
	_stack_top = ABSOLUTE(.);
}