use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident, $repr:ty) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[repr(transparent)]
        #[serde(transparent)]
        pub struct $name($repr);

        impl $name {
            /// Creates an identifier from its canonical integer value.
            #[must_use]
            pub const fn new(value: $repr) -> Self { Self(value) }

            /// Returns the canonical integer value.
            #[must_use]
            pub const fn value(self) -> $repr { self.0 }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

id_type!(
    /// Identity of an immutable graph snapshot.
    GraphSnapshotId,
    u64
);
id_type!(
    /// Snapshot-local dense node identity.
    NodeId,
    u32
);
id_type!(
    /// Snapshot-local dense physical road-segment identity.
    RoadSegmentId,
    u32
);
id_type!(
    /// Snapshot-local dense directed-edge identity.
    EdgeId,
    u32
);
id_type!(
    /// Deterministic construction-time node key.
    BuilderNodeId,
    u64
);
id_type!(
    /// Deterministic construction-time segment key.
    BuilderSegmentId,
    u64
);
