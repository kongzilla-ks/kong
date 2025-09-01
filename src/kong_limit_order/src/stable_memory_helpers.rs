use std::{cell::RefCell, rc::Rc};

use crate::{
    orderbook::{book_name::BookName, order_history::ORDER_HISTORY, orderbook::ORDERBOOKS, orderbook_path_helper::{add_to_synth_path, add_token_pair}},
    stable_memory::{STABLE_AVAILABLE_TOKEN_POOLS, STABLE_LIMIT_ORDER_SETTINGS, STABLE_ORDERBOOKS, STABLE_ORDER_HISTORY},
};
use ic_cdk::{post_upgrade, pre_upgrade};
use kong_lib::storable_vec::StorableVec;

fn validate_token(token: &String) -> Result<(), String> {
    if token.is_empty() {
        return Err("Token is empty".to_string());
    }

    if !token.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err("Only ascii alphabeic is supported".to_string());
    }

    Ok(())
}

fn sort_token_pair<'a>(receive_token: &'a str, send_token: &'a str) -> (&'a str, &'a str) {
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

pub fn add_available_token_pair_impl(receive_token: String, send_token: String) -> Result<(), String> {
    validate_token(&receive_token)?;
    validate_token(&send_token)?;
    if send_token == receive_token {
        return Err(format!("send token {} == receive token {}", send_token, receive_token));
    }

    let (token_0, token_1) = sort_token_pair(&receive_token, &send_token);
    let book_name = BookName::new(&token_0, &token_1);
    match STABLE_AVAILABLE_TOKEN_POOLS.with_borrow_mut(|m| m.insert(book_name)) {
        true => Ok(()),
        false => Err(format!("{}/{} already exists", receive_token, send_token)),
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

#[pre_upgrade]
fn pre_upgrade() {
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
