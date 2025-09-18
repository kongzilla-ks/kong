use std::{cell::RefCell, collections::HashMap};

use candid::{Nat, Principal};
use kong_lib::{ic::address::Address, token_management::send};

use crate::{
    stable_memory_helpers::{get_and_inc_next_claim_id, get_token_by_symbol},
    token_management::claim::{Claim, ClaimStatus},
};

thread_local! {
    pub(crate) static USER_CLAIMS: RefCell<HashMap<Principal, Vec<u64>>> = RefCell::new(HashMap::new());
    pub(crate) static CLAIMS: RefCell<HashMap<u64, Claim>> = RefCell::new(HashMap::new());
}

pub fn get_by_claim_id(claim_id: u64) -> Option<Claim> {
    CLAIMS.with(|claims| claims.borrow().get(&claim_id).cloned())
}

pub fn insert(claim: Claim) {
    USER_CLAIMS.with_borrow_mut(|user_claims| {
        user_claims.entry(claim.user.clone()).or_insert_with(Vec::new).push(claim.claim_id);
    });

    CLAIMS.with_borrow_mut(|claims| {
        claims.insert(claim.claim_id, claim);
    })
}

pub fn create_and_insert(user: Principal, token_symbol: String, amount: Nat, to_address: Option<Address>) -> u64 {
    let mut claim = Claim::new(user, token_symbol, &amount, to_address);
    let claim_id = get_and_inc_next_claim_id();
    claim.claim_id = claim_id;
    insert(claim);
    claim_id
}

pub fn create_insert_and_try_to_execute(user: Principal, token_symbol: String, amount: Nat, to_address: Option<Address>) -> u64 {
    let claim_id = create_and_insert(user, token_symbol, amount, to_address);
    ic_cdk::futures::spawn(async move { 
        let _ = process_claim(claim_id).await;
    });
    claim_id
}

pub fn update_status(claim_id: u64, status: ClaimStatus) -> Option<Claim> {
    CLAIMS.with(|m| {
        let mut map = m.borrow_mut();
        match map.get_mut(&claim_id) {
            Some(v) => {
                v.status = status;
                Some(v.clone())
            }
            None => None,
        }
    })
}

pub fn get_all_claims() -> HashMap<u64, Claim> {
    let mut res = HashMap::new();
    CLAIMS.with_borrow(|active_claims| {
        for (id, claim) in active_claims {
            res.insert(*id, claim.clone());
        }
    });

    // FINISHED_CLAIMS.with_borrow(|active_claims| {
    //     for (id, claim) in active_claims {
    //         res.insert(*id, claim.clone());
    //     }
    // });

    res
}

pub async fn process_claim(claim_id: u64) -> Result<(), String> {
    let claim = CLAIMS.with_borrow_mut(|claims| match claims.get_mut(&claim_id) {
        Some(claim) => {
            if !claim.status.is_active() {
                return Err("Status not active".to_string());
            }

            if claim.status == ClaimStatus::Claiming {
                return Err("Status is claiming".to_string());
            }

            claim.status = ClaimStatus::Claiming;
            Ok(claim.clone())
        }
        _ => Err("Claim not found".to_string()),
    })?;

    let token = match get_token_by_symbol(&claim.token_symbol) {
        Some(token) => token,
        None => return Err(format!("Token {} not found", claim.token_symbol)),
    };

    let address = send::get_address_to_send(claim.user, claim.to_address.clone(), &token);

    let success = match send::send(&claim.amount, &address, &token, None).await {
        Ok(_) => true,
        Err(e) => {
            ic_cdk::eprintln!("Process claim, id={}, process amount={}, error={}", claim_id, claim.amount, e);
            false
        }
    };

    CLAIMS.with_borrow_mut(|claims| match claims.get_mut(&claim_id) {
        Some(claim) => {
            if success {
                claim.status = ClaimStatus::Claimed;
            } else {
                claim.failed_attempts += 1;
                if claim.failed_attempts >= 5 {
                    claim.status = ClaimStatus::Failed("Too many failed attempts".to_string());
                } else {
                    claim.status = ClaimStatus::Unclaimed;
                }
            }
            Ok(())
        }
        None => return Err("Claim not found after asset send".to_string()),
    })
}
