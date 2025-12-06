use std::{
    borrow::Cow,
    fmt,
    ops::{Add, Div, Mul, Sub},
    str::FromStr,
};

use candid::Nat;
use ic_stable_structures::{storable::Bound, Storable};
use num::{BigRational, BigUint};
use num_bigint::ToBigInt;
use serde::{Deserialize, Serialize};

use crate::storable_nat::StorableNat;

// Wrapper around candid::Rational that implements Storable
#[derive(candid::CandidType, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Hash)]
pub struct StorableRational(StorableNat, StorableNat);

impl StorableRational {
    fn reduced(self) -> StorableRational {
        BigRational::from(self).reduced().into()
    }

    pub fn reversed(self) -> StorableRational {
        StorableRational(self.1, self.0)
    }

    pub fn to_f64(&self) -> f64 {
        self.0.to_f64() / self.1.to_f64()
    }

    pub fn to_f64_decimals(&self, num_decimals: u8, denom_decimals: u8) -> f64 {
        let new_num = StorableRational::new(self.0.0.clone(), Nat(BigUint::from(10u32).pow(num_decimals as u32))).unwrap();
        let new_denom = StorableRational::new(self.1.0.clone(), Nat(BigUint::from(10u32).pow(denom_decimals as u32))).unwrap();

        (new_num / new_denom).to_f64()
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn new(numerator: Nat, denominator: Nat) -> Result<StorableRational, String> {
        let denominator = StorableNat(denominator);
        let numerator = StorableNat(numerator);
        if denominator.is_zero() {
            return Err("Zero denominator".to_string());
        }

        Ok(StorableRational(numerator, denominator).reduced())
    }

    pub fn new_nat(numerator: Nat) -> StorableRational {
        Self::new(Nat::from(numerator), Nat::from(1u32)).unwrap()
    }

    pub fn new_u64(numerator: u64, denominator: u64) -> Result<StorableRational, String> {
        Self::new(Nat::from(numerator), Nat::from(denominator))
    }

    pub fn one() -> StorableRational {
        Self::new_u64(1, 1).unwrap()
    }

    pub fn new_str(s: &str) -> Result<StorableRational, String> {
        fn parse_rational(s: &str) -> Result<StorableRational, String> {
            let splitted: Vec<&str> = s.split('.').collect();
            if splitted.len() > 2 {
                return Err("Failed to parse rational: unexpected number of '/'".to_string());
            }

            let int_part = Nat::from_str(splitted[0]).map_err(|e| e.to_string())?;

            let mul = if splitted.len() == 2 {
                Nat(BigUint::from(10u32).pow(splitted[1].len() as u32))
            } else {
                Nat(BigUint::from(1u32))
            };

            let fract_part = if splitted.len() == 2 {
                Nat::from_str(splitted[1]).map_err(|e| e.to_string())?
            } else {
                Nat::from(0u32)
            };

            StorableRational::new(int_part * mul.clone() + fract_part, mul)
        }

        if s.contains('/') {
            let splitted: Vec<&str> = s.split('/').collect();
            if splitted.len() > 2 {
                return Err("Failed to parse rational: unexpected number of '/'".to_string());
            }

            let num = parse_rational(splitted[0])?;
            let denom = if splitted.len() == 2 {
                parse_rational(splitted[1])
            } else {
                StorableRational::new(Nat::from(1u32), Nat::from(1u32))
            }?;

            Ok(num / denom)
        } else {
            parse_rational(s)
        }
    }

    pub fn round_to_nat(&self) -> Nat {
        self.0.0.clone() / self.1.0.clone()
    } 
}

impl Default for StorableRational {
    fn default() -> Self {
        Self::new_u64(0, 1).unwrap()
    }
}

impl PartialOrd for StorableRational {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (self.0.clone() * other.1.clone()).partial_cmp(&(self.1.clone() * other.0.clone()))
    }
}

impl Ord for StorableRational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0.clone() * other.1.clone()).cmp(&(self.1.clone() * other.0.clone()))
    }
}

// Implement Display to allow .to_string() calls
impl fmt::Display for StorableRational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.0 .0.to_string(), self.1 .0.to_string())
    }
}

impl From<BigRational> for StorableRational {
    fn from(value: BigRational) -> Self {
        StorableRational(
            StorableNat(Nat::from(value.numer().to_biguint().unwrap())),
            StorableNat(Nat::from(value.denom().to_biguint().unwrap())),
        )
    }
}

impl From<StorableRational> for BigRational {
    fn from(value: StorableRational) -> Self {
        BigRational::new(value.0 .0 .0.to_bigint().unwrap(), value.1 .0 .0.to_bigint().unwrap())
    }
}

impl Add for StorableRational {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        (BigRational::from(self) + BigRational::from(other)).into()
    }
}

impl std::ops::AddAssign for StorableRational {
    fn add_assign(&mut self, other: Self) {
        let mut self_br = BigRational::from(self.clone());
        self_br += BigRational::from(other);

        *self = self_br.into();
    }
}

impl Sub for StorableRational {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        let res: Self = (BigRational::from(self) - BigRational::from(other)).into();
        if res >= Self::default() {
            res
        } else {
            Self::default()
        }
    }
}

impl Mul for StorableRational {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        (BigRational::from(self) * BigRational::from(other)).into()
    }
}

impl std::ops::MulAssign for StorableRational {
    fn mul_assign(&mut self, other: Self) {
        let mut self_br = BigRational::from(self.clone());
        self_br *= BigRational::from(other);

        *self = self_br.into();
    }
}

impl Div for StorableRational {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        (BigRational::from(self) / BigRational::from(other)).into()
    }
}

impl Storable for StorableRational {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(serde_json::to_vec(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        serde_json::from_slice(&bytes).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_str_delim() {
        let s1 = StorableRational::new_str("123/456").unwrap();
        assert_eq!(s1.0, StorableNat::from_u64(41));
        assert_eq!(s1.1, StorableNat::from_u64(152));
    }
    #[test]
    fn test_new_str_rat() {
        let s1 = StorableRational::new_str("123.456").unwrap();
        let res = BigRational::new(123456.into(), 1000.into());
        assert_eq!(s1.0, StorableNat(Nat(res.numer().to_biguint().unwrap())));
        assert_eq!(s1.1, StorableNat(Nat(res.denom().to_biguint().unwrap())));
    }

    #[test]
    fn test_new_str_rat2() {
        let s1 = StorableRational::new_str("123.1000").unwrap();
        assert_eq!(s1.0, StorableNat::from_u64(1231));
        assert_eq!(s1.1, StorableNat::from_u64(10));
    }

    #[test]
    fn test_new_str_rat3() {
        let s1 = StorableRational::new_str("0.0").unwrap();
        assert_eq!(s1.0, StorableNat::from_u64(0));
    }

    #[test]
    fn test_new_str_delim_rat() {
        let s1 = StorableRational::new_str("1/0.02").unwrap();
        assert_eq!(s1.0, StorableNat::from_u64(50));
        assert_eq!(s1.1, StorableNat::from_u64(1));
    }

    #[test]
    fn test_new_str_delim_rat2() {
        let s1 = StorableRational::new_str("1/0.200").unwrap();
        assert_eq!(s1.0, StorableNat::from_u64(5));
        assert_eq!(s1.1, StorableNat::from_u64(1));
    }

    #[test]
    fn invalid_cases() {
        let s1 = StorableRational::new_str("1/");
        assert_eq!(true, s1.is_err());

        let s2 = StorableRational::new_str("1.");
        assert_eq!(true, s2.is_err());

        let s3 = StorableRational::new_str(".");
        assert_eq!(true, s3.is_err());

        let s4 = StorableRational::new_str("/");
        assert_eq!(true, s4.is_err());

        let s5 = StorableRational::new_str("a");
        assert_eq!(true, s5.is_err());

        let s6 = StorableRational::new_str("1a");
        assert_eq!(true, s6.is_err());

        let s7 = StorableRational::new_str("-1");
        assert_eq!(true, s7.is_err());
    }
}
