use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;

// Wrapper around HashMap that implements Storable
#[derive(Debug, Default, Clone, Serialize)]
pub struct StorableHashMap<K, V>
where
{
    pub data: HashMap<K, V>,
}


impl<K, V> StorableHashMap<K, V> {
    pub fn new() -> Self {
        StorableHashMap::<K, V>{
            data: HashMap::new()
        }
    }
}

// Implement Deserialize separately with the 'de lifetime
impl<'de, K, V> Deserialize<'de> for StorableHashMap<K, V>
where
    K: Deserialize<'de> + Eq + std::hash::Hash,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = HashMap::deserialize(deserializer)?;
        Ok(StorableHashMap { data })
    }
}

impl<K, V> Storable for StorableHashMap<K, V>
where
    K: Serialize + for<'a> Deserialize<'a> + Clone + std::hash::Hash + Eq,
    V: Serialize + for<'a> Deserialize<'a> + Clone,
{
    fn to_bytes(&self) -> Cow<[u8]> {
        serde_cbor::to_vec(&self.data).expect("Failed to encode StorableHashMap").into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let data = serde_cbor::from_slice(&bytes).expect("Failed to decode StableClaim");
        StorableHashMap::<K, V> { data }
    }

    const BOUND: Bound = Bound::Unbounded;
}
