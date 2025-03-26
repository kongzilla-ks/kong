use candid::{CandidType, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};

use crate::ic::{
    canister_address::{KONG_BACKEND, KONG_DATA},
    ckusdt::{CKUSDT_ADDRESS, CKUSDT_ADDRESS_WITH_CHAIN, CKUSDT_SYMBOL, CKUSDT_SYMBOL_WITH_CHAIN, CKUSDT_TOKEN_ID},
    icp::{ICP_ADDRESS, ICP_ADDRESS_WITH_CHAIN, ICP_SYMBOL, ICP_SYMBOL_WITH_CHAIN, ICP_TOKEN_ID},
};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct StableKongSettings {
    pub kong_backend: Account,
    pub kong_data: Principal,
    pub maintenance_mode: bool,
    pub kingkong: Vec<u32>, // list of user_ids of maintainers
    pub ckusdt_token_id: u32,
    pub ckusdt_symbol: String,
    pub ckusdt_symbol_with_chain: String,
    pub ckusdt_address: String,
    pub ckusdt_address_with_chain: String,
    pub icp_token_id: u32,
    pub icp_symbol: String,
    pub icp_symbol_with_chain: String,
    pub icp_address: String,
    pub icp_address_with_chain: String,
    pub default_max_slippage: f64,
    pub default_lp_fee_bps: u8,
    pub default_kong_fee_bps: u8,
    pub user_map_idx: u32,     // counter for USER_MAP
    pub token_map_idx: u32,    // counter for TOKEN_MAP
    pub pool_map_idx: u32,     // counter for POOL_MAP
    pub tx_map_idx: u64,       // counter for TX_MAP
    pub request_map_idx: u64,  // counter for REQUEST_MAP
    pub transfer_map_idx: u64, // counter for TRANSFER_MAP
    pub claim_map_idx: u64,    // counter for CLAIM_MAP
    pub lp_token_map_idx: u64, // counter for LP_TOKEN_MAP
    pub claims_interval_secs: u64,
    pub transfer_expiry_nanosecs: u64,
    pub requests_archive_interval_secs: u64,
    pub txs_archive_interval_secs: u64,
    pub transfers_archive_interval_secs: u64,
    pub archive_to_kong_data: bool,
}

impl Default for StableKongSettings {
    fn default() -> Self {
        let user_map_idx = 0;
        let token_map_idx = 0;
        let pool_map_idx = 0;
        let tx_map_idx = 0;
        let request_map_idx = 0;
        let transfer_map_idx = 0;
        let claim_map_idx = 0;
        let lp_token_map_idx = 0;
        Self {
            kong_backend: Account::from(Principal::from_text(KONG_BACKEND).unwrap()),
            kong_data: Principal::from_text(KONG_DATA).unwrap(),
            maintenance_mode: false,
            kingkong: vec![100, 101], // default kingkong users
            ckusdt_token_id: CKUSDT_TOKEN_ID,
            ckusdt_symbol: CKUSDT_SYMBOL.to_string(),
            ckusdt_symbol_with_chain: CKUSDT_SYMBOL_WITH_CHAIN.to_string(),
            ckusdt_address: CKUSDT_ADDRESS.to_string(),
            ckusdt_address_with_chain: CKUSDT_ADDRESS_WITH_CHAIN.to_string(),
            icp_token_id: ICP_TOKEN_ID,
            icp_symbol: ICP_SYMBOL.to_string(),
            icp_symbol_with_chain: ICP_SYMBOL_WITH_CHAIN.to_string(),
            icp_address: ICP_ADDRESS.to_string(),
            icp_address_with_chain: ICP_ADDRESS_WITH_CHAIN.to_string(),
            default_max_slippage: 2.0_f64,
            default_lp_fee_bps: 30,
            default_kong_fee_bps: 0,
            user_map_idx,
            token_map_idx,
            pool_map_idx,
            tx_map_idx,
            request_map_idx,
            transfer_map_idx,
            claim_map_idx,
            lp_token_map_idx,
            claims_interval_secs: 300,                   // claims every 5 minutes
            transfer_expiry_nanosecs: 3_600_000_000_000, // 1 hour (nano seconds)
            requests_archive_interval_secs: 3600,        // archive requests every hour
            txs_archive_interval_secs: 3600,             // archive txs every hour
            transfers_archive_interval_secs: 3600,       // archive transfers every hour
            archive_to_kong_data: true,                  // replicate to kong_data
        }
    }
}

impl Storable for StableKongSettings {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).unwrap().into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        // if any error occurs retreiving the StableKongSettings, use the default
        serde_cbor::from_slice(&bytes).unwrap_or_default()
    }

    const BOUND: Bound = Bound::Unbounded;
}
