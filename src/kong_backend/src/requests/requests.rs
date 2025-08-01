use ic_cdk::query;

use crate::ic::guards::not_in_maintenance_mode;
use crate::stable_request::request_map;

use super::request_reply::RequestsReply;

#[query(guard = "not_in_maintenance_mode")]
async fn requests(request_id: Option<u64>) -> Result<Vec<RequestsReply>, String> {
    let request_id = match request_id {
        Some(request_id) => request_id,
        None => Err("request_id is required".to_string())?,
    };

    let requests = request_map::get_by_request_id(request_id).iter().map(RequestsReply::from).collect();

    Ok(requests)
}
