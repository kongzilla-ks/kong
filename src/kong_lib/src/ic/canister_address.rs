pub const KONG_BACKEND: &str = if cfg!(feature = "staging") {
    "l4lgk-raaaa-aaaar-qahpq-cai"
} else {
    "2ipq2-uqaaa-aaaar-qailq-cai"
};

pub const KONG_DATA: &str = if cfg!(feature = "staging") {
    "6ukzc-hiaaa-aaaah-qpxqa-cai"
} else {
    "cbefx-hqaaa-aaaar-qakrq-cai"
};

pub const KONG_LIMIT: &str = if cfg!(feature = "staging") {
    "umunu-kh777-77774-qaaca-cai"
} else {
    // TODO: change me
    "umunu-kh777-77774-qaaca-cai"
};