use candid::Principal;
use std::cell::RefCell;
use std::collections::HashSet;

const DEFAULT_ADMIN_PRINCIPALS: [&str; 5] = [
    "4jxje-hbmra-4otqc-6hor3-cpwlh-sqymk-6h4ef-42sqn-o3ip5-s3mxk-uae",
    "6rjil-isfbu-gsmpe-ffvcl-v3ifl-xqgkr-en2ir-pbr54-cetku-syp4i-bae",
    // Shill principals below
    "7ohni-sbpse-y327l-syhzk-jn6n4-hw277-erei5-xhkjr-lbh6b-rjqei-sqe",
    "6ydau-gqejl-yqbq7-tm2i5-wscbd-lsaxy-oaetm-dxddd-s5rtd-yrpq2-eae",
    "bc4tr-kdoww-zstxb-plqge-bo6ho-abuc2-mft22-6tdpb-5ofll-yknor-sae",
];

thread_local! {
    static ADMIN_PRINCIPALS: RefCell<HashSet<Principal>> = RefCell::new({
        let mut admins = HashSet::new();
        // Add canister_id as admin
        admins.insert(ic_cdk::api::id());
        // Add admin principal ids as admin
        for principal in DEFAULT_ADMIN_PRINCIPALS.iter() {
            if let Ok(principal) = Principal::from_text(principal) {
                admins.insert(principal);
            }
        }
        admins
    });
}

/// Checks if a principal is an admin
pub fn is_admin(principal: Principal) -> bool {
    ADMIN_PRINCIPALS.with(|admins| admins.borrow().contains(&principal))
}
