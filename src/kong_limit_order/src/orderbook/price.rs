use std::cmp::Ordering;

use candid::CandidType;
use kong_lib::storable_rational::StorableRational;
use serde::{Deserialize, Serialize};


// We use a custom key for the BTreeMap to handle floating point prices correctly
#[derive(Debug, Clone, CandidType, Serialize, Deserialize)]
pub struct Price(pub StorableRational);

impl Price {
    pub fn new(val: StorableRational) -> Self {
        Price(val)
    }

    pub fn one() -> Self {
        Price(StorableRational::one())
    }

    pub fn reversed(&self) -> Self {
        Price(self.0.clone().reversed())
    }
}

impl Default for Price {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Eq for Price {}

impl PartialEq for Price {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Ord for Price {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for Price {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<StorableRational> for Price {
    fn from(value: StorableRational) -> Self {
        Self::new(value)
    }
}


impl std::iter::Sum for Price {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Price::default(), |a, b| Price(a.0 + b.0))
    }
}