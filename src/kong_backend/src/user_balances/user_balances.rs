use ic_cdk::query;

use crate::ic::guards::not_in_maintenance_mode;
use crate::ic::network::ICNetwork;
use crate::stable_lp_token::lp_token_map;
use crate::stable_user::user_map;

use super::user_balances_reply::UserBalancesReply;

#[query(guard = "not_in_maintenance_mode")]
pub async fn user_balances(principal_id: String) -> Result<Vec<UserBalancesReply>, String> {
    let user_id = user_map::get_by_principal_id(&principal_id)
        .ok()
        .flatten()
        .ok_or("User not found")?
        .user_id;

    let ts = ICNetwork::get_time();

    Ok(lp_token_map::get_by_user_id(user_id)
        .iter()
        .filter_map(|lp_token| UserBalancesReply::try_from((lp_token, ts)).ok())
        .collect())
}
