use candid::{decode_one, encode_one, CandidType, Deserialize, Principal};
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use std::borrow::Cow;
use std::time::{SystemTime, UNIX_EPOCH};

// We'll implement our own simple hash function since we don't have sha2
pub(crate) fn hash_principals(principals: &[Principal]) -> Vec<u8> {
    let mut result = Vec::new();
    for principal in principals {
        result.extend_from_slice(principal.as_slice());
    }
    result
}

// Error types
#[derive(CandidType, Deserialize, Debug)]
pub enum DelegationError {
    InvalidRequest(String),
    Expired,
    NotFound,
    StorageError(String),
    Unauthorized,
}

// ICRC-34 Types and Functions
#[derive(CandidType, Deserialize, Debug, Clone)]
pub struct DelegationRequest {
    pub targets: Vec<Principal>,
    pub expiration: Option<u64>, // Unix timestamp in nanoseconds
}

impl DelegationRequest {
    pub(crate) fn validate(&self) -> Result<(), DelegationError> {
        if self.targets.is_empty() {
            return Err(DelegationError::InvalidRequest("No targets specified".to_string()));
        }

        if let Some(exp) = self.expiration {
            let current_time = get_current_time();
            if exp <= current_time {
                return Err(DelegationError::InvalidRequest("Expiration time must be in the future".to_string()));
            }
        }

        Ok(())
    }

    pub(crate) fn compute_targets_hash(&self) -> Vec<u8> {
        let mut targets = self.targets.clone();
        targets.sort();
        hash_principals(&targets)
    }
}

#[derive(CandidType, Deserialize, Debug, Clone)]
pub struct DelegationResponse {
    pub delegations: Vec<Delegation>,
}

#[derive(CandidType, Deserialize, Debug, Clone)]
pub struct Delegation {
    pub target: Principal,
    pub created: u64,               // Unix timestamp in nanoseconds
    pub expiration: Option<u64>,    // Unix timestamp in nanoseconds
    pub targets_list_hash: Vec<u8>, // Hash of the sorted list of targets
}

impl Delegation {
    pub(crate) fn is_expired(&self) -> bool {
        if let Some(exp) = self.expiration {
            let current_time = get_current_time();
            exp <= current_time
        } else {
            false
        }
    }
}

// Implement Storable for Delegation
impl Storable for Delegation {
    fn to_bytes(&'_ self) -> Cow<'_, [u8]> {
        let bytes = candid::encode_one(self).unwrap();
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        candid::decode_one(&bytes).unwrap()
    }

    const BOUND: ic_stable_structures::storable::Bound = ic_stable_structures::storable::Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Debug)]
pub struct RevokeDelegationRequest {
    pub targets: Vec<Principal>,
}

// Helper function to get current time in nanoseconds
pub(crate) fn get_current_time() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
}

// Wrapper type for Vec<Delegation> that implements Storable
#[derive(Debug, Clone)]
pub(crate) struct DelegationVec(Vec<Delegation>);

impl DelegationVec {
    pub(crate) fn new() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn push(&mut self, delegation: Delegation) {
        self.0.push(delegation);
    }

    pub(crate) fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Delegation) -> bool,
    {
        self.0.retain(f);
    }

    #[allow(dead_code)]
    pub(crate) fn into_vec(self) -> Vec<Delegation> {
        self.0
    }

    pub(crate) fn as_vec(&self) -> &Vec<Delegation> {
        &self.0
    }
}

impl Default for DelegationVec {
    fn default() -> Self {
        Self::new()
    }
}

impl Storable for DelegationVec {
    fn to_bytes(&'_ self) -> Cow<'_, [u8]> {
        let bytes = encode_one(&self.0).unwrap();
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self(decode_one(&bytes).unwrap())
    }

    const BOUND: Bound = Bound::Unbounded;
}
