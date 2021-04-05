fn main() {
    let mut builder = cc::Build::new();
    builder
        .compiler("/opt/llvm/bin/clang")
        .no_default_flags(true) // since this is asm the default flags just screw us up
        .flag("--target=riscv64-none-unknown-elf") // clang uses different triples than rustc, lol
        .flag("-march=rv64imac")
        .flag("-mno-relax");
    // some weird linker error:
    // ld.lld: error: out/libinit.a(init.o):(.text+0x0): relocation
    // R_RISCV_ALIGN requires unimplemented linker relaxation; recompile
    // with -mno-relax

    builder.clone().file("src/trampoline.s").compile("kern_asm");

    println!("cargo:rerun-if-changed=src/trampoline.s");
    println!("cargo:rerun-if-changed=kern.ld");
}
