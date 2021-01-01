OUTPUT_ARCH("riscv64")
ENTRY(_entry)

SECTIONS {
    /*
    This one will be loaded with an actual ELF loader, if a not-that-smart one.

    Thus we don't have to do as much funny stuff with the entry point as shoo.

    Start with a pretty chonky guard page.
    */
    . = 0xffffffc000000000 + 0x100000;

    .text : {
        *(.text .text.*)
        . = ALIGN(0x1000);
        _trampoline = .;
        *(trampolines)
        /* get off the trampoline page */
        . = ALIGN(0x1000);
        ASSERT(. - _trampoline == 0x1000, "trampoline must only be one page");
    }

    .rodata ALIGN(0x1000) : {
        *(.srodata .srodata.*) /* apparently these are the same as rodata for rv64 ? */
        . = ALIGN(16);
        *(.rodata .rodata.*)
    }

    .data ALIGN(0x1000) : {
        . = ALIGN(0x1000);
        *(.sdata .sdata.*)
        . = ALIGN(16);
        *(.data .data.*)
    }

    .bss ALIGN(0x1000) : {
        . = ALIGN(16);
        *(.sbss .sbss.*)
        . = ALIGN(16);
        *(.bss .bss.*)
    }

    . = ALIGN(4096);
}