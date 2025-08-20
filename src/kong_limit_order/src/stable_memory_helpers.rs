use crate::{orderbook::book_name::BookName, stable_memory::STABLE_AVAILABLE_ORDERBOOKS};

fn validate_token(token: &String) -> Result<(), String> {
    if token.is_empty() {
        return Err("Token is empty".to_string());
    }

    if !token.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err("Only ascii alphabeic is supported".to_string());
    }

    Ok(())
}

fn is_available_token_pair(token_0: &String, token_1: &String) -> bool {
    let book_name = BookName::new(token_0, token_1);
    STABLE_AVAILABLE_ORDERBOOKS.with_borrow(|m| m.contains(&book_name))
}

pub fn get_available_orderbook_name(token_0: &String, token_1: &String) -> Result<BookName, String> {
    if is_available_token_pair(token_0, token_1) {
        return Ok(BookName::new(token_0, token_1));
    }

    if is_available_token_pair(token_1, token_0) {
        return Ok(BookName::new(token_1, token_0));
    }

    return Err(format!("Available orderbook not found, token_0={}, token_1={}", token_0, token_1));
}

pub fn add_available_token_pair(token_0: String, token_1: String) -> Result<(), String> {
    validate_token(&token_0)?;
    validate_token(&token_1)?;

    let book_name = BookName::new(&token_0, &token_1);
    match STABLE_AVAILABLE_ORDERBOOKS.with_borrow_mut(|m| m.insert(book_name)) {
        true => Ok(()),
        false => Err(format!("{}/{} already exists", token_0, token_1)),
    }
}

pub fn remove_available_token_pair(token_0: String, token_1: String) -> bool {
    let book_name = BookName::new(&token_0, &token_1);
    STABLE_AVAILABLE_ORDERBOOKS.with_borrow_mut(|m| m.remove(&book_name))
}
