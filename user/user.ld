OUTPUT_ARCH("riscv64");
ENTRY(_start);

SECTIONS {
    /* guard page */
    . = 0x100000;

    .text ALIGN(0x1000) : {
        . = .;
        *(.text .text.*)
    }

    .rodata ALIGN(0x1000) : {
        *(.srodata .srodata.*)
        *(.rodata .rodata.*)
    }

    .data ALIGN(0x1000) : {
        *(.sdata .sdata.*)
        *(.data .data.*)
    }

    .bss ALIGN(0x1000) : {
        *(.sbss .sbss.*)
        *(.bss .bss.*)
    }

    . = ALIGN(0x1000);
}