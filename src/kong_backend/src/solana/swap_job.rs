use candid::{CandidType, Deserialize, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use serde::Serialize;
use std::borrow::Cow;

#[derive(CandidType, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SwapJobId(pub u64);

impl Storable for SwapJobId {
    fn to_bytes(&self) -> Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode SwapJobId").into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode SwapJobId")
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 16,
        is_fixed_size: false,
    };
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy)]
pub enum SwapJobStatus {
    PendingVerification, // Payment verification in progress
    Pending,             // Job created, awaiting processing by kong_rpc
    Confirmed,           // Confirmed by kong_rpc as successful on Solana
    Failed,              // Failed (either Solana tx failed, or an internal error)
}

impl Storable for SwapJobStatus {
    fn to_bytes(&self) -> Cow<[u8]> {
        match self {
            SwapJobStatus::PendingVerification => Cow::Borrowed(&[0]),
            SwapJobStatus::Pending => Cow::Borrowed(&[1]),
            SwapJobStatus::Confirmed => Cow::Borrowed(&[2]),
            SwapJobStatus::Failed => Cow::Borrowed(&[3]),
        }
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        match bytes.first() {
            Some(&0) => SwapJobStatus::PendingVerification,
            Some(&1) => SwapJobStatus::Pending,
            Some(&2) => SwapJobStatus::Confirmed,
            Some(&3) => SwapJobStatus::Failed,
            _ => panic!("Invalid SwapJobStatus bytes"), // Or handle error appropriately
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 1,
        is_fixed_size: true,
    };
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct SwapJob {
    pub id: u64,
    pub caller: Principal,
    pub original_args_json: String,
    pub status: SwapJobStatus,
    pub created_at: u64, // ic_cdk::api::time()
    pub updated_at: u64,
    pub encoded_signed_solana_tx: String,
    pub solana_tx_signature_of_payout: Option<String>,
    pub error_message: Option<String>,
    pub attempts: u32,  // For kong_rpc retry logic
    pub tx_sig: String, // Transaction signature computed at signing time
}

impl Storable for SwapJob {
    fn to_bytes(&self) -> Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode SwapJob").into()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode SwapJob")
    }
    const BOUND: Bound = Bound::Unbounded;
}

impl SwapJob {
    pub fn new(
        id: u64,
        caller: Principal,
        original_args_json: String,
        status: SwapJobStatus,
        created_at: u64,
        updated_at: u64,
        encoded_signed_solana_tx: String,
        solana_tx_signature_of_payout: Option<String>,
        error_message: Option<String>,
        attempts: u32,
        tx_sig: String,
    ) -> Self {
        Self {
            id,
            caller,
            original_args_json,
            status,
            created_at,
            updated_at,
            encoded_signed_solana_tx,
            solana_tx_signature_of_payout,
            error_message,
            attempts,
            tx_sig,
        }
    }
}