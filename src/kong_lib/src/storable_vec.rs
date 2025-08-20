use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

// Wrapper around candid::Nat that implements Storable
#[derive(candid::CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct StorableVec<T>(pub Vec<T>);

impl<T> StorableVec<T>
{
    pub fn new() -> Self {
        StorableVec(Vec::new())
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T>
    {
        self.0.iter()
    }

    pub fn from_vec(v: Vec<T>) -> Self {
        StorableVec { 0: v }
    }

    pub fn inner(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T> Default for StorableVec<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> From<Vec<T>> for StorableVec<T>
where
    T: Storable,
{
    fn from(value: Vec<T>) -> Self {
        Self::from_vec(value)
    }
}

impl<T> Storable for StorableVec<T>
where
    T: Storable,
{
    fn to_bytes(&self) -> Cow<[u8]> {
        let prepared: Vec<Cow<[u8]>> = self.0.iter().map(|v| v.to_bytes()).collect();
        Cow::Owned(serde_cbor::to_vec(&prepared).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let vec: Vec<Cow<[u8]>> = match bytes {
            Cow::Borrowed(b) => serde_cbor::from_slice(b).unwrap(),
            Cow::Owned(o) => serde_cbor::from_slice(&o).unwrap(),
        };

        StorableVec::from_vec(vec.into_iter().map(|b| T::from_bytes(b)).collect())
    }

    const BOUND: Bound = Bound::Unbounded;
}
