//! Core domain and routing foundations for Roadrunner.
//!
//! Graph types and routing algorithms are intentionally introduced in their
//! dedicated implementation phases. This crate currently establishes the stable
//! library boundary shared by Roadrunner's adapters.

pub mod geo;
pub mod graph;

/// Returns the version of the Roadrunner core crate.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn reports_the_package_version() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
