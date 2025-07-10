use crate::ic::kong::get_kong_id;
use crate::ic::network::ICNetwork;
use crate::reward::reward_rules::RewardRules;
use crate::{
    reward::reward_info::{RewardInfo, RewardInfoId},
    stable_token::stable_token::StableTokenId,
};
use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct RewardInfoArgs {
    pub reward_volume: Nat,
    pub reward_asset_id: Option<StableTokenId>,

    pub is_active: bool,

    pub reward_rules: RewardRules,
}

pub fn reward_info_from_args(id: RewardInfoId, args: RewardInfoArgs) -> RewardInfo {
    let ts = ICNetwork::get_time();
    let caller_principal_id = ICNetwork::caller().to_text();
    RewardInfo {
        id: id,
        reward_amount: args.reward_volume,
        reward_token_id: args.reward_asset_id.unwrap_or(StableTokenId(get_kong_id())),
        created_ts: ts,
        updated_ts: ts,
        updated_user: caller_principal_id,
        is_active: args.is_active,
        reward_rules: args.reward_rules,
    }
}
