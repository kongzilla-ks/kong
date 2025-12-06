use crate::stable_memory_helpers::get_kong_backend;

pub fn caller_is_kingkong() -> Result<(), String> {
    let caller = ic_cdk::api::msg_caller();
    if ic_cdk::api::is_controller(&caller) {
        return Ok(());
    }

    if caller.to_text() == get_kong_backend() {
        return Ok(());
    }

    return Err("Caller is not King Kong".to_string());
}
