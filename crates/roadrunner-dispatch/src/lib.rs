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
mod history;
mod identity;
mod operational;
mod pooling;
mod preparation;
mod recovery;
mod routing;
mod temporal;
mod time;
mod world;

pub mod spatial;

pub use decision::*;
pub use domain::*;
pub use evaluation::*;
pub use fleet::*;
pub use history::*;
pub use identity::RiderId;
pub use operational::*;
pub use pooling::*;
pub use preparation::*;
pub use recovery::*;
pub use routing::*;
pub use temporal::*;
pub use time::*;
pub use world::*;
