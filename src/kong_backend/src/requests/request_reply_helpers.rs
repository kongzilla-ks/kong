use super::request_reply::RequestReply;

use crate::stable_request::stable_request::StableRequest;

// creates a RequestReply from a StableRequest
pub fn to_request_reply(request: &StableRequest) -> RequestReply {
    RequestReply::from(request)
}
