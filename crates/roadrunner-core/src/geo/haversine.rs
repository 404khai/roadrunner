use super::{Coordinate, Meters};

pub(crate) const MEAN_EARTH_RADIUS_METERS: f64 = 6_371_008.8;

/// Calculates the great-circle distance between two WGS 84 coordinates.
///
/// The result uses the Haversine formula and the mean Earth radius. It represents
/// straight-line surface distance, not road-network travel distance.
#[must_use]
pub fn haversine_distance(a: Coordinate, b: Coordinate) -> Meters {
    let latitude_a = a.latitude().to_radians();
    let latitude_b = b.latitude().to_radians();
    let latitude_delta = (b.latitude() - a.latitude()).to_radians();
    let longitude_delta = (b.longitude() - a.longitude()).to_radians();

    let half_latitude_sine = (latitude_delta / 2.0).sin();
    let half_longitude_sine = (longitude_delta / 2.0).sin();
    let haversine = half_latitude_sine.mul_add(
        half_latitude_sine,
        latitude_a.cos() * latitude_b.cos() * half_longitude_sine.powi(2),
    );
    let bounded_haversine = haversine.clamp(0.0, 1.0);
    let central_angle = 2.0
        * bounded_haversine
            .sqrt()
            .atan2((1.0 - bounded_haversine).sqrt());

    Meters::from_calculation(MEAN_EARTH_RADIUS_METERS * central_angle)
}

#[cfg(test)]
mod tests {
    use std::f64::consts::FRAC_PI_2;

    use super::*;

    fn coordinate(latitude: f64, longitude: f64) -> Coordinate {
        let result = Coordinate::new(latitude, longitude);
        let Ok(coordinate) = result else {
            panic!("expected test coordinate to be valid: {result:?}");
        };
        coordinate
    }

    fn assert_distance_close(actual: Meters, expected: f64, tolerance: f64) {
        let difference = (actual.value() - expected).abs();
        assert!(
            difference <= tolerance,
            "expected {expected} ± {tolerance} meters, got {actual}"
        );
    }

    #[test]
    fn identical_coordinates_have_zero_distance() {
        let lagos = coordinate(6.5244, 3.3792);

        assert_eq!(haversine_distance(lagos, lagos), Meters::ZERO);
    }

    #[test]
    fn equatorial_quarter_circumference_matches_the_mean_radius() {
        let distance = haversine_distance(Coordinate::ORIGIN, coordinate(0.0, 90.0));

        assert_distance_close(distance, MEAN_EARTH_RADIUS_METERS * FRAC_PI_2, 0.001);
    }

    #[test]
    fn london_to_paris_matches_a_known_distance() {
        let london = coordinate(51.5074, -0.1278);
        let paris = coordinate(48.8566, 2.3522);

        assert_distance_close(haversine_distance(london, paris), 343_556.0, 100.0);
    }

    #[test]
    fn distance_is_symmetric() {
        let lagos = coordinate(6.5244, 3.3792);
        let abuja = coordinate(9.0765, 7.3986);

        assert_eq!(
            haversine_distance(lagos, abuja),
            haversine_distance(abuja, lagos)
        );
    }
}
