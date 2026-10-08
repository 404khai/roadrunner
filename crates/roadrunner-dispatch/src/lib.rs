//! Deterministic dispatch over coherent snapshots and logical remaining work.
//!
//! Evaluation is read-only. World transitions validate complete replacement state
//! before publication. Basic and preparation-aware dispatch share idle-rider eligibility,
//! feasible plan evaluation, and atomic commit; that policy is not a domain limit.
//! Fleet batch construction/local search preserves committed owners and frozen execution.

mod decision;
mod domain;
mod evaluation;
mod fleet;
mod identity;
mod pooling;
mod preparation;
mod routing;
mod time;
mod world;

pub mod spatial;

pub use decision::*;
pub use domain::*;
pub use evaluation::*;
pub use fleet::*;
pub use identity::RiderId;
pub use pooling::*;
pub use preparation::*;
pub use routing::*;
pub use time::*;
pub use world::*;
