use crate::stable_memory::DB_UPDATE_MAP;
use std::sync::atomic::{AtomicU64, Ordering};

use super::stable_db_update::{StableDBUpdate, StableDBUpdateId};

pub static DB_UPDATE_ID: AtomicU64 = AtomicU64::new(0);

pub fn max_db_update_id() -> u64 {
    DB_UPDATE_MAP.with(|m| m.borrow().last_key_value().map_or(0, |(k, _)| k.0))
}

pub fn insert(db_update: &StableDBUpdate) -> u64 {
    DB_UPDATE_MAP.with(|m| {
        let mut map = m.borrow_mut();
        let db_update_id = DB_UPDATE_ID.fetch_add(1, Ordering::SeqCst) + 1;
        let db_update = StableDBUpdate {
            db_update_id,
            ..db_update.clone()
        };
        map.insert(StableDBUpdateId(db_update_id), db_update);
        db_update_id
    })
}
