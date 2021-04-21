use std::path::Path;

/// Adds an external dependency on something. This is used so our .d files have
/// absolute paths in them and don't fuck up make with our horrifying workspace
/// disaster.
pub fn external_dep(src: &str) {
    let trampoline_abs = match Path::new(src).canonicalize() {
        Ok(v) => v,
        Err(e) => panic!("failed to canonicalize {:?}, {:?}", src, e),
    };
    let trampoline_abs = trampoline_abs.to_str().unwrap();
    println!("cargo:rerun-if-changed={}", trampoline_abs);
}
