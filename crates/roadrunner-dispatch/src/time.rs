use roadrunner_core::geo::Seconds;
use serde::Serialize;
use thiserror::Error;

/// Logical instant in the caller's declared dispatch time domain.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
pub struct DispatchInstant(Seconds);

/// Checked time construction, arithmetic, or epoch conversion failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("invalid dispatch time or routing epoch conversion")]
pub struct DispatchTimeError;

impl DispatchInstant {
    /// Creates a finite non-negative instant in seconds since dispatch origin.
    ///
    /// # Errors
    /// Rejects negative and non-finite values.
    pub fn new(seconds: f64) -> Result<Self, DispatchTimeError> {
        Seconds::new(seconds)
            .map(Self)
            .map_err(|_| DispatchTimeError)
    }

    /// Returns the numeric logical timestamp, not a duration.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0.value()
    }

    /// Advances an instant by a duration without clamping.
    ///
    /// # Errors
    /// Rejects overflow and a positive duration lost to floating-point precision.
    pub fn checked_add(self, duration: Seconds) -> Result<Self, DispatchTimeError> {
        let next = self
            .0
            .checked_add(duration)
            .map_err(|_| DispatchTimeError)?;
        if duration > Seconds::ZERO && next <= self.0 {
            return Err(DispatchTimeError);
        }
        Ok(Self(next))
    }

    /// Returns elapsed duration from an earlier instant.
    ///
    /// # Errors
    /// Rejects reversed instants or invalid subtraction.
    pub fn duration_since(self, earlier: Self) -> Result<Seconds, DispatchTimeError> {
        Seconds::new(self.value() - earlier.value()).map_err(|_| DispatchTimeError)
    }
}

/// Dispatch instant corresponding to routing/scenario departure second zero.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RoutingEpoch(pub DispatchInstant);

impl RoutingEpoch {
    /// Centralized checked conversion to routing's logical departure seconds.
    ///
    /// # Errors
    /// Rejects departure before the scenario epoch; never silently clamps.
    pub fn departure_seconds(self, at: DispatchInstant) -> Result<Seconds, DispatchTimeError> {
        at.duration_since(self.0)
    }
}
