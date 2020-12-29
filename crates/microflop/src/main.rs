#[cfg(feature = "cli")]
mod std_main;

#[cfg(feature = "cli")]
fn main() -> color_eyre::eyre::Result<()> {
    crate::std_main::main()
}

#[cfg(not(feature = "cli"))]
fn main() {
    panic!("main requires the cli feature");
}
