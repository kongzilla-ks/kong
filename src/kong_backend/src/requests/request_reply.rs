use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::stable_request::reply::Reply;
use crate::stable_request::request::Request;
use crate::stable_request::stable_request::StableRequest;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct RequestsReply {
    pub request_id: u64,
    pub statuses: Vec<String>,
    pub request: Request,
    pub reply: Reply,
    pub ts: u64,
}

impl From<&StableRequest> for RequestsReply {
    fn from(request: &StableRequest) -> Self {
        RequestsReply {
            request_id: request.request_id,
            statuses: request.statuses.iter().map(|status| status.to_string()).collect(),
            request: request.request.clone(),
            reply: request.reply.clone(),
            ts: request.ts,
        }
    }
}
