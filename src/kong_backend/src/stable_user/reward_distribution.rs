use candid::Nat;

use crate::reward::reward_info::RewardInfo;
use crate::reward::user_reward_progress::update_stats;
use crate::stable_claim::claim_map;
use crate::stable_claim::stable_claim::{ClaimStatus, StableClaim};
use crate::stable_memory::REWARD_INFO_MAP;
use crate::stable_user::stable_user::StableUser;
use crate::stable_user::user_map;

pub fn on_made_swap(user: &mut StableUser, volume_notional: &Nat, ts: u64) -> Vec<u64> {
    update_stats(user, volume_notional);

    let mut new_claims = Vec::new();

    fn add_reward_to_user_if_reached(
        volume_notional: &Nat,
        reward_info: &RewardInfo,
        rewarded_user: &mut StableUser,
        ts: u64,
    ) -> Option<u64> {
        // Dups are possible, that's why we need is_new_reward && add_reward
        if reward_info
            .reward_rules
            .is_new_reward(rewarded_user.get_user_reward_progress(), &volume_notional)
            && rewarded_user.get_user_reward_progress_mut().add_reward(&reward_info.id)
        {
            let claim_id = create_claim(reward_info, rewarded_user.user_id, ts);
            user_map::update(rewarded_user.clone());
            Some(claim_id)
        } else {
            None
        }
    }

    REWARD_INFO_MAP.with(|reward_infos| {
        let reward_infos = reward_infos.borrow();
        for reward_info in reward_infos.values() {
            if !reward_info.is_active {
                continue;
            }

            match reward_info.reward_rules.who_is_rewarded() {
                crate::reward::reward_rules::RewardedUser::User => {
                    if let Some(claim_id) = add_reward_to_user_if_reached(volume_notional, &reward_info, user, ts) {
                        new_claims.push(claim_id);
                    }
                }
                crate::reward::reward_rules::RewardedUser::Referrer => match user.referred_by {
                    Some(referred_by) => {
                        match user_map::get_by_user_id(referred_by) {
                            Some(mut referred_user) => {
                                let _ = add_reward_to_user_if_reached(volume_notional, &reward_info, &mut referred_user, ts);
                            }
                            None => ic_cdk::eprintln!(
                                "user={}, failed to find referred user: referred_user_id={}",
                                user.user_id,
                                referred_by
                            ),
                        };
                    }
                    None => ic_cdk::eprintln!("Unexpected missed referrer for user {}", user.user_id),
                },
            };
        }
    });

    new_claims
}

fn create_claim(reward_info: &RewardInfo, user_id: u32, ts: u64) -> u64 {
    let mut claim = StableClaim::new(user_id, reward_info.reward_token_id.0, &reward_info.reward_amount, None, None, ts);

    claim.status = ClaimStatus::Claimable; // Need to set so user is able to claim it
    claim.desc = Some(format!("Reward {}", reward_info.id.0));
    claim.reward_id = Some(reward_info.id.clone());
    let claim_id = claim_map::insert(&claim);

    return claim_id;
}
