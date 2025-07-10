use crate::ic::network::ICNetwork;
use crate::reward::{reward_info::RewardInfoId, reward_rules::RewardRules};
use crate::stable_user::stable_user::StableUser;
use crate::stable_user::user_map;
use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserRewardProgress {
    received_rewards: Vec<RewardInfoId>,

    consecutive_trading_days: u32,
    total_swap_count: u32,
    total_user_volume: Nat,
    total_referred_volume: Nat,

    last_trading_day: u64,
}

impl UserRewardProgress {
    pub fn add_reward(&mut self, id: &RewardInfoId) -> bool {
        match self.received_rewards.binary_search(id) {
            Ok(_) => false,
            Err(pos) => {
                self.received_rewards.insert(pos, id.clone());
                true
            }
        }
    }
}


fn time_to_day(day_ts: u64) -> u64 {
    // let day_ts = ICNetwork::get_time();
    let nanos_in_second = 1_000_000_000u64;
    let nanos_in_day = nanos_in_second * 60 * 60 * 24;
    day_ts / nanos_in_day
}

fn get_current_trading_day() -> u64 {
    time_to_day(ICNetwork::get_time())
}

pub fn update_stats(user: &mut StableUser, volume_notional: &Nat) {
    let stats = &mut user.user_reward_progress;
    // Update consecutive trading days
    let current_day = get_current_trading_day();
    if stats.last_trading_day == 0 {
        stats.last_trading_day = current_day;
        stats.consecutive_trading_days = 1;
    } else {
        let day_diff = current_day - stats.last_trading_day;
        if day_diff == 1 {
            stats.last_trading_day = current_day;
            stats.consecutive_trading_days = stats.consecutive_trading_days + 1;
        } else if day_diff > 1 {
            stats.last_trading_day = current_day;
            stats.consecutive_trading_days = 1;
        }
        // do nothing if day_diff == 0
    }

    // Update total swap count
    stats.total_swap_count += 1;

    // Update total volume notional
    stats.total_user_volume += volume_notional.clone();

    if let Some(referred_by) = user.referred_by {
        match user_map::get_by_user_id(referred_by) {
            Some(mut referred_user) => {
                referred_user.user_reward_progress.total_referred_volume += volume_notional.clone();
                user_map::update(referred_user);
            },
            None => ic_cdk::eprintln!("Failed to find referred user in user map, user={}, referred_user={}", user.user_id, referred_by),
        }
    }
    
    user_map::update(user.clone());
}

impl RewardRules {
    pub fn is_new_reward(&self, progress: &UserRewardProgress, trade_notional_volume: &Nat) -> bool {
        fn passed_desired_volume(current_volume: &Nat, trade_notional_volume: &Nat, desired_volume: &Nat) -> bool {
            current_volume >= desired_volume && &(current_volume.clone() - trade_notional_volume.clone()) < desired_volume
        }
        match self {
            Self::UserNotionalVolume(v) => {
                passed_desired_volume(&progress.total_user_volume, &trade_notional_volume, &v.desired_volume)
            },
            Self::ReferredNotionalVolume(v) => {
                passed_desired_volume(&progress.total_referred_volume, &trade_notional_volume, &v.desired_volume)
            },
            Self::UserSwapCount(v) => v.swap_count == progress.total_swap_count,
            Self::ConsecutiveDays(v) => v.consecutive_days == progress.consecutive_trading_days,
        }
    }
}
