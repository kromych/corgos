ENTRY(kernel_start)
SECTIONS
{
    . = 0xffff810000000000;
    .text ALIGN(4K) :
    {
        *(.text.*)
    } = 0xcccc
    .rodata ALIGN(4K) :
    {
        *(.rodata.*)
        KEEP(*(.note.gnu.build-id))
    } = 0xdddd
    .data ALIGN(4K) :
    {
        *(.data.*)
    } = 0xeeee
    . = ALIGN(8);
    .init_array :
    {
        __init_array_start = .;
        KEEP(*(SORT(.init_array.*)))
        KEEP(*(.init_array))
        __init_array_end = .;
    }
    .bss ALIGN(4K) :
    {
        *(COMMON)
        *(.bss)
    } = 0xffff
    /DISCARD/ :
    {
        *(.interp)
    }
}
