use ic_cdk::{query, update};

use crate::stable_db_update::db_update_map;
use crate::stable_db_update::stable_db_update::StableDBUpdate;
use crate::stable_memory::DB_UPDATE_MAP;

const MAX_DB_UPDATES: usize = 1_000;

#[query(hidden = true)]
fn backup_db_updates() -> Result<String, String> {
    DB_UPDATE_MAP.with(|m| {
        let map = m.borrow();
        let db_updates = map.iter().take(MAX_DB_UPDATES).map(|(_, v)| v).collect::<Vec<StableDBUpdate>>();
        serde_json::to_string(&db_updates).map_err(|e| format!("Failed to serialize updates: {}", e))
    })
}

#[update(hidden = true)]
fn remove_db_updates(db_update_id: u64) -> Result<String, String> {
    db_update_map::remove_old_updates(db_update_id);

    Ok("DB updates removed".to_string())
}
