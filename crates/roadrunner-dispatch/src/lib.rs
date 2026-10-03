//! Deterministic dispatch over coherent snapshots and logical remaining work.
//!
//! Evaluation is read-only. World transitions validate complete replacement state
//! before publication. Basic Dispatch is an idle-rider policy, not a domain limit.

mod decision;
mod domain;
mod evaluation;
mod identity;
mod routing;
mod time;
mod world;

pub mod spatial;

pub use decision::*;
pub use domain::*;
pub use evaluation::*;
pub use identity::RiderId;
pub use routing::*;
pub use time::*;
pub use world::*;
