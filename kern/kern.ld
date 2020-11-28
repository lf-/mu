/* much of this is cribbed from https://github.com/mit-pdos/xv6-riscv/blob/riscv/kernel/kernel.ld */

OUTPUT_ARCH("riscv64")
ENTRY(_entry)

SECTIONS {
    /* VIRT_DRAM */
    . = 0x80000000;

    .text : {
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
        *(.srodata .srodata.*) /* apparently these are the same as rodata for rv64 ? */
        . = ALIGN(16);
        *(.rodata .rodata.*)
    }

    .data : {
        . = ALIGN(16);
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

    PROVIDE(end = .);
}