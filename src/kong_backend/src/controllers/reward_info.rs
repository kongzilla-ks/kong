use ic_cdk::update;

use crate::ic::guards::caller_is_kingkong;
use crate::reward::reward_info::{RewardInfo, RewardInfoId};
use crate::reward::reward_info_args::{reward_info_from_args, RewardInfoArgs};
use crate::stable_kong_settings::kong_settings_map::inc_reward_info_map_idx;
use crate::stable_memory::REWARD_INFO_MAP;

fn serialize_reward_info(reward_info: &RewardInfo) -> Result<String, String> {
    serde_json::to_string(&reward_info).map_err(|e| format!("Failed to serialize reward info: {}", e))
}

#[update(hidden = true, guard = "caller_is_kingkong")]
fn add_new_reward_info(args: RewardInfoArgs) -> Result<String, String> {
    let next_id = RewardInfoId(inc_reward_info_map_idx());
    let reward_info = reward_info_from_args(next_id.clone(), args);

    REWARD_INFO_MAP.with(|reward_infos| {
        if reward_infos.borrow().contains_key(&next_id) {
            return Err(format!("Invalid next id={}", next_id.0));
        }
        reward_infos.borrow_mut().insert(next_id, reward_info.clone());
        serialize_reward_info(&reward_info)
    })
}

#[update(hidden = true, guard = "caller_is_kingkong")]
fn remove_reward_info(id: RewardInfoId) -> Result<String, String> {
    REWARD_INFO_MAP.with(|reward_infos| match reward_infos.borrow_mut().remove(&id) {
        Some(v) => serialize_reward_info(&v),
        None => Err(format!("Reward info with id={} not found", id.0)),
    })
}

#[update(hidden = true, guard = "caller_is_kingkong")]
fn reward_info_set_is_active(reward_info_id: RewardInfoId, is_active: bool) -> Result<(), String> {
    REWARD_INFO_MAP.with(|reward_infos| {
        let mut reward_infos = reward_infos.borrow_mut();
        let mut reward_info = match reward_infos.get(&reward_info_id) {
            Some(reward_info) => reward_info,
            None => return Err(format!("Reward info with id={} does not exist", reward_info_id.0)),
        };

        reward_info.is_active = is_active;

        reward_infos.insert(reward_info_id, reward_info);

        Ok(())
    })
}

// TODO: add generic json edit similar to set_kong_settings?
