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

pub fn get_price_from_volumes(receive_amount: Nat, send_amount: Nat) -> Price {
    Price(StorableRational::new(send_amount, receive_amount).expect("denominator should not be zero"))
}

impl PriceObserverVolumes {
    pub fn get_price(&self) -> Price {
        get_price_from_volumes(self.receive_amount.clone(), self.send_amount.clone())
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
    pub fn update_volumes(
        &mut self,
        token_0: &str,
        volume_0: Nat,
        token_1: &str,
        volume_1: Nat,
    ) -> Option<(BookName, PriceObserverVolumes, PriceObserverVolumes)> {
        if volume_0 == Nat::default() || volume_1 == Nat::default() {
            ic_cdk::eprintln!("Zero volume for tokens: {}/{}", token_0, token_1);
            return None;
        }
        if token_0 == token_1 {
            ic_cdk::eprintln!("token_0 should not be equal token_1, name={}", token_0);
            return None;
        }

        let (token_0, volume_0, token_1, volume_1) = if token_0 < token_1 {
            (token_0, volume_0, token_1, volume_1)
        } else {
            (token_1, volume_1, token_0, volume_0)
        };

        let book_name = BookName::new(&token_0, &token_1);
        let current_volumes = PriceObserverVolumes {
            receive_amount: volume_0,
            send_amount: volume_1,
        };
        let prev_volumes = self.volumes.insert(book_name.clone(), current_volumes.clone());

        prev_volumes.map(|prev_volumes| (book_name, prev_volumes, current_volumes))
    }

    pub fn get_price(&self, receive_symbol: &str, send_symbol: &str) -> Option<Price> {
        if receive_symbol < send_symbol {
            let book_name = BookName::new(&receive_symbol, &send_symbol);
            self.volumes.get(&book_name).map(|t| t.get_price())
        } else {
            let book_name = BookName::new(&receive_symbol, &send_symbol).reversed();
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

pub fn get_price(receive_token: &str, send_token: &str) -> Option<Price> {
    PRICE_OBSERVER.with(|price_observer| price_observer.borrow().get_price(receive_token, send_token))
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
