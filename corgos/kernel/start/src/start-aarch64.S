	.arch armv8.1-a
	.file	"start-aarch64.S"
    .extern start, _stack_bot, _image_size, relocate
	.weak _DYNAMIC
	.hidden _DYNAMIC

	.section ".bss.page_tables", "aw", @nobits
    .global _page_tables_start
    .global _page_tables_end

    .balign 0x4000
_page_tables_start:
    .space 0x800000
_page_tables_end:
