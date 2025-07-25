use crate::ic::{guards::not_in_maintenance_mode, network::ICNetwork};
use crate::stable_claim::claim_map;
use crate::stable_claim::stable_claim::ClaimStatus;
use crate::stable_memory::CLAIM_MAP;
use crate::stable_request::{request::Request, request_map, stable_request::StableRequest, status::StatusCode};
use crate::stable_token::token::Token;
use crate::stable_token::token_map;
use crate::stable_user::stable_user::CLAIMS_TIMER_USER_ID;

use super::archive_to_kong_data::archive_to_kong_data;
use super::process_claim::process_claim;

/// Send out claims where status is Unclaimed or UnclaimedOverride
pub async fn process_claims_timer() {
    if not_in_maintenance_mode().is_err() {
        return;
    }

    let ts = ICNetwork::get_time();

    // get snapshot of claim_ids where status is Unclaimed or UnclaimedOverride
    let claim_ids = CLAIM_MAP.with(|m| {
        m.borrow()
            .iter()
            .rev()
            .filter_map(|(_, v)| {
                if v.status == ClaimStatus::Unclaimed || v.status == ClaimStatus::UnclaimedOverride {
                    Some(v.claim_id)
                } else {
                    None
                }
            })
            .collect::<Vec<u64>>()
    });

    let mut consecutive_errors = 0_u8;
    for claim_id in claim_ids {
        let claim = match claim_map::get_by_claim_id(claim_id) {
            Some(claim) => claim,
            None => continue, // continue to next claim if claim not found
        };
        if claim.status != ClaimStatus::Unclaimed && claim.status != ClaimStatus::UnclaimedOverride {
            // continue to next claim if claim status is not Unclaimed or UnclaimedOverride.
            // This must have changed from the time we got the claim_ids above. This can happen as process_claim() makes inter-canister calls and other states can change
            continue;
        }
        let token = match token_map::get_by_token_id(claim.token_id) {
            Some(token) => token,
            None => continue, // continue to next claim if token not found
        };
        let to_address = match &claim.to_address {
            Some(address) => address.clone(),
            None => continue, // continue to next claim if to_address is not found
        };
        if (claim.attempt_request_id.len() > 50 || (claim.attempt_request_id.len() > 10 && token.is_removed()))
            && claim.status != ClaimStatus::UnclaimedOverride
        {
            // if claim has more than 50 attempts, update status to too_many_attempts and investigate manually
            claim_map::update_too_many_attempts_status(claim.claim_id);
            let _ = claim_map::archive_to_kong_data(claim.claim_id);
            continue;
        }
        if claim.attempt_request_id.len() > 20 {
            // after 20 attempts, only try to claim if last attempt was more than 1 hour ago
            if let Some(last_attempt_request_id) = claim.attempt_request_id.last() {
                if let Some(request) = request_map::get_by_request_id(*last_attempt_request_id) {
                    if request.ts + 3_600_000_000_000 > ts {
                        continue;
                    }
                }
            }
        }

        // register new request for this claim with CLAIMS_TIMER_USER_ID as user_id
        let request_id = request_map::insert(&StableRequest::new(CLAIMS_TIMER_USER_ID, &Request::Claim(claim.claim_id), ts));
        match process_claim(request_id, &claim, &token, &claim.amount, &to_address, ts).await {
            Ok(_) => {
                request_map::update_status(request_id, StatusCode::Success, None);
                consecutive_errors = 0;
            }
            Err(_) => {
                request_map::update_status(request_id, StatusCode::Failed, None);
                consecutive_errors += 1;
            }
        }
        let _ = archive_to_kong_data(request_id);

        if consecutive_errors > 4 {
            ICNetwork::error_log("Too many consecutive errors, stopping claims process");
            break;
        }
    }
}
