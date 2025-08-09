//! Utility functions and types for the fncache crate.

use std::time::Duration;

/// Converts a Duration to seconds as f64
#[allow(dead_code)]
pub fn duration_to_secs(duration: Duration) -> f64 {
    duration.as_secs() as f64 + f64::from(duration.subsec_nanos()) * 1e-9
}

/// Converts seconds as f64 to a Duration
#[allow(dead_code)]
pub fn secs_to_duration(secs: f64) -> Duration {
    let secs = secs.max(0.0);
    let nanos = (secs.fract() * 1_000_000_000.0) as u32;
    Duration::new(secs as u64, nanos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_duration_conversion() {
        let duration = Duration::new(1, 500_000_000);
        let secs = duration_to_secs(duration);
        assert!((secs - 1.5).abs() < f64::EPSILON);

        let converted = secs_to_duration(secs);
        assert_eq!(duration, converted);
    }
}
