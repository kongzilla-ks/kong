use ic_cdk::update;

use crate::ic::guards::caller_is_kong_rpc;
use crate::ic::network::ICNetwork;
use crate::stable_memory::with_swap_job_queue_mut;
use crate::solana::swap_job::{SwapJobId, SwapJobStatus};
use crate::solana::kong_rpc::transaction_notification::{TransactionNotification, TransactionNotificationId, TransactionNotificationStatus};
use crate::stable_memory::with_solana_tx_notifications_mut;

/// Update a Solana swap job status (called by kong_rpc after transaction execution)
#[update(hidden = true, guard = "caller_is_kong_rpc")]
pub fn update_solana_swap(job_id: u64, final_solana_tx_sig: String, was_successful: bool, error_msg: Option<String>) -> Result<(), String> {
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
            let target_status = if was_successful {
                SwapJobStatus::Confirmed
            } else {
                SwapJobStatus::Failed
            };

            match job.status {
                SwapJobStatus::PendingVerification => {
                    // Jobs still in verification shouldn't be finalized
                    Err(format!("Job {} is still in payment verification, cannot finalize yet", job_id))
                }
                SwapJobStatus::Pending => {
                    // Normal transition: Pending -> Confirmed/Failed
                    job.status = target_status;
                    job.solana_tx_signature_of_payout = Some(final_solana_tx_sig);
                    job.error_message = error_msg;
                    job.updated_at = ICNetwork::get_time();
                    queue.insert(SwapJobId(job_id), job);
                    
                    // Remove successfully completed jobs to prevent reprocessing
                    if was_successful {
                        queue.remove(&SwapJobId(job_id));
                    }
                    Ok(())
                }
                SwapJobStatus::Confirmed => {
                    if was_successful {
                        // Already confirmed - idempotent if same signature
                        match &job.solana_tx_signature_of_payout {
                            Some(existing_sig) if existing_sig == &final_solana_tx_sig => {
                                // Remove completed jobs to prevent reprocessing
                                queue.remove(&SwapJobId(job_id));
                                Ok(())
                            },
                            _ => Err(format!("Job {} already confirmed with different signature", job_id)),
                        }
                    } else {
                        // Can't fail a confirmed job
                        Err(format!("Job {} is already confirmed, cannot mark as failed", job_id))
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
