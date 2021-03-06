use build_bits::external_dep;

fn main() {
    let mut builder = cc::Build::new();
    builder
        .compiler("clang")
        .no_default_flags(true) // since this is asm the default flags just screw us up
        .flag("--target=riscv64-none-unknown-elf") // clang uses different triples than rustc, lol
        .flag("-march=rv64imac")
        .flag("-mno-relax");

    // some weird linker error:
    // ld.lld: error: out/libinit.a(init.o):(.text+0x0): relocation
    // R_RISCV_ALIGN requires unimplemented linker relaxation; recompile
    // with -mno-relax

    builder
        .clone()
        .file("src/init.s")
        .file("src/vectors.s")
        .compile("shoo_asm");

    external_dep("build.rs");
    external_dep("src/init.s");
    external_dep("src/vectors.s");
    external_dep("shoo.ld");
}
