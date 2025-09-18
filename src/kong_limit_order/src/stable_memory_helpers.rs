use std::{cell::RefCell, rc::Rc};

use crate::{
    limit_order_settings::LimitOrderSettings,
    orderbook::{
        book_name::BookName,
        order_history::ORDER_HISTORY,
        orderbook::ORDERBOOKS,
        orderbook_path_helper::{add_to_synth_path, add_token_pair},
    },
    price_observer::price_observer::PRICE_OBSERVER,
    stable_memory::{
        STABLE_AVAILABLE_TOKEN_POOLS, STABLE_CLAIMS, STABLE_LIMIT_ORDER_SETTINGS, STABLE_ORDERBOOKS, STABLE_ORDER_HISTORY,
        STABLE_PRICE_OBSERVER, STABLE_TWAP_EXECUTOR, TOKEN_MAP,
    },
    twap::twap_executor::TWAP_EXECUTOR,
};
use ic_cdk::{post_upgrade, pre_upgrade};
use kong_lib::{
    stable_token::{stable_token::StableToken, token::Token},
    storable_vec::StorableVec,
};

fn validate_token(token: &String) -> Result<(), String> {
    if token.is_empty() {
        return Err("Token is empty".to_string());
    }

    if !token.chars().all(|c| c.is_alphanumeric()) {
        return Err("Only ascii alphabeic is supported".to_string());
    }

    Ok(())
}

pub fn sort_token_pair<'a>(receive_token: &'a str, send_token: &'a str) -> (&'a str, &'a str) {
    if receive_token < send_token {
        (receive_token, send_token)
    } else {
        (send_token, receive_token)
    }
}

// Available token pair for direct/reverse swap
pub fn is_available_token_pair(token_0: &str, token_1: &str) -> bool {
    let (token_0, token_1) = sort_token_pair(token_0, token_1);

    let book_name = BookName::new(&token_0, &token_1);
    STABLE_AVAILABLE_TOKEN_POOLS.with_borrow(|m| m.contains(&book_name))
}

pub fn add_available_token_pair_impl(symbol_0: String, symbol_1: String) -> Result<(), String> {
    validate_token(&symbol_0)?;
    validate_token(&symbol_1)?;
    if symbol_1 == symbol_0 {
        return Err(format!("send token {} == receive token {}", symbol_1, symbol_0));
    }

    let (token_0, token_1) = sort_token_pair(&symbol_0, &symbol_1);
    let book_name = BookName::new(&token_0, &token_1);
    match STABLE_AVAILABLE_TOKEN_POOLS.with_borrow_mut(|m| m.insert(book_name)) {
        true => Ok(()),
        false => Err(format!("{}/{} already exists", symbol_0, symbol_1)),
    }
}

pub fn remove_available_token_pair_impl(token_0: &String, token_1: &String) -> bool {
    let (token_0, token_1) = sort_token_pair(token_0, token_1);
    let book_name = BookName::new(token_0, token_1);
    STABLE_AVAILABLE_TOKEN_POOLS.with_borrow_mut(|m| m.remove(&book_name))
}

pub fn get_max_orders_per_instruments() -> usize {
    STABLE_LIMIT_ORDER_SETTINGS.with_borrow(|s| s.get().max_orders_per_instrument)
}

pub fn get_kong_backend() -> String {
    STABLE_LIMIT_ORDER_SETTINGS.with_borrow(|s| s.get().kong_backend.clone())
}

// pub fn get_limit_backend() -> String {
//     STABLE_LIMIT_ORDER_SETTINGS.with_borrow(|s| s.get().limit_backend.clone())
// }

pub fn get_and_inc_next_claim_id() -> u64 {
    STABLE_LIMIT_ORDER_SETTINGS.with_borrow_mut(|s| {
        let res = s.get().next_claim_id;
        let _ = s.set(LimitOrderSettings {
            next_claim_id: res + 1,
            ..s.get().clone()
        });
        res
    })
}

pub fn get_twap_default_seconds_delay_after_failure() -> u64 {
    STABLE_LIMIT_ORDER_SETTINGS.with_borrow(|s| s.get().twap_default_seconds_delay_after_failure)
}


// pub fn get_stable_token(token_id: &StableTokenId) -> Option<StableToken> {
//     TOKEN_MAP.with_borrow(|tm| tm.get(token_id))
// }

pub fn get_token_by_address(address: &str) -> Option<StableToken> {
    TOKEN_MAP.with_borrow(|tm| tm.iter().find(|t| t.1.address() == address).map(|t| t.1))
}

pub fn get_token_by_symbol(symbol: &str) -> Option<StableToken> {
    TOKEN_MAP.with_borrow(|tm| tm.get(&symbol.to_string()))
}

// pub fn add_stable_token(token: StableToken) {
//     match &token {
//         StableToken::LP(_) => {
//             ic_cdk::eprintln!("LPToken is invalid token kind");
//             return;
//         }
//         StableToken::IC(_) => {}
//     };

//     TOKEN_MAP.with_borrow_mut(|tm| _ = tm.insert(token.name(), token));
// }

#[pre_upgrade]
fn pre_upgrade() {
    // save claims
    STABLE_CLAIMS.with_borrow_mut(|stable_claims| {
        stable_claims.clear_new();
        for (k, v) in crate::token_management::claim_map::get_all_claims() {
            stable_claims.insert(k, v);
        }
    });

    // save prices
    STABLE_PRICE_OBSERVER.with_borrow_mut(|stable_price_observer| {
        let _ = stable_price_observer.set(PRICE_OBSERVER.with(|price_observer| price_observer.borrow().clone()));
    });

    // save twap executor
    STABLE_TWAP_EXECUTOR.with_borrow_mut(|stable_twap_executor| {
        let _ = stable_twap_executor.set(TWAP_EXECUTOR.with(|twap_executor| twap_executor.borrow().clone()));
    });

    // save orderbooks
    STABLE_ORDERBOOKS.with_borrow_mut(|stable_orderbooks| {
        let mut m = StorableVec::new();

        ORDERBOOKS.with_borrow(|runtime_orderbooks| {
            for book in runtime_orderbooks.values() {
                m.0.push(book.borrow().clone());
            }
        });

        match stable_orderbooks.set(m) {
            Ok(_) => {}
            Err(e) => {
                ic_cdk::eprintln!("Failed to save orderbooks, error={:?}", e)
            }
        }
    });

    // save order history
    STABLE_ORDER_HISTORY.with_borrow_mut(|stable_order_history| {
        stable_order_history.clear_new();
        ORDER_HISTORY.with_borrow(|runtime_order_history| {
            for (book_name, storage) in runtime_order_history {
                stable_order_history.insert(book_name.clone(), storage.clone());
            }
        })
    });
}

#[post_upgrade]
fn post_upgrade() {
    // load claims
    STABLE_CLAIMS.with_borrow(|stable_claims| {
        for claim in stable_claims.values() {
            crate::token_management::claim_map::insert(claim);
        }
    });

    // load prices
    STABLE_PRICE_OBSERVER.with_borrow(|stable_price_observer| {
        PRICE_OBSERVER.with(|price_observer| *price_observer.borrow_mut() = stable_price_observer.get().clone());
    });

    // load twap executor
    STABLE_TWAP_EXECUTOR.with_borrow(|stable_twap_executor| {
        TWAP_EXECUTOR.with(|twap_executor| *twap_executor.borrow_mut() = stable_twap_executor.get().clone());
    });

    // load orderbooks
    ORDERBOOKS.with_borrow_mut(|runtime_orderbooks| {
        STABLE_ORDERBOOKS.with_borrow(|stable_orderbooks| {
            for book in stable_orderbooks.get().0.iter() {
                runtime_orderbooks.insert(book.name.clone(), Rc::new(RefCell::new(book.clone())));
            }
        });
    });

    // load order history
    ORDER_HISTORY.with_borrow_mut(|runtime_order_history| {
        STABLE_ORDER_HISTORY.with_borrow(|stable_order_history| {
            for (book_name, storage) in stable_order_history.iter() {
                runtime_order_history.insert(book_name, storage);
            }
        })
    });

    // Update paths
    STABLE_AVAILABLE_TOKEN_POOLS.with_borrow(|token_pairs| {
        for token_pair in token_pairs.iter() {
            if let Err(e) = add_token_pair(token_pair.receive_token().to_string(), token_pair.send_token().to_string()) {
                ic_cdk::eprintln!("Error adding new token pair, e={}", e);
                continue;
            };
            let max_hops = STABLE_LIMIT_ORDER_SETTINGS.with_borrow(|s| s.get().synthetic_orderbook_max_hops);
            add_to_synth_path(&token_pair.receive_token(), &token_pair.send_token(), max_hops);
        }
    })
}
