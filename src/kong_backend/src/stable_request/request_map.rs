use crate::ic::network::ICNetwork;
use crate::stable_kong_settings::kong_settings_map;
use crate::stable_memory::REQUEST_MAP;

use super::reply::Reply;
use super::stable_request::{StableRequest, StableRequestId};
use super::status::{Status, StatusCode};

pub fn get_by_request_id(request_id: u64) -> Option<StableRequest> {
    REQUEST_MAP.with(|m| m.borrow().get(&StableRequestId(request_id)))
}

pub fn insert(request: &StableRequest) -> u64 {
    REQUEST_MAP.with(|m| {
        let mut map = m.borrow_mut();
        let request_id = kong_settings_map::inc_request_map_idx();
        let insert_request = StableRequest {
            request_id,
            ..request.clone()
        };
        map.insert(StableRequestId(request_id), insert_request);
        request_id
    })
}

pub fn update_status(key: u64, status_code: StatusCode, message: Option<&str>) -> Option<StableRequest> {
    REQUEST_MAP.with(|m| {
        let mut map = m.borrow_mut();
        let key = StableRequestId(key);
        match map.get(&key) {
            Some(mut v) => {
                v.statuses.push(Status {
                    status_code,
                    message: message.map(|s| s.to_string()),
                });
                map.insert(key, v)
            }
            None => None,
        }
    })
}

pub fn update_reply(key: u64, reply: Reply) -> Option<StableRequest> {
    REQUEST_MAP.with(|m| {
        let mut map = m.borrow_mut();
        let key = StableRequestId(key);
        match map.get(&key) {
            Some(mut v) => {
                v.reply = reply;
                map.insert(key, v)
            }
            None => None,
        }
    })
}

pub fn archive_to_kong_data(request: &StableRequest) -> Result<(), String> {
    if !kong_settings_map::get().archive_to_kong_data {
        return Ok(());
    }

    let request_id = request.request_id;
    let request_json = match serde_json::to_string(request) {
        Ok(request_json) => request_json,
        Err(e) => Err(format!("Failed to archive request_id #{}. {}", request_id, e))?,
    };

            ic_cdk::futures::spawn(async move {
        let kong_data = kong_settings_map::get().kong_data;
        match ic_cdk::call::Call::unbounded_wait(kong_data, "update_request")
            .with_arg((request_json,))
            .await
            .map_err(|e| format!("{:?}", e))
            .and_then(|response| response.candid::<(Result<String, String>,)>()
                .map_err(|e| format!("{:?}", e)))
            .unwrap_or_else(|e| (Err(e),))
            .0
        {
            Ok(_) => (),
            Err(e) => ICNetwork::error_log(&format!("Failed to archive request_id #{}. {}", request_id, e)),
        }
    });

    Ok(())
}
