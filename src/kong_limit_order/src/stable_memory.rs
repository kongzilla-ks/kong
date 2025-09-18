use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableCell};
use ic_stable_structures::{StableBTreeMap, StableBTreeSet};
use kong_lib::stable_token::stable_token::StableToken;
use kong_lib::storable_vec::StorableVec;
use std::cell::RefCell;

use crate::limit_order_settings::LimitOrderSettings;
use crate::orderbook::book_name::BookName;
use crate::orderbook::order_storage::OrderStorage;
use crate::orderbook::orderbook::SidedOrderBook;
use crate::price_observer::price_observer::PriceObserver;
use crate::token_management::claim::Claim;
use crate::twap::twap_executor::TwapExecutor;

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

// Stable memory
pub const LIMIT_ORDER_SETTINGS_MEMORY_ID: MemoryId = MemoryId::new(1);
pub const ORDERBOOKS_MEMORY_ID: MemoryId = MemoryId::new(2);
// pub const ORDERBOOKS_ORDER_STATUSES: MemoryId = MemoryId::new(3);
pub const AVAILABLE_ORDERBOOKS_MEMORY_ID: MemoryId = MemoryId::new(4);
pub const ORDERBOOK_PRICES_MEMORY_ID: MemoryId = MemoryId::new(5);
pub const ORDER_HISTORY_MEMORY_ID: MemoryId = MemoryId::new(6);
pub const CLAIM_MEMORY_ID: MemoryId = MemoryId::new(7);
pub const TOKEN_MEMORY_ID: MemoryId = MemoryId::new(8);
pub const PRICE_OBSERVER_ID: MemoryId = MemoryId::new(10);
pub const TWAP_EXECUTOR_ID: MemoryId = MemoryId::new(11);

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    // stable memory for storing Kong settings
    pub static STABLE_LIMIT_ORDER_SETTINGS: RefCell<StableCell<LimitOrderSettings, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableCell::init(memory_manager.get(LIMIT_ORDER_SETTINGS_MEMORY_ID), LimitOrderSettings::default()).expect("Failed to initialize Limit order settings"))
    });

    pub static STABLE_ORDERBOOKS: RefCell<StableCell<StorableVec<SidedOrderBook>, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableCell::init(memory_manager.get(ORDERBOOKS_MEMORY_ID), StorableVec::new()).expect("Failed to initialize orderbooks"))
    });

    // available pools
    pub static STABLE_AVAILABLE_TOKEN_POOLS: RefCell<StableBTreeSet<BookName, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableBTreeSet::init(memory_manager.get(AVAILABLE_ORDERBOOKS_MEMORY_ID)))
    });

    pub static STABLE_ORDER_HISTORY: RefCell<StableBTreeMap<BookName, OrderStorage, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableBTreeMap::init(memory_manager.get(ORDER_HISTORY_MEMORY_ID)))
    });

    pub static STABLE_CLAIMS: RefCell<StableBTreeMap<u64, Claim, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableBTreeMap::init(memory_manager.get(CLAIM_MEMORY_ID)))
    });

    pub static TOKEN_MAP: RefCell<StableBTreeMap<String, StableToken, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableBTreeMap::init(memory_manager.get(TOKEN_MEMORY_ID)))
    });

    pub static STABLE_PRICE_OBSERVER: RefCell<StableCell<PriceObserver, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableCell::init(memory_manager.get(PRICE_OBSERVER_ID), PriceObserver::default()).expect("Failed to initialize price observer"))
    });

    pub static STABLE_TWAP_EXECUTOR: RefCell<StableCell<TwapExecutor, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableCell::init(memory_manager.get(TWAP_EXECUTOR_ID), TwapExecutor::default()).expect("Failed to initialize twap executor"))
    });


}

fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R) -> R {
    MEMORY_MANAGER.with(|cell| f(&cell.borrow()))
}
