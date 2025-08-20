use ic_cdk::{post_upgrade, pre_upgrade};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableCell};
use ic_stable_structures::{StableBTreeMap, StableBTreeSet};
use kong_lib::storable_vec::StorableVec;
use std::cell::RefCell;
use std::rc::Rc;

use crate::limit_order_settings::LimitOrderSettings;
use crate::orderbook::book_name::BookName;
use crate::orderbook::order_history::ORDER_HISTORY;
use crate::orderbook::order_storage::OrderStorage;
use crate::orderbook::orderbook::{OrderBook, ORDERBOOKS};

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

// Stable memory
pub const LIMIT_ORDER_SETTINGS_MEMORY_ID: MemoryId = MemoryId::new(1);
pub const ORDERBOOKS_MEMORY_ID: MemoryId = MemoryId::new(2);
// pub const ORDERBOOKS_ORDER_STATUSES: MemoryId = MemoryId::new(3);
pub const AVAILABLE_ORDERBOOKS_MEMORY_ID: MemoryId = MemoryId::new(4);
pub const ORDERBOOK_PRICES_MEMORY_ID: MemoryId = MemoryId::new(5);
pub const ORDER_HISTORY_MEMORY_ID: MemoryId = MemoryId::new(6);

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    // stable memory for storing Kong settings
    pub static STABLE_LIMIT_ORDER_SETTINGS: RefCell<StableCell<LimitOrderSettings, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableCell::init(memory_manager.get(LIMIT_ORDER_SETTINGS_MEMORY_ID), LimitOrderSettings::default()).expect("Failed to initialize Limit order settings"))
    });

    pub static STABLE_ORDERBOOKS: RefCell<StableCell<StorableVec<OrderBook>, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableCell::init(memory_manager.get(ORDERBOOKS_MEMORY_ID), StorableVec::new()).expect("Failed to initialize orderbooks"))
    });

    pub static STABLE_AVAILABLE_ORDERBOOKS: RefCell<StableBTreeSet<BookName, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableBTreeSet::init(memory_manager.get(AVAILABLE_ORDERBOOKS_MEMORY_ID)))
    });

    pub static STABLE_ORDER_HISTORY: RefCell<StableBTreeMap<BookName, OrderStorage, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableBTreeMap::init(memory_manager.get(ORDER_HISTORY_MEMORY_ID)))
    });
}

fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R) -> R {
    MEMORY_MANAGER.with(|cell| f(&cell.borrow()))
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
}
