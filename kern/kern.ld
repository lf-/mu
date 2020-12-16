/* much of this is cribbed from https://github.com/mit-pdos/xv6-riscv/blob/riscv/kernel/kernel.ld */

OUTPUT_ARCH("riscv64")
ENTRY(_entry)

SECTIONS {
    /* VIRT_DRAM */
    . = 0x80000000;

    .text : {
        PROVIDE(stext = .);
        /* 
         * force the entry point to the start. I have no god damn idea how this
         * linker works or why it works for xv6...
         */
        *(.text.first)
        *(.text .text.*)
        . = ALIGN(0x1000);
        _trampoline = .;
        *(trampolines)
        /* get off the trampoline page */
        . = ALIGN(0x1000);
        ASSERT(. - _trampoline == 0x1000, "trampoline must only be one page");
        /* TODO: figure out what this does */
        /* it's probably creating a symbol with the value of . ? */
        PROVIDE(etext = .);
    }

    .rodata : {
        . = ALIGN(16);
        PROVIDE(srodata = .);
        *(.srodata .srodata.*) /* apparently these are the same as rodata for rv64 ? */
        . = ALIGN(16);
        *(.rodata .rodata.*)
        PROVIDE(erodata = .);
    }

    .data : {
        . = ALIGN(4096);
        PROVIDE(srwdata = .);
        *(.sdata .sdata.*)
        . = ALIGN(16);
        *(.data .data.*)
    }

    .bss : {
        . = ALIGN(16);
        *(.sbss .sbss.*)
        . = ALIGN(16);
        *(.bss .bss.*)
    }
    /*
    be extra super careful to not cause bugs when we make the free list initially
    */
    . = ALIGN(4096);

    PROVIDE(end = .);
}