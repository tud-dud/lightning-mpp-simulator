// Mostly from https://git.tu-berlin.de/rohrer/cdt-data/-/blob/master/simulator/lnsim/src/simtime.rs
use std::cmp::Ordering;
use std::ops::{Add, AddAssign, Sub};

use std::fmt;

#[derive(Debug, Copy, Clone)]
pub struct Time(u64);

static SIMTIME_SCALING_FACTOR_SECS: f32 = 1000000.0; // in nano secs.
static SIMTIME_SCALING_FACTOR_MILLIS: f32 = 1000.0;

impl Time {
    pub(crate) fn as_secs(&self) -> f32 {
        self.0 as f32 / SIMTIME_SCALING_FACTOR_SECS
    }

    #[allow(unused)]
    pub fn as_millis(&self) -> f32 {
        self.0 as f32 / SIMTIME_SCALING_FACTOR_MILLIS
    }

    #[allow(unused)]
    pub fn as_nanos(&self) -> f32 {
        self.0 as f32
    }

    pub fn from_secs(secs: f32) -> Self {
        let nanos = secs * SIMTIME_SCALING_FACTOR_SECS;
        Time(nanos as u64)
    }

    pub fn from_millis(millis: f32) -> Self {
        let nanos = millis * SIMTIME_SCALING_FACTOR_MILLIS;
        Time(nanos as u64)
    }

    #[allow(unused)]
    pub fn from_nanos(nanos: f32) -> Self {
        Time(nanos as u64)
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Time {}

impl Add for Time {
    type Output = Time;
    fn add(self, other: Time) -> Self {
        Self(self.0 + other.0)
    }
}

impl AddAssign for Time {
    fn add_assign(&mut self, other: Self) {
        *self = Self(self.0 + other.0);
    }
}

impl Sub for Time {
    type Output = Time;
    fn sub(self, other: Time) -> Self {
        if other.0 > self.0 {
            return Self(0);
        }
        Self(self.0 - other.0)
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_works() {
        let secs = 1.0;
        let millis = 1000.0;
        let s0 = Time::from_secs(secs);
        let s1 = Time::from_millis(millis);
        assert_eq!(millis, s0.as_millis());
        assert_eq!(secs, s0.as_secs());
        assert_eq!(s0, s1);
    }

    #[test]
    fn cmp_time() {
        let smaller_millis = 1000.0;
        let greater_millis = 2000.0;
        let s_smaller = Time::from_millis(smaller_millis);
        let s_greater = Time::from_millis(greater_millis);
        assert!(s_smaller < s_greater);
        assert_ne!(s_smaller, s_greater);
    }

    #[test]
    fn addition() {
        let s0 = Time::from_secs(1.0);
        let s1 = Time::from_secs(2.0);
        let s2 = s0 + s1;
        assert_eq!(3.0, s2.as_secs());
        assert_eq!(s2.as_secs(), s0.as_secs() + s1.as_secs());
    }

    #[test]
    fn substraction() {
        let s0 = Time::from_secs(1.0);
        let s1 = Time::from_secs(2.0);
        let s2 = s0 - s0;
        let s3 = s0 - s1;
        let s4 = s1 - s0;
        assert_eq!(0.0, s2.as_secs());
        assert_eq!(0.0, s3.as_secs());
        assert_eq!(1.0, s4.as_secs());
        assert_eq!(s4.as_secs(), s1.as_secs() - s0.as_secs());
    }
}
