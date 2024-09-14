	.file	"start-x86_64.S"
	.weak _DYNAMIC
	.hidden _DYNAMIC

	.section ".bss.page_tables", "aw", @nobits
    .global _page_tables_start
    .global _page_tables_end

    .balign 0x4000
_page_tables_start:
    .space 0x800000
_page_tables_end:
