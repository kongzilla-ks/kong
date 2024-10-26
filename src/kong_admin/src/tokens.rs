use kong_lib::stable_token::stable_token::{StableToken, StableTokenId};
use num_traits::ToPrimitive;
use postgres_types::{FromSql, ToSql};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use tokio_postgres::Client;

#[derive(Debug, ToSql, FromSql)]
#[postgres(name = "token_type")]
enum TokenType {
    #[postgres(name = "IC")]
    IC,
    #[postgres(name = "LP")]
    LP,
}

pub async fn dump_tokens(db_client: &Client) -> Result<BTreeMap<u32, u8>, Box<dyn std::error::Error>> {
    let file = File::open("./backup/tokens.json")?;
    let reader = BufReader::new(file);
    let tokens_map: BTreeMap<StableTokenId, StableToken> = serde_json::from_reader(reader)?;

    for (k, v) in tokens_map.iter() {
        let (token_id, type_type, name, symbol, address, canister_id, decimals, fee, icrc1, icrc2, icrc3, on_kong) = match v {
            StableToken::IC(token) => {
                let decimals = 10_u64.pow(token.decimals as u32 - 1) as f64;
                let fee = token.fee.0.to_f64().unwrap() / decimals;
                (
                    token.token_id as i32,
                    TokenType::IC,
                    Some(token.name.clone()),
                    token.symbol.clone(),
                    None,
                    Some(token.canister_id.to_string()),
                    token.decimals as i16,
                    Some(fee),
                    Some(token.icrc1),
                    Some(token.icrc2),
                    Some(token.icrc3),
                    token.on_kong,
                )
            }
            StableToken::LP(token) => (
                token.token_id as i32,
                TokenType::LP,
                None,
                token.symbol.clone(),
                Some(token.address.clone()),
                None,
                token.decimals as i16,
                None,
                None,
                None,
                None,
                token.on_kong,
            ),
        };

        // insert or update
        db_client
            .execute(
                "INSERT INTO tokens 
                    (token_id, token_type, name, symbol, address, canister_id, decimals, fee, icrc1, icrc2, icrc3, on_kong)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    ON CONFLICT (token_id) DO UPDATE SET
                        token_type = $2,
                        name = $3,
                        symbol = $4,
                        address = $5,
                        canister_id = $6,
                        decimals = $7,
                        fee = $8,
                        icrc1 = $9,
                        icrc2 = $10,
                        icrc3 = $11,
                        on_kong = $12",
                &[
                    &token_id,
                    &type_type,
                    &name,
                    &symbol,
                    &address,
                    &canister_id,
                    &decimals,
                    &fee,
                    &icrc1,
                    &icrc2,
                    &icrc3,
                    &on_kong,
                ],
            )
            .await?;
        println!("token_id={} saved", k.0);
    }

    load_tokens(db_client).await
}

pub async fn load_tokens(db_client: &Client) -> Result<BTreeMap<u32, u8>, Box<dyn std::error::Error>> {
    let mut tokens_map = BTreeMap::new();
    let rows = db_client.query("SELECT token_id, decimals FROM tokens", &[]).await?;
    for row in rows {
        let token_id: i32 = row.get(0);
        let decimals: i16 = row.get(1);
        tokens_map.insert(token_id as u32, decimals as u8);
    }
    Ok(tokens_map)
}