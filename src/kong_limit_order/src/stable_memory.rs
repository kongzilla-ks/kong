use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableCell};
use ic_stable_structures::{StableBTreeMap, StableBTreeSet};
use kong_lib::storable_vec::StorableVec;
use std::cell::RefCell;

use crate::limit_order_settings::LimitOrderSettings;
use crate::orderbook::book_name::BookName;
use crate::orderbook::order_storage::OrderStorage;
use crate::orderbook::orderbook::SidedOrderBook;

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
}

fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R) -> R {
    MEMORY_MANAGER.with(|cell| f(&cell.borrow()))
}
