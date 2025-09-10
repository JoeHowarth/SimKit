//! Q40.24 fixed-point number with a signed 64-bit backing.
//! - 24 fractional bits, 40 integer bits (including sign)
//! - ~7.2 decimal digits of fractional precision
//! - Range ≈ [-2^39, 2^39)
//! - Fast, deterministic, Eq + Ord based on raw bits

#![allow(clippy::manual_range_contains)]
use core::{
    fmt,
    iter::{Product, Sum},
    ops::*,
};

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

pub type FP64 = Q40p24;

/// Q40.24 = i64 scaled by 2^24
#[derive(
    Copy,
    Clone,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Reflect,
    Serialize,
    Deserialize,
)]
#[repr(transparent)]
pub struct Q40p24(pub i64);

impl Q40p24 {
    pub const FRAC_BITS: u32 = 24;
    pub const SCALE: i64 = 1_i64 << Self::FRAC_BITS;
    pub const HALF: i64 = 1_i64 << (Self::FRAC_BITS - 1);
    pub const FRACTION_MASK: i64 = Self::SCALE - 1;

    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(Self::SCALE);
    pub const EPS: Self = Self(1);

    #[inline]
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }
    #[inline]
    pub const fn to_raw(self) -> i64 {
        self.0
    }

    #[inline]
    pub const fn from_int(n: i64) -> Self {
        Self(n << Self::FRAC_BITS)
    }
    #[inline]
    pub const fn trunc(self) -> i64 {
        self.0 >> Self::FRAC_BITS
    }

    /// Round to nearest integer, ties away from zero.
    #[inline]
    pub const fn round(self) -> i64 {
        let raw = if self.0 >= 0 {
            self.0 + Self::HALF
        } else {
            self.0 - Self::HALF
        };
        raw >> Self::FRAC_BITS
    }

    /// Floor toward -inf.
    #[inline]
    pub const fn floor(self) -> i64 {
        if self.0 >= 0 || (self.0 & Self::FRACTION_MASK) == 0 {
            self.trunc()
        } else {
            self.trunc() - 1
        }
    }

    /// Ceil toward +inf.
    #[inline]
    pub const fn ceil(self) -> i64 {
        if self.0 >= 0 {
            if (self.0 & Self::FRACTION_MASK) == 0 {
                self.trunc()
            } else {
                self.trunc() + 1
            }
        } else {
            self.trunc()
        }
    }
}

// ===== Arithmetic (wrapping add/sub, rounded mul/div) =====

impl Add for Q40p24 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0.wrapping_add(rhs.0))
    }
}
impl AddAssign for Q40p24 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_add(rhs.0);
    }
}

impl Sub for Q40p24 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.wrapping_sub(rhs.0))
    }
}
impl SubAssign for Q40p24 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.wrapping_sub(rhs.0);
    }
}

impl Neg for Q40p24 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self(self.0.wrapping_neg())
    }
}

impl Mul for Q40p24 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        // (a*b) >> FRAC, rounded to nearest, ties away from zero
        let prod = (self.0 as i128) * (rhs.0 as i128);
        let bias = if prod >= 0 {
            Q40p24::HALF as i128
        } else {
            -(Q40p24::HALF as i128)
        };
        Q40p24(((prod + bias) >> Q40p24::FRAC_BITS) as i64)
    }
}
impl MulAssign for Q40p24 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Div for Q40p24 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        assert!(rhs.0 != 0, "division by zero");
        // (a<<FRAC)/b rounded to nearest, ties away from zero
        let num = (self.0 as i128) << Q40p24::FRAC_BITS;
        let den = rhs.0 as i128;
        let q = num / den;
        let r = num % den;

        let needs_adjust = (r.abs() << 1) >= den.abs();
        let positive_quot = (num >= 0 && den >= 0) || (num < 0 && den < 0);
        let adj = if needs_adjust {
            if positive_quot { 1 } else { -1 }
        } else {
            0
        };

        Q40p24((q + adj) as i64)
    }
}
impl DivAssign for Q40p24 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

/// Remainder using integer semantics on raw storage (trunc toward zero
/// division).
impl Rem for Q40p24 {
    type Output = Self;
    #[inline]
    fn rem(self, rhs: Self) -> Self {
        Q40p24(self.0 % rhs.0)
    }
}
impl RemAssign for Q40p24 {
    #[inline]
    fn rem_assign(&mut self, rhs: Self) {
        self.0 %= rhs.0;
    }
}

// ===== Iterator traits =====

impl Sum for Q40p24 {
    #[inline]
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut acc = Q40p24::ZERO;
        for v in iter {
            acc += v;
        }
        acc
    }
}
impl Product for Q40p24 {
    #[inline]
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut acc = Q40p24::ONE;
        for v in iter {
            acc *= v;
        }
        acc
    }
}

// ===== Conversions =====

impl From<i8> for Q40p24 {
    #[inline]
    fn from(v: i8) -> Self {
        Q40p24::from_int(v as i64)
    }
}
impl From<i16> for Q40p24 {
    #[inline]
    fn from(v: i16) -> Self {
        Q40p24::from_int(v as i64)
    }
}
impl From<u32> for Q40p24 {
    #[inline]
    fn from(v: u32) -> Self {
        Q40p24::from_int(v as i64)
    }
}
impl From<i32> for Q40p24 {
    #[inline]
    fn from(v: i32) -> Self {
        Q40p24::from_int(v as i64)
    }
}
impl From<i64> for Q40p24 {
    #[inline]
    fn from(v: i64) -> Self {
        Q40p24::from_int(v)
    }
}

impl TryFrom<u64> for Q40p24 {
    type Error = &'static str;
    #[inline]
    fn try_from(v: u64) -> Result<Self, Self::Error> {
        if v <= ((i64::MAX as u64) >> Q40p24::FRAC_BITS) {
            Ok(Q40p24::from_int(v as i64))
        } else {
            Err("u64 value out of range for Q40p24")
        }
    }
}

impl From<Q40p24> for u64 {
    #[inline]
    fn from(v: Q40p24) -> u64 {
        v.round().max(0) as u64
    }
}

impl From<Q40p24> for i64 {
    #[inline]
    fn from(v: Q40p24) -> i64 {
        v.round()
    }
}
impl From<Q40p24> for i32 {
    #[inline]
    fn from(v: Q40p24) -> i32 {
        v.round() as i32
    }
}

impl From<f32> for Q40p24 {
    #[inline]
    fn from(v: f32) -> Self {
        // round to nearest, ties away from zero
        let scaled = (v as f64) * (Q40p24::SCALE as f64);
        let raw = if scaled >= 0.0 {
            (scaled + 0.5).floor()
        } else {
            (scaled - 0.5).ceil()
        } as i64;
        Q40p24(raw)
    }
}
impl From<f64> for Q40p24 {
    #[inline]
    fn from(v: f64) -> Self {
        let scaled = v * (Q40p24::SCALE as f64);
        let raw = if scaled >= 0.0 {
            (scaled + 0.5).floor()
        } else {
            (scaled - 0.5).ceil()
        } as i64;
        Q40p24(raw)
    }
}

impl From<Q40p24> for f32 {
    #[inline]
    fn from(v: Q40p24) -> f32 {
        (v.0 as f64 / Q40p24::SCALE as f64) as f32
    }
}
impl From<Q40p24> for f64 {
    #[inline]
    fn from(v: Q40p24) -> f64 {
        v.0 as f64 / Q40p24::SCALE as f64
    }
}

// ===== Formatting =====

impl fmt::Debug for Q40p24 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Show both raw and human value
        write!(f, "Q40p24{{raw:{}, val:{}}}", self.0, self)
    }
}

impl fmt::Display for Q40p24 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Fast decimal print with 6 fractional digits, rounded
        let raw = self.0;
        if raw == 0 {
            return f.write_str("0");
        }

        let neg = raw < 0;
        let abs = if neg { raw.wrapping_neg() } else { raw };

        let mut int_part: i64 = abs >> Q40p24::FRAC_BITS;
        let frac_raw = abs & Q40p24::FRACTION_MASK;

        // scale fractional to 6 digits with rounding
        let mut frac6 = (((frac_raw as i128) * 1_000_000i128)
            + (1i128 << (Q40p24::FRAC_BITS - 1)))
            >> Q40p24::FRAC_BITS;
        if frac6 == 1_000_000 {
            frac6 = 0;
            int_part = int_part.saturating_add(1);
        }

        if neg {
            write!(f, "-")?;
        }
        write!(f, "{}.{:06}", int_part, frac6 as i64)
    }
}

// ===== Convenience ops with integers =====

impl Add<u32> for Q40p24 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: u32) -> Self {
        self + Q40p24::from_int(rhs as i64)
    }
}
impl Sub<u32> for Q40p24 {
    type Output = Self;

    fn sub(self, rhs: u32) -> Self::Output {
        self - Q40p24::from_int(rhs as i64)
    }
}
impl AddAssign<u32> for Q40p24 {
    #[inline]
    fn add_assign(&mut self, rhs: u32) {
        *self = *self + rhs;
    }
}
impl SubAssign<u32> for Q40p24 {
    #[inline]
    fn sub_assign(&mut self, rhs: u32) {
        *self = *self - rhs;
    }
}
impl Add<i64> for Q40p24 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: i64) -> Self {
        self + Q40p24::from_int(rhs)
    }
}
impl Sub<i64> for Q40p24 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: i64) -> Self {
        self - Q40p24::from_int(rhs)
    }
}
impl Mul<u32> for Q40p24 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: u32) -> Self {
        self * Q40p24::from_int(rhs as i64)
    }
}
impl Mul<i32> for Q40p24 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i32) -> Self {
        self * Q40p24::from_int(rhs as i64)
    }
}
impl Mul<i64> for Q40p24 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i64) -> Self {
        self * Q40p24::from_int(rhs)
    }
}
impl Div<i64> for Q40p24 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i64) -> Self {
        self / Q40p24::from_int(rhs)
    }
}
impl AddAssign<i64> for Q40p24 {
    #[inline]
    fn add_assign(&mut self, rhs: i64) {
        *self = *self + rhs;
    }
}
impl SubAssign<i64> for Q40p24 {
    #[inline]
    fn sub_assign(&mut self, rhs: i64) {
        *self = *self - rhs;
    }
}
impl MulAssign<i64> for Q40p24 {
    #[inline]
    fn mul_assign(&mut self, rhs: i64) {
        *self = *self * rhs;
    }
}
impl DivAssign<i64> for Q40p24 {
    #[inline]
    fn div_assign(&mut self, rhs: i64) {
        *self = *self / rhs;
    }
}

// ===== Minimal tests =====
#[cfg(test)]
mod tests {
    use super::Q40p24 as Q;

    #[test]
    fn basics() {
        let a = Q::from_int(2);
        let b = Q::from_int(3);
        assert_eq!(i64::from(a + b), 5);
        assert_eq!(i64::from(b - a), 1);
        assert_eq!(i64::from(a * b), 6);
        assert_eq!(i64::from(b / a), 2); // 3/2 rounded to nearest -> 2
    }

    #[test]
    fn roundtrip_float() {
        let x = 1234.125f64;
        let q = Q::from(x);
        let y: f64 = q.into();
        assert!((y - x).abs() < 1e-6);
    }

    #[test]
    fn display() {
        let q = Q::from_int(1) + Q::from_raw(500_000); // ~1.0298
        let s = q.to_string();
        assert!(s.starts_with("1."));
    }
}
