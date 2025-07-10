use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub enum RewardRules {
    UserNotionalVolume(RewardRuleVolumeNotional),
    ReferredNotionalVolume(RewardRuleVolumeNotional),
    UserSwapCount(RewardRuleSwapCount),
    ConsecutiveDays(RewardRuleConsecutiveDays),
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct RewardRuleVolumeNotional {
    pub desired_volume: Nat,
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct RewardRuleSwapCount {
    pub swap_count: u32,
}

#[derive(CandidType, Debug, Clone, Serialize, Deserialize)]
pub struct RewardRuleConsecutiveDays {
    pub consecutive_days: u32,
}

pub enum RewardedUser
{
    User,
    Referrer,
}

impl RewardRules {
    pub fn who_is_rewarded(&self) -> RewardedUser {
        match &self {
            RewardRules::UserNotionalVolume(_) => RewardedUser::User,
            RewardRules::ReferredNotionalVolume(_) => RewardedUser::Referrer,
            RewardRules::UserSwapCount(_) => RewardedUser::User,
            RewardRules::ConsecutiveDays(_) => RewardedUser::User,
        }
    }
}
