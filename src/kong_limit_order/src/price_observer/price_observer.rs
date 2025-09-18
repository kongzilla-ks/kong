use std::{cell::RefCell, collections::HashMap, rc::Rc};

use candid::{CandidType, Nat};
use ic_stable_structures::{storable::Bound, Storable};
use kong_lib::storable_rational::StorableRational;
use serde::{Deserialize, Serialize};

use crate::orderbook::{book_name::BookName, orderbook_path::Path, price::Price};

thread_local! {
    pub static PRICE_OBSERVER: Rc<RefCell<PriceObserver>> = Rc::new(RefCell::default());
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize, Default)]
pub struct PriceObserverVolumes {
    receive_amount: Nat,
    send_amount: Nat,
}

impl PriceObserverVolumes {
    pub fn get_price(&self) -> Price {
        Price(StorableRational::new(self.send_amount.clone(), self.receive_amount.clone()).expect("denominator should not be zero"))
    }
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct PriceObserver {
    volumes: HashMap<BookName, PriceObserverVolumes>,
}

impl Default for PriceObserver {
    fn default() -> Self {
        Self {
            volumes: Default::default(),
        }
    }
}

impl PriceObserver {
    pub fn update_volumes(&mut self, token_0: &str, volume_0: Nat, token_1: &str, volume_1: Nat) {
        if volume_0 == Nat::default() || volume_1 == Nat::default() {
            ic_cdk::eprintln!("Zero volume for tokens: {}/{}", token_0, token_1);
            return;
        }
        if token_0 == token_1 {
            ic_cdk::eprintln!("token_0 should not be equal token_1, name={}", token_0);
            return;
        }

        if token_0 < token_1 {
            let book_name = BookName::new(&token_0, &token_1);
            let _ = self.volumes.insert(
                book_name,
                PriceObserverVolumes {
                    receive_amount: volume_0,
                    send_amount: volume_1,
                },
            );
        } else {
            let book_name = BookName::new(&token_1, &token_0);
            let _ = self.volumes.insert(
                book_name,
                PriceObserverVolumes {
                    receive_amount: volume_1,
                    send_amount: volume_0,
                },
            );
        }
    }

    pub fn get_price(&self, receive_token: &str, send_token: &str) -> Option<Price> {
        if receive_token < send_token {
            let book_name = BookName::new(&receive_token, &send_token);
            self.volumes.get(&book_name).map(|t| t.get_price())
        } else {
            let book_name = BookName::new(&receive_token, &send_token).reversed();
            self.volumes.get(&book_name).map(|t| t.get_price().reversed())
        }
    }

    pub fn get_price_path(&self, path: &Path) -> Option<Price> {
        let mut price = Price::one();
        for bookname in &path.0 {
            let p = match self.get_price(bookname.receive_token(), bookname.send_token()) {
                Some(p) => p,
                None => return None,
            };
    
            price.0 *= p.0;
        }
    
        Some(price)
    }
}


pub fn get_price_path(path: &Path) -> Option<Price> {
    PRICE_OBSERVER.with(|price_observer| price_observer.borrow().get_price_path(path))
}

impl Storable for PriceObserver {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        serde_cbor::to_vec(self).expect("Failed to encode PriceObserver").into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        serde_cbor::from_slice(&bytes).expect("Failed to decode PriceObserver")
    }

    const BOUND: ic_stable_structures::storable::Bound = Bound::Unbounded;
}
