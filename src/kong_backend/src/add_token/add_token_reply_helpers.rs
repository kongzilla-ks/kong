use crate::stable_token::stable_token::StableToken;
use crate::stable_token::token::Token;
use crate::tokens::ic_reply::ICReply;

use super::add_token_reply::AddTokenReply;

pub fn to_add_token_reply(token: &StableToken) -> Result<AddTokenReply, String> {
    let token_id = token.token_id();
    match token {
        StableToken::IC(ref ic_token) => Ok(AddTokenReply::IC(ICReply {
            token_id,
            chain: token.chain(),
            name: token.name(),
            symbol: token.symbol(),
            canister_id: token.address(),
            decimals: token.decimals(),
            fee: token.fee(),
            icrc1: ic_token.icrc1,
            icrc2: ic_token.icrc2,
            icrc3: ic_token.icrc3,
            on_kong: token.on_kong(),
        })),
        _ => Err("Unsupported token type".to_string()),
    }
}
