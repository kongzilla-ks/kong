use candid::Nat;

use crate::{
    ic::ckusdt::ckusdt_amount,
    stable_token::stable_token::StableToken,
    stable_user::{reward_distribution::on_made_swap, user_map},
};

pub fn process_swap_rewards(user_id: u32, receive_token: &StableToken, receive_amount: &Nat, ts: u64) -> Vec<u64> {
    if let Some(mut stable_user) = user_map::get_by_user_id(user_id) {
        match ckusdt_amount(receive_token, receive_amount) {
            Ok(volume_notional) => on_made_swap(&mut stable_user, &volume_notional, ts),
            Err(e) => {
                ic_cdk::eprintln!("check ckusdt amount, err={}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    }
}
