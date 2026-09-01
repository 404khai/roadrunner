//! Command-line entry point for Roadrunner.

fn main() {
    tracing::info!(
        core_version = roadrunner_core::version(),
        "Roadrunner CLI initialized"
    );
}
