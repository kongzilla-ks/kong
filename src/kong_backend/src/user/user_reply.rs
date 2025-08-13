use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::convert::From;

use crate::ic::network::ICNetwork;
use crate::stable_user::stable_user::StableUser;
use crate::stable_user::user_map;

use crate::reward::user_reward_progress::UserRewardProgress;

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub struct UserReply {
    pub user_id: u32,
    pub principal_id: String,
    pub account_id: String,
    pub my_referral_code: String,
    pub referred_by: Option<String>,
    pub referred_by_expires_at: Option<u64>,
    pub fee_level: u8,
    pub fee_level_expires_at: Option<u64>,
    pub user_reward_progress: UserRewardProgress,
}

impl From<&StableUser> for UserReply {
    fn from(user: &StableUser) -> Self {
        let principal = Principal::from_text(&user.principal_id).unwrap();
        let account_id = ICNetwork::principal_to_account_id(principal);
        // if referred by user exists, get the referred user's referral code
        let referred_by = user
            .referred_by
            .and_then(|referred_user| user_map::get_by_user_id(referred_user)
                .map(|referred_user| referred_user.my_referral_code));
        
        UserReply {
            user_id: user.user_id,
            principal_id: user.principal_id.clone(),
            account_id: account_id.to_string(),
            my_referral_code: user.my_referral_code.clone(),
            referred_by,
            referred_by_expires_at: user.referred_by_expires_at,
            fee_level: user.fee_level,
            fee_level_expires_at: user.fee_level_expires_at,
            user_reward_progress: user.get_user_reward_progress().clone(),
        }
    }
}
