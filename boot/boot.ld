ENTRY(uefi_entry)

SECTIONS {
    . = 0x001000000;

    .start          : { *(.start) }
    .text           : { *(.text .text.*) }
    .rodata         : { *(.rodata .rodata.*) }
    .data           : { *(.data .data.*) }
    .bss            : { *(.bss .bss.*) }
    .sdata          : { *(.sdata .sdata.*) }
    .dynamic        : { *(.dynamic .dynamic.*) }
    .dynsym         : { *(.dynsym .dynsym.*) }
    .rel            : { *(.rel .rel.*) }
    .rela           : { *(.rela .rela.*) }
    .reloc          : { *(.reloc .reloc.*) }
    .eh_frame_hdr   : { *(.eh_frame_hdr .eh_frame_hdr.*) }
    .eh_frame       : { *(.eh_frame .eh_frame.*) }
}
