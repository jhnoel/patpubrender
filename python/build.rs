fn main() {
    // A pyo3 extension module leaves CPython symbols undefined, to be resolved
    // by the host interpreter at load time. On macOS the linker must be told to
    // allow that. Emitting the args here (rather than in .cargo/config.toml)
    // makes it work regardless of the build's working directory — notably when
    // pip builds from the sdist, where cargo runs from a different cwd and a
    // crate-local .cargo/config.toml would not be discovered.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg-cdylib=-undefined");
        println!("cargo:rustc-link-arg-cdylib=dynamic_lookup");
    }
}
