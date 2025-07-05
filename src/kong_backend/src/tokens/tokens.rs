use ic_cdk::query;

use crate::ic::guards::not_in_maintenance_mode;
use crate::stable_token::token_map;

use super::tokens_reply::TokensReply;

#[query(guard = "not_in_maintenance_mode")]
fn tokens(symbol: Option<String>) -> Result<Vec<TokensReply>, String> {
    Ok(match symbol {
        Some(symbol) => token_map::get_by_token_wildcard(&symbol),
        None => token_map::get(),
    }
    .iter()
    .map(TokensReply::from)
    .collect())
}
