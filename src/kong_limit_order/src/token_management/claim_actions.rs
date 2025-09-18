use ic_cdk::{query, update};

use crate::token_management::{
    claim::Claim,
    claim_map::{process_claim, CLAIMS, USER_CLAIMS},
};

#[query]
pub fn query_active_user_claims() -> Vec<Claim> {
    let user = ic_cdk::api::msg_caller();
    CLAIMS.with_borrow(|claims| {
        USER_CLAIMS.with_borrow(|user_claims| {
            let claim_ids = match user_claims.get(&user) {
                Some(claim_ids) => claim_ids,
                None => return Vec::new(),
            };

            claim_ids.iter().filter_map(|claim_id| match claims.get(&claim_id) {
                Some(claim) => {
                    if claim.status.is_active() {
                        Some(claim.clone())
                    } else {
                        None
                    }
                }
                None => None,
            }).collect()
        })
    })
}

#[query]
pub fn get_claim_by_id(claim_id: u64) -> Option<Claim> {
    CLAIMS.with_borrow(|claims| match claims.get(&claim_id) {
        Some(claim) => {
            let user = ic_cdk::api::msg_caller();
            if claim.user == user {
                Some(claim.clone())
            } else {
                None
            }
        }
        None => None,
    })
}

#[update]
pub async fn apply_claim(claim_id: u64) -> Result<(), String> {
    let user = ic_cdk::api::msg_caller();
    CLAIMS.with_borrow(|claims| match claims.get(&claim_id) {
        Some(claim) if claim.user == user => Ok(()),
        _ => Err("Claim id not found".to_string()),
    })?;

    process_claim(claim_id).await
}
