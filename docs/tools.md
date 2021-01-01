# Tools

This page is basically just me pasting a bunch of crap in a page. But at
least it's checked in!!!

## binutils/gdb

// or as I'd like to call it, binutils plus gdb

We don't actually use this one, because gnu sucks. However, llvm steals all
their options and linker script format, so we still need their stuff.

Use this shell snippet to build all the html docs as single giant HTML files,
and as PDFs for pleasant browsing.

```bash
git clone git://sourceware.org/git/binutils-gdb.git
cd binutils-gdb
mkdir build
cd build
../configure
make html MAKEINFO=makeinfo MAKEINFOFLAGS='--no-split'
make pdf
find . '(' -name '*.pdf' -or -name '*.html' ')' -exec cp '{}' ~/dev/docs/gnu ';'
```

If you want to use this to build all the gnu stuff minus gdb, then, pass
`--disable-gdb` to configure, along with `--target=riscv64`.

```bash
./configure --prefix=/opt/gdb --enable-targets=all
make -j24
```

You may have issues with gdb/other bits:

```
  CXX    arch/aarch32.o
../../gdb/arch/aarch32.c:43:1: fatal error: opening dependency file arch/.deps/aarch32.Tpo: No such file or directory
   43 | }
      | ^
```

It means that there's some rubbish with a reconfigured source directory. You
can fix it with `git clean -fxd .` in the `binutils-gdb` root. Then, rebuild.

## qemu

Build from source, with this configure command:

Presumably from a `build` folder,

```bash
../configure --target-list=riscv64-softmmu,arm-linux-user,arm-softmmu --disable-sdl --disable-spice --disable-gtk --smbd= --disable-seccomp --disable-tpm --disable-rbd --disable-xen --disable-vnc --disable-libusb --disable-gnutls --prefix=/opt/qemu
```

You can get `ccls` info for qemu with:

```bash
ninja -t compdb > ../compile_commands.json
```

## llvm

From a build folder,

```bash
cmake -G 'Ninja' -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/opt/llvm '-DLLVM_ENABLE_PROJECTS=clang;libcxx;libunwind;lldb;compiler-rt;lld;libcxxabi' ../llvm
ninja -j16
```