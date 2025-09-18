use candid::Nat;
use kong_lib::{stable_token::token::Token, storable_rational::StorableRational};

use crate::{
    orderbook::{
        book_name::BookName,
        orderbook_path::{Path, TOKEN_PATHS},
    },
    price_observer::price_observer::get_price_path,
    stable_memory_helpers::get_token_by_symbol,
};

const NOTIONAL_ASSET_NAMES: [&str; 4] = ["USDT", "USDC", "ksUSDT", "ksUSDC"];

pub fn usd_notional(token: String, amount: Nat) -> Option<f64> {
    if NOTIONAL_ASSET_NAMES.iter().any(|&t| t == token) {
        let token = match get_token_by_symbol(&token) {
            Some(token) => token,
            None => {
                ic_cdk::eprintln!("Missing {} in tokens", token);
                return None;
            }
        };

        return Some(StorableRational::new_nat(amount).to_f64_decimals(token.decimals(), 0));
    }

    let price = TOKEN_PATHS.with_borrow(|token_paths| {
        for usd_name in NOTIONAL_ASSET_NAMES {
            // token_paths.
            let book_name = BookName::new(usd_name, &token);
            match token_paths.get(&book_name) {
                Some(paths) => match get_price_path_f64(paths, amount.clone()) {
                    Some(v) => return Some(v),
                    None => continue,
                },
                _ => continue,
            }
        }
        return None;
    });
    price
}

fn get_price_path_f64(paths: &Vec<Path>, amount: Nat) -> Option<f64> {
    for path in paths {
        let price = match get_price_path(path) {
            Some(p) => p,
            None => continue,
        };

        let send_token = match get_token_by_symbol(path.send_token()) {
            Some(send_token) => send_token,
            None => {
                ic_cdk::eprintln!("Token exists in paths, but not in tokens, token={}", path.send_token());
                continue;
            }
        };

        let receive_token = match get_token_by_symbol(path.receive_token()) {
            Some(receive_token) => receive_token,
            None => {
                ic_cdk::eprintln!("Token exists in paths, but not in tokens, token={}", path.receive_token());
                continue;
            }
        };

        let amount_f64 = StorableRational::new_nat(amount).to_f64_decimals(send_token.decimals(), 0);
        return Some(amount_f64 * price.0.to_f64_decimals(send_token.decimals(), receive_token.decimals()));
    }

    None
}
