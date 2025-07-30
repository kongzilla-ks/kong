use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::helpers::nat_helpers::{nat_add, nat_divide, nat_multiply, nat_to_decimals_f64, nat_zero};
use crate::ic::ckusdt::{ckusdt_amount, to_ckusdt_decimals_f64};
use crate::stable_lp_token::lp_token_map;
use crate::stable_lp_token::stable_lp_token::StableLPToken;
use crate::stable_token::stable_token::StableToken;
use crate::stable_token::token::Token;
use candid::Nat;
use crate::stable_token::token_map;

use super::lp_reply::LPReply;

#[derive(CandidType, Clone, Debug, Serialize, Deserialize)]
pub enum UserBalancesReply {
    LP(LPReply), // only return LP token balances for now
}

impl TryFrom<(&StableLPToken, u64)> for UserBalancesReply {
    type Error = ();

    fn try_from((lp_token, ts): (&StableLPToken, u64)) -> Result<Self, Self::Error> {
        let lp_token_id = lp_token.lp_token_id;
        let token_id = lp_token.token_id;
        let token = match token_map::get_by_token_id(token_id).ok_or(())? {
            StableToken::LP(lp_token) => lp_token,
            _ => return Err(()),
        };
        if token.is_removed {
            return Err(());
        }
        // get user balance. filter out LP tokens with zero balance
        let user_lp_token_balance = if lp_token.amount == nat_zero() {
            return Err(());
        } else {
            lp_token.amount.clone()
        };
        let lp_token_total_supply = lp_token_map::get_total_supply(token_id);
        // pool of the LP token
        let pool = token.pool_of().ok_or(())?;

        // convert balance to real number
        let balance_nat = user_lp_token_balance.clone();
        let balance = nat_to_decimals_f64(token.decimals, &user_lp_token_balance).ok_or(())?;

        // user_amount_0 = reserve0 * user_lp_token_balance / lp_token_total_supply
        let token_0 = pool.token_0();
        let reserve0 = nat_add(&pool.balance_0, &pool.lp_fee_0);
        let numerator = nat_multiply(&reserve0, &user_lp_token_balance);
        let amount_0_nat = nat_divide(&numerator, &lp_token_total_supply).unwrap_or(nat_zero());
        let amount_0 = nat_to_decimals_f64(token_0.decimals(), &amount_0_nat).ok_or(())?;
        let usd_amount_0_nat = ckusdt_amount(&token_0, &amount_0_nat).unwrap_or(Nat::default());
        let usd_amount_0 = to_ckusdt_decimals_f64(&usd_amount_0_nat).unwrap_or(0_f64);

        // user_amount_1 = reserve1 * user_lp_token_balance / lp_token_total_supply
        let token_1 = pool.token_1();
        let reserve1 = nat_add(&pool.balance_1, &pool.lp_fee_1);
        let numerator = nat_multiply(&reserve1, &user_lp_token_balance);
        let amount_1_nat = nat_divide(&numerator, &lp_token_total_supply).unwrap_or(nat_zero());
        let amount_1 = nat_to_decimals_f64(token_1.decimals(), &amount_1_nat).ok_or(())?;
        let usd_amount_1_nat = ckusdt_amount(&token_1, &amount_1_nat).unwrap_or(Nat::default());
        let usd_amount_1 = ckusdt_amount(&token_1, &amount_1_nat)
            .and_then(|amount_1| to_ckusdt_decimals_f64(&amount_1).ok_or("Error converting amount 1 to ckUSDT".to_string()))
            .unwrap_or(0_f64);

        let usd_balance = usd_amount_0 + usd_amount_1;
        let usd_balance_nat = usd_amount_0_nat.clone() + usd_amount_1_nat.clone();

        Ok(UserBalancesReply::LP(LPReply {
            name: token.name(),
            symbol: token.symbol.clone(),
            pool_id: pool.pool_id,
            lp_token_id,
            balance,
            balance_nat,
            usd_balance,
            usd_balance_nat,
            chain_0: token_0.chain(),
            symbol_0: token_0.symbol(),
            address_0: token_0.address(),
            amount_0,
            amount_0_nat,
            usd_amount_0,
            usd_amount_0_nat,
            chain_1: token_1.chain(),
            symbol_1: token_1.symbol(),
            address_1: token_1.address(),
            amount_1,
            amount_1_nat,
            usd_amount_1,
            usd_amount_1_nat,
            ts,
        }))
    }
}
