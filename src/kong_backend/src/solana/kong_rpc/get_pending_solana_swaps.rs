use ic_cdk::query;
use std::ops::Bound::{Excluded, Unbounded};

use crate::ic::guards::caller_is_kong_rpc;
use crate::stable_memory::with_swap_job_queue;
use crate::swap::swap_job::{SwapJob, SwapJobStatus};

/// Get pending Solana swap jobs for kong_rpc processing (called by kong_rpc)
#[query(hidden = true, guard = "caller_is_kong_rpc")]
pub fn get_pending_solana_swaps(from_job_id: Option<u64>) -> Result<Vec<SwapJob>, String> {
    const MAX_BATCH_SIZE: usize = 100;

    with_swap_job_queue(|queue| {
        Ok(queue
            .range((from_job_id.map_or(Unbounded, Excluded), Unbounded))
            .filter_map(|(_, job)| matches!(job.status, SwapJobStatus::Pending).then(|| job.clone()))
            .take(MAX_BATCH_SIZE)
            .collect())
    })
}
