use std::{cell::RefCell, collections::HashMap, fmt};

use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::orderbook::book_name::BookName;

#[derive(CandidType, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path(pub Vec<BookName>);

impl Path {
    pub fn with_added_step(&self, book_name: BookName) -> Self {
        let mut res = self.0.clone();
        assert!(self.receive_token() == book_name.send_token());
        res.push(book_name);
        Self(res)
    }

    pub fn send_token(&self) -> &str {
        self.0.first().unwrap().send_token()
    }

    pub fn receive_token(&self) -> &str {
        self.0.last().unwrap().receive_token()
    }

    pub fn get_book_name(&self) -> BookName {
        BookName::new(self.receive_token(), self.send_token())
    }

    pub fn from_book_name(b: BookName) -> Self {
        Path(vec![b])
    }

    pub fn contains_book_name(&self, b: &BookName) -> bool {
        self.0.iter().find(|&v| v == b).is_some()
    }

    #[allow(dead_code)]
    pub fn check_path_valid(&self) {
        #[cfg(test)]
        {
            for i in 1..self.0.len() {
                assert!(self.0[i].send_token() == self.0[i - 1].receive_token());
            }
        }
    }

    pub fn to_symbol_sequence(&self) -> Vec<String> {
        self.check_path_valid();

        let mut res: Vec<String> = self.0.iter().map(|b| b.send_token().to_string()).collect();
        res.push(self.0.last().unwrap().receive_token().to_string());
        res
    }

    pub fn has_loops(&self) -> bool {
        let mut symbol_sequence = self.to_symbol_sequence();
        symbol_sequence.sort();

        for i in 1..symbol_sequence.len() {
            if symbol_sequence[i - 1] == symbol_sequence[i] {
                return true;
            }
        }

        return false;
    }

    pub fn reversed(&self) -> Self {
        let mut res = self.clone();
        res.0.reverse();
        for name in res.0.iter_mut() {
            name.reverse();
        }
        res
    }
}


impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.to_symbol_sequence().join(", "))
    }
}

thread_local! {
    pub static TOKEN_PAIRS: RefCell<HashMap<String, Vec<String>>> = RefCell::default();
    pub static TOKEN_PATHS: RefCell<HashMap<BookName, Vec<Path>>> = RefCell::default();

    // orderbooks, that are end of paths
    // e.g. we have orderbook sequence 'a-b-c-d' and its reversed 'd-c-b-a' in token_paths
    // border paths will be the following: {'a-b': ['a-b-c-d'], 'c-d': ['a-b-c-d'], 'd-c': ['d-c-b-a'], 'b-a': ['d-c-b-a']}
    // This map is useful for updating prices for all related
    pub static BORDER_PATHS: RefCell<HashMap<BookName, Vec<Path>>> = RefCell::default();
}

pub fn is_available_token_path(token_0: &str, token_1: &str) -> bool {
    TOKEN_PATHS.with_borrow(|p| p.contains_key(&BookName::new(token_0, token_1)))
}

pub fn get_border_paths_by_id(bookname: &BookName, idx: usize) -> Option<Path> {
    BORDER_PATHS.with_borrow(|border_paths| border_paths.get(bookname).and_then(|paths| paths.get(idx).cloned()))
}
