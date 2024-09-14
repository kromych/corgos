/* 
  objdump -i and objdump -f 
  print out all available options
*/

/*
    OUTPUT_FORMAT(elf64-aarch64)
    OUTPUT_ARCH(aarch64)
*/

ENTRY(kernel_start)
_stack_size = 0x20000;

. = 0xffff810000000000;

PHDRS
{
    hdr     PT_LOAD;
    text    PT_LOAD;
    rodata  PT_LOAD;
    data    PT_LOAD;
    bss     PT_LOAD;
    dynamic PT_DYNAMIC;
}

SECTIONS {
    _base = .;

    _text_start = .;
    .init : ALIGN(4K) {
        *(.init)
        *(.init.*)
    } :text
    .text : ALIGN(4K) {
        *(.text)
        *(.text.*)
        . = ALIGN(8);
        *(.got.plt)
        . = ALIGN(4K);
    } :text
    _text_size = . - _text_start;
    .rodata : ALIGN(4K) {
        *(.rodata)
        *(.rodata.*)
        . = ALIGN(8);
    } :rodata
    .rela.dyn : {
        _rela_start = .;
        *(.rela)
        *(.rela.text)
        *(.rela.got)
        *(.rela.plt)
        *(.rela.bss)
        *(.rela.ifunc)
        *(.rela.text.*)
        *(.rela.data)
        *(.rela.data.*)
        *(.rela.rodata)
        *(.rela.rodata*)
        *(.rela.dyn)
        _rela_end = .;
        . = ALIGN(4K);
    } :rodata
    _rodata_end = .;
    _data_start = .;
    .data : ALIGN(4K) {
        *(.data)
        *(.data.*)
        . = ALIGN(8);
        _got_start = .;
        *(.got)
        _got_end = .;
        . = ALIGN(4K);
        _file_end = .;
    } :data
    .dynamic : ALIGN(4K){
        _dynamic_start = .;
        *(.dynamic)
        _dynamic_end = .;
    } :data :dynamic
    _data_size = . - _data_start;

    .stack (NOLOAD) : ALIGN(4K) {
        PROVIDE(_stack_top = .);
        . += _stack_size - 8;
        QUAD(0x544f424b43415453);
        PROVIDE(_stack_bot = .);
    } : bss
    ASSERT(ALIGN(4K) == ., "Stack size is not aligned!")

    .bss (NOLOAD) : ALIGN(4K) {
        _bss_start = .;
        *(.bss)
        *(.bss.*)
        *(.dynbss)
        *(COMMON)
        _bss_end = .;
    } : bss
    _end = .;
    _image_size = _end - _base;

    . = ALIGN(4K);

    _payload_start = .;

    /DISCARD/ : {
        *(.discard)
        *(.discard.*)
        *(.interp .dynamic)
        *(.dynsym .dynstr .hash .gnu.hash)
        *(.eh_frame)
        *(.gnu.version*)
        *(.note*)
        *(.comment*)
    }

    .empty (NOLOAD) : {
        *(.plt) *(.plt.*) *(.iplt) *(.igot)
        *(.data.rel.ro)
    }
    ASSERT(SIZEOF(.empty) == 0, "Unexpected sections detected!")

    .got.plt (NOLOAD) : {
        *(.got.plt)
    }
    ASSERT(SIZEOF(.got.plt) == 0 || SIZEOF(.got.plt) == 0x18, "Unexpected GOT PLT detected!")
}
