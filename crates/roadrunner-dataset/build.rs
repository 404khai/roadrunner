//! Retain the compiler actually building this extractor, independent of PATH at export.
use std::{env, process::Command};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for (tool, args, key) in [
        ("RUSTC", vec!["-vV"], "ROADRUNNER_DATASET_RUSTC"),
        ("CARGO", vec!["--version"], "ROADRUNNER_DATASET_CARGO"),
    ] {
        let output = Command::new(env::var_os(tool).ok_or("missing build tool path")?)
            .args(args)
            .output()?;
        if !output.status.success() {
            return Err("build tool identity unavailable".into());
        }
        let identity = String::from_utf8(output.stdout)?.replace('\n', " | ");
        println!("cargo:rustc-env={key}={identity}");
    }
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
