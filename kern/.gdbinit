set confirm off
set architecture riscv:rv64
target extended-remote 127.0.0.1:1234
alias connect = target extended-remote :1234
symbol-file target/riscv64imac-unknown-none-elf/debug/kern
set disassemble-next-line auto
set riscv use-compressed-breakpoints yes