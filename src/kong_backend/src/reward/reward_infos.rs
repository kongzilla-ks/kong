use ic_cdk::query;

use crate::ic::guards::not_in_maintenance_mode;
use crate::reward::reward_info::RewardInfo;
use crate::stable_memory::REWARD_INFO_MAP;

#[query(guard = "not_in_maintenance_mode")]
fn reward_infos() -> Vec<RewardInfo> {
    REWARD_INFO_MAP.with(|reward_infos| reward_infos.borrow().values().collect())
}
