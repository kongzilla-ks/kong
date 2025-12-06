use ic_cdk::{query, update};
use kong_lib::helpers::json_helpers;

use crate::{guards::caller_is_kingkong, limit_order_settings::LimitOrderSettings, stable_memory::STABLE_LIMIT_ORDER_SETTINGS};

#[query(hidden = true, guard = "caller_is_kingkong")]
fn backup_limit_order_settings() -> Result<String, String> {
    let limit_order_settings = STABLE_LIMIT_ORDER_SETTINGS.with(|m| m.borrow().get().clone());
    serde_json::to_string(&limit_order_settings).map_err(|e| format!("Failed to serialize: {}", e))
}

#[update(hidden = true, guard = "caller_is_kingkong")]
fn set_limit_order_settings(update_settings: String) -> Result<String, String> {
    // get current Kong settings
    let limit_order_settings_value = STABLE_LIMIT_ORDER_SETTINGS
        .with(|m| serde_json::to_value(m.borrow().get()))
        .map_err(|e| format!("Failed to serialize Kong settings: {}", e))?;

    // get updates and merge them into Kong settings
    let updates = serde_json::from_str(&update_settings).map_err(|e| format!("Failed to parse update Kong settings: {}", e))?;
    let mut limit_order_settings_value = limit_order_settings_value;
    json_helpers::merge(&mut limit_order_settings_value, &updates);

    let limit_order_settings: LimitOrderSettings =
        serde_json::from_value(limit_order_settings_value).map_err(|e| format!("Failed to parse updated Kong settings: {}", e))?;

    STABLE_LIMIT_ORDER_SETTINGS.with(|m| {
        m.borrow_mut()
            .set(limit_order_settings.clone())
            .map_err(|_| "Failed to update Kong settings".to_string())?;
        serde_json::to_string(&limit_order_settings).map_err(|e| format!("Failed to serialize: {}", e))
    })
}
