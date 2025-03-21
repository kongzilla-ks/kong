use ic_cdk::{query, update};
use std::collections::BTreeMap;

use crate::ic::get_time::get_time;
use crate::ic::guards::{caller_is_kingkong, caller_is_kong_backend};
use crate::stable_claim::stable_claim::{StableClaim, StableClaimId};
use crate::stable_db_update::db_update_map;
use crate::stable_db_update::stable_db_update::{StableDBUpdate, StableMemory};
use crate::stable_memory::CLAIM_MAP;

const MAX_CLAIMS: usize = 1_000;

#[query(hidden = true, guard = "caller_is_kingkong")]
fn max_claim_idx() -> u64 {
    CLAIM_MAP.with(|m| m.borrow().last_key_value().map_or(0, |(k, _)| k.0))
}

/// serialize CLAIM_MAP for backup
#[query(hidden = true, guard = "caller_is_kingkong")]
fn backup_claims(claim_id: Option<u64>, num_claims: Option<u16>) -> Result<String, String> {
    CLAIM_MAP.with(|m| {
        let map = m.borrow();
        let claims: BTreeMap<_, _> = match claim_id {
            Some(claim_id) => {
                let start_id = StableClaimId(claim_id);
                let num_claims = num_claims.map_or(1, |n| n as usize);
                map.range(start_id..).take(num_claims).collect()
            }
            None => {
                let num_claims = num_claims.map_or(MAX_CLAIMS, |n| n as usize);
                map.iter().take(num_claims).collect()
            }
        };
        serde_json::to_string(&claims).map_err(|e| format!("Failed to serialize claims: {}", e))
    })
}

/// deserialize CLAIM_MAP and update stable memory
#[update(hidden = true, guard = "caller_is_kingkong")]
fn update_claims(stable_claims: String) -> Result<String, String> {
    let claims: BTreeMap<StableClaimId, StableClaim> = match serde_json::from_str(&stable_claims) {
        Ok(claims) => claims,
        Err(e) => return Err(format!("Invalid claims: {}", e)),
    };

    CLAIM_MAP.with(|claim_map| {
        let mut map = claim_map.borrow_mut();
        for (k, v) in claims {
            map.insert(k, v);
        }
    });

    Ok("Claims updated".to_string())
}

#[update(hidden = true, guard = "caller_is_kong_backend")]
fn update_claim(stable_claim_json: String) -> Result<String, String> {
    let claim: StableClaim = match serde_json::from_str(&stable_claim_json) {
        Ok(claim) => claim,
        Err(e) => return Err(format!("Invalid claim: {}", e)),
    };

    CLAIM_MAP.with(|claim_map| {
        let mut map = claim_map.borrow_mut();
        map.insert(StableClaimId(claim.claim_id), claim.clone());
    });

    // add to UpdateMap for archiving to database
    let ts = get_time();
    let update = StableDBUpdate {
        db_update_id: 0,
        stable_memory: StableMemory::ClaimMap(claim),
        ts,
    };
    db_update_map::insert(&update);

    Ok("Claim updated".to_string())
}
