//! Build script.
//!
//! Links the C++ runtime on FreeBSD. The vendored Clipper2 sources in
//! `clipper2c-sys` are C++ but its build script does not request a C++
//! runtime on FreeBSD (FreeBSD uses libc++), which fails the final
//! link of the `raygeo` cdylib with undefined `std::` symbols.
//! Link flags from every crate in the graph are aggregated into the
//! final link command, so emitting the flag here fixes the cdylib
//! without patching the upstream crate.
//!
//! See rayforge issue #389.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("freebsd") {
        println!("cargo:rustc-link-lib=dylib=c++");
    }
}
