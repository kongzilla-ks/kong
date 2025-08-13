// TODO: these settings require adjusting after merging to main kong repo
// TODO: Gorazd/Jon dbl check

pub const KONG_TOKEN_ID: u32 = if cfg!(any(feature = "local", feature = "staging")) {
    8
} else {
    103
};

// pub const KONG_SYMBOL: &str = if cfg!(any(feature = "local", feature = "staging")) {
//     "ksKONG"
// } else {
//     "KONG"
// };
// pub const KONG_SYMBOL_WITH_CHAIN: &str = if cfg!(any(feature = "local", feature = "staging")) {
//     "IC.ksKONG"
// } else {
//     "IC.KONG"
// };
// pub const KONG_ADDRESS: &str = if cfg!(any(feature = "local", feature = "staging")) {
//     "u6s2n-gx777-77774-qaaba-cai"
// } else {
//     "o7oak-iyaaa-aaaaq-aadzq-cai"
// };
// pub const KONG_ADDRESS_WITH_CHAIN: &str = if cfg!(any(feature = "local", feature = "staging")) {
//     "IC.u6s2n-gx777-77774-qaaba-cai"
// } else {
//     "IC.o7oak-iyaaa-aaaaq-aadzq-cai"
// };

// pub fn is_kong_token_id(token_id: u32) -> bool {
//     token_id == KONG_TOKEN_ID
// }

pub fn get_kong_id() -> u32 {
    KONG_TOKEN_ID
}
