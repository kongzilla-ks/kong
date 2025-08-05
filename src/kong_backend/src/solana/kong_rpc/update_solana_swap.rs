use ic_cdk::update;

use crate::ic::address::Address;
use crate::ic::guards::caller_is_kong_rpc;
use crate::ic::network::ICNetwork;
use crate::solana::kong_rpc::transaction_notification::{
    TransactionNotification, TransactionNotificationId, TransactionNotificationStatus,
};
use crate::solana::swap_job::{SwapJobId, SwapJobStatus};
use crate::stable_claim::{claim_map, stable_claim::StableClaim};
use crate::stable_memory::{with_solana_tx_notifications_mut, with_swap_job_queue_mut, CLAIM_MAP};
use crate::stable_request::{request_map, reply::Reply, request::Request};
use crate::stable_token::{token_map, token::Token};

/// Update a Solana swap job status (called by kong_rpc after transaction execution)
#[update(hidden = true, guard = "caller_is_kong_rpc")]
pub fn update_solana_swap(
    job_id: u64,
    final_solana_tx_sig: String,
    was_successful: bool,
    error_msg: Option<String>,
) -> Result<(), String> {
    // Add or update transaction notification
    with_solana_tx_notifications_mut(|notifications| {
        let notification_id = TransactionNotificationId(final_solana_tx_sig.clone());
        let mut notification = notifications.get(&notification_id).unwrap_or_else(|| {
            // Create a new notification if it doesn't exist (e.g., if submission failed and was_successful is false)
            TransactionNotification {
                status: TransactionNotificationStatus::Processed,
                metadata: None,
                timestamp: ICNetwork::get_time(),
                tx_signature: final_solana_tx_sig.clone(),
                job_id,
                is_completed: false, // Default to false
            }
        });
        notification.is_completed = was_successful; // Set is_completed based on success status
        notifications.insert(notification_id, notification);
    });

    // Update the swap job status in the main map
    with_swap_job_queue_mut(|queue| {
        if let Some(mut job) = queue.get(&SwapJobId(job_id)) {
            match job.status {
                SwapJobStatus::PendingVerification => {
                    // Jobs still in verification shouldn't be finalized
                    Err(format!(
                        "Job {} is still in payment verification, cannot finalize yet",
                        job_id
                    ))
                }
                SwapJobStatus::Pending => {
                    if was_successful {
                        // Normal transition: Pending -> Confirmed
                        job.status = SwapJobStatus::Confirmed;
                        job.solana_tx_signature_of_payout = Some(final_solana_tx_sig);
                        job.error_message = None;
                        job.updated_at = ICNetwork::get_time();
                        queue.insert(SwapJobId(job_id), job);
                        
                        // Remove successfully completed jobs to prevent reprocessing
                        queue.remove(&SwapJobId(job_id));
                        Ok(())
                    } else {
                        // Transition: Pending -> Failed
                        // Create a claim for failed Solana swaps so user can recover funds
                        
                        // First check if claim already exists to prevent double-spend
                        let existing_claims: Vec<_> = CLAIM_MAP.with(|m| {
                            m.borrow()
                                .iter()
                                .filter(|(_, claim)| claim.request_id == Some(job.request_id))
                                .collect::<Vec<_>>()
                        });
                        
                        if !existing_claims.is_empty() {
                            ICNetwork::info_log(&format!(
                                "Claim already exists for request {}, skipping claim creation",
                                job.request_id
                            ));
                            job.status = SwapJobStatus::Failed;
                            job.solana_tx_signature_of_payout = Some(final_solana_tx_sig);
                            job.error_message = error_msg;
                            job.updated_at = ICNetwork::get_time();
                            queue.insert(SwapJobId(job_id), job);
                            return Ok(());
                        }
                        
                        // Get the original request to extract swap details
                        match request_map::get_by_request_id(job.request_id) {
                            Some(request) => {
                                // We need to check the actual request, not just the reply
                                match &request.request {
                                    Request::Swap(swap_args) => {
                                        // Get the actual destination address from swap args
                                        let destination_address = match &swap_args.receive_address {
                                            Some(addr) => addr.clone(),
                                            None => {
                                                ICNetwork::error_log(&format!(
                                                    "No receive_address in swap args for job #{}", job.id
                                                ));
                                                return Ok(());
                                            }
                                        };
                                        
                                        match request.reply {
                                            Reply::Swap(swap_reply) => {
                                                // Get the receive token symbol for lookup
                                                match token_map::get_by_token(&swap_reply.receive_symbol) {
                                                    Ok(receive_token) => {
                                                        // Only create claims for retryable failures
                                                        let should_create_claim = match &error_msg {
                                                            Some(msg) => {
                                                                // Network/timeout failures - retryable
                                                                msg.contains("timeout") || 
                                                                msg.contains("Timeout") ||
                                                                msg.contains("RPC") || 
                                                                msg.contains("network") ||
                                                                msg.contains("Network") ||
                                                                msg.contains("connection") ||
                                                                msg.contains("Connection") ||
                                                                msg.contains("blockhash") ||
                                                                // WebSocket failures - retryable
                                                                msg.contains("WebSocket") ||
                                                                msg.contains("websocket") ||
                                                                // Processing crashes - retryable
                                                                msg.contains("crashed") ||
                                                                msg.contains("panic") ||
                                                                // General processing errors - retryable
                                                                msg.contains("Processing error") ||
                                                                // Submission errors - retryable
                                                                msg.contains("submission error")
                                                            }
                                                            None => true // If no error message, assume retryable
                                                        };
                                                        
                                                        if should_create_claim {
                                                            ICNetwork::info_log(&format!(
                                                                "Creating claim for failed swap job #{} (user: {}, request: {}, dest: {})",
                                                                job.id, job.user_id, job.request_id, destination_address
                                                            ));
                                                            
                                                            // Create the claim with the ACTUAL destination address
                                                            let claim = StableClaim::new(
                                                                job.user_id,
                                                                receive_token.token_id(),
                                                                &swap_reply.receive_amount,
                                                                Some(job.request_id),
                                                                Some(Address::SolanaAddress(destination_address)),
                                                                ICNetwork::get_time(),
                                                            );
                                                            
                                                            let claim_id = claim_map::insert(&claim);
                                                            ICNetwork::info_log(&format!(
                                                                "Created claim #{} for failed swap job #{}",
                                                                claim_id, job.id
                                                            ));
                                                        }
                                                    }
                                                    Err(e) => {
                                                        ICNetwork::error_log(&format!(
                                                            "Failed to get receive token for swap job #{}: {}",
                                                            job.id, e
                                                        ));
                                                    }
                                                }
                                            }
                                            _ => {
                                                ICNetwork::error_log(&format!(
                                                    "Request {} reply is not a swap reply",
                                                    job.request_id
                                                ));
                                            }
                                        }
                                    }
                                    _ => {
                                        ICNetwork::error_log(&format!(
                                            "Request {} is not a swap request, cannot create claim",
                                            job.request_id
                                        ));
                                    }
                                }
                            }
                            None => {
                                ICNetwork::error_log(&format!(
                                    "Request {} not found for swap job #{}",
                                    job.request_id, job.id
                                ));
                            }
                        }
                        
                        // Update job status regardless of claim creation
                        job.status = SwapJobStatus::Failed;
                        job.solana_tx_signature_of_payout = Some(final_solana_tx_sig);
                        job.error_message = error_msg;
                        job.updated_at = ICNetwork::get_time();
                        queue.insert(SwapJobId(job_id), job);
                        Ok(())
                    }
                }
                SwapJobStatus::Confirmed => {
                    if was_successful {
                        // Already confirmed - idempotent if same signature
                        match &job.solana_tx_signature_of_payout {
                            Some(existing_sig) if existing_sig == &final_solana_tx_sig => {
                                // Remove completed jobs to prevent reprocessing
                                queue.remove(&SwapJobId(job_id));
                                Ok(())
                            }
                            _ => Err(format!(
                                "Job {} already confirmed with different signature",
                                job_id
                            )),
                        }
                    } else {
                        // Can't fail a confirmed job
                        Err(format!(
                            "Job {} is already confirmed, cannot mark as failed",
                            job_id
                        ))
                    }
                }
                SwapJobStatus::Failed => {
                    if was_successful {
                        // Rare case: job previously failed but now succeeded (retry worked)
                        job.status = SwapJobStatus::Confirmed;
                        job.solana_tx_signature_of_payout = Some(final_solana_tx_sig);
                        job.error_message = None;
                        job.updated_at = ICNetwork::get_time();
                        queue.insert(SwapJobId(job_id), job);

                        // Remove successfully completed jobs to prevent reprocessing
                        queue.remove(&SwapJobId(job_id));
                        Ok(())
                    } else {
                        // Already failed - update error message if different
                        if job.error_message != error_msg {
                            job.error_message = error_msg;
                            job.updated_at = ICNetwork::get_time();
                            queue.insert(SwapJobId(job_id), job);
                        }
                        Ok(())
                    }
                }
            }
        } else {
            Err(format!("Job {} not found", job_id))
        }
    })
}

