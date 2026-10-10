//! Explicit local trust and startup failure boundaries.
#![allow(clippy::unwrap_used)]
use std::process::Command;

#[test]
fn entrypoint_requires_explicit_trust_loopback_and_a_credential() {
    let binary = env!("CARGO_BIN_EXE_roadrunner-api");
    let unspecified = Command::new(binary).output().unwrap();
    assert!(!unspecified.status.success());
    assert!(String::from_utf8_lossy(&unspecified.stderr).contains("--trusted-local"));
    let exposed = Command::new(binary)
        .args(["--trusted-local", "unused.rr-graph"])
        .env("ROADRUNNER_API_BIND", "0.0.0.0:3000")
        .env_remove("ROADRUNNER_API_TOKEN")
        .output()
        .unwrap();
    assert!(!exposed.status.success());
    assert!(String::from_utf8_lossy(&exposed.stderr).contains("loopback"));
    let missing = Command::new(binary)
        .args(["--trusted-local", "unused.rr-graph"])
        .env_remove("ROADRUNNER_API_TOKEN")
        .env_remove("ROADRUNNER_API_BIND")
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("ROADRUNNER_API_TOKEN is required"));
}
