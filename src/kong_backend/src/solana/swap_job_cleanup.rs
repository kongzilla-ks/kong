//! Cleanup task for expired Solana swap jobs
//! 
//! This module handles the expiration of swap jobs that have been pending
//! for too long without confirmation from kong_rpc.

use crate::ic::network::ICNetwork;
use crate::solana::swap_job::SwapJobStatus;
use crate::stable_claim::{claim_map, stable_claim::StableClaim};
use crate::stable_memory::{with_swap_job_queue_mut, CLAIM_MAP};
use crate::stable_request::{request_map, reply::Reply, request::Request};
use crate::stable_token::{token_map, token::Token};
use crate::ic::address::Address;

/// Timeout for swap jobs in nanoseconds (300 seconds = 5 minutes)
const SWAP_JOB_TIMEOUT_NS: u64 = 300_000_000_000;

/// Clean up expired swap jobs that have been pending for too long.
/// 
/// This function:
/// 1. Finds all swap jobs in Pending status older than SWAP_JOB_TIMEOUT_NS
/// 2. Marks them as Failed with an appropriate error message
/// 3. Creates claims for fund recovery
/// 
/// This ensures that if kong_rpc fails to report back (network issues, crashes, etc.),
/// users can still recover their funds through the claim system.
pub fn cleanup_expired_swap_jobs() {
    let current_time = ICNetwork::get_time();
    let cutoff_time = current_time.saturating_sub(SWAP_JOB_TIMEOUT_NS);
    
    with_swap_job_queue_mut(|queue| {
        let mut expired_count = 0;
        
        // Collect jobs to update (avoid borrowing issues)
        let jobs_to_update: Vec<_> = queue
            .iter()
            .filter(|(_, job)| {
                job.status == SwapJobStatus::Pending && job.created_at < cutoff_time
            })
            .map(|(id, job)| (id, job.clone()))
            .collect();
        
        for (job_id, mut job) in jobs_to_update {
            // Check if a claim already exists for this request to prevent duplicates
            let existing_claims: Vec<_> = CLAIM_MAP.with(|m| {
                m.borrow()
                    .iter()
                    .filter(|(_, claim)| claim.request_id == Some(job.request_id))
                    .collect::<Vec<_>>()
            });
            
            if !existing_claims.is_empty() {
                ICNetwork::info_log(&format!(
                    "[CLEANUP] Job #{} - Claim already exists for request {}, skipping",
                    job.id, job.request_id
                ));
                // Still mark as failed but don't create another claim
                job.status = SwapJobStatus::Failed;
                job.error_message = Some("Transaction expired (>5 minutes) - claim already exists".to_string());
                job.updated_at = current_time;
                queue.insert(job_id, job);
                continue;
            }
            
            // Get the original request to create a claim
            match request_map::get_by_request_id(job.request_id) {
                Some(request) => {
                    match &request.request {
                        Request::Swap(swap_args) => {
                            let destination_address = match &swap_args.receive_address {
                                Some(addr) => addr.clone(),
                                None => {
                                    ICNetwork::error_log(&format!(
                                        "[CLEANUP] Job #{} - No receive_address in swap args",
                                        job.id
                                    ));
                                    continue;
                                }
                            };
                            
                            match request.reply {
                                Reply::Swap(swap_reply) => {
                                    match token_map::get_by_token(&swap_reply.receive_symbol) {
                                        Ok(receive_token) => {
                                            ICNetwork::info_log(&format!(
                                                "[CLEANUP] Job #{} expired after {} seconds - creating claim for user {}",
                                                job.id,
                                                (current_time - job.created_at) / 1_000_000_000,
                                                job.user_id
                                            ));
                                            
                                            // Create claim for fund recovery
                                            let claim = StableClaim::new(
                                                job.user_id,
                                                receive_token.token_id(),
                                                &swap_reply.receive_amount,
                                                Some(job.request_id),
                                                Some(Address::SolanaAddress(destination_address)),
                                                current_time,
                                            );
                                            
                                            let claim_id = claim_map::insert(&claim);
                                            ICNetwork::info_log(&format!(
                                                "[CLEANUP] Created claim #{} for expired job #{}",
                                                claim_id, job.id
                                            ));
                                            
                                            expired_count += 1;
                                        }
                                        Err(e) => {
                                            ICNetwork::error_log(&format!(
                                                "[CLEANUP] Job #{} - Failed to get receive token: {}",
                                                job.id, e
                                            ));
                                        }
                                    }
                                }
                                _ => {
                                    ICNetwork::error_log(&format!(
                                        "[CLEANUP] Job #{} - Request {} is not a swap",
                                        job.id, job.request_id
                                    ));
                                }
                            }
                        }
                        _ => {
                            ICNetwork::error_log(&format!(
                                "[CLEANUP] Job #{} - Request {} is not a swap",
                                job.id, job.request_id
                            ));
                        }
                    }
                }
                None => {
                    ICNetwork::error_log(&format!(
                        "[CLEANUP] Job #{} - Request {} not found",
                        job.id, job.request_id
                    ));
                }
            }
            
            // Mark job as failed
            job.status = SwapJobStatus::Failed;
            job.error_message = Some(format!(
                "Transaction expired after {} seconds without confirmation from kong_rpc",
                (current_time - job.created_at) / 1_000_000_000
            ));
            job.updated_at = current_time;
            queue.insert(job_id, job);
        }
        
        if expired_count > 0 {
            ICNetwork::info_log(&format!(
                "[CLEANUP] Expired {} swap job(s) and created claims for fund recovery",
                expired_count
            ));
        }
    });
}