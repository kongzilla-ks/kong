use kong_lib::stable_token::stable_token::{StableToken, StableTokenId};
use kong_lib::stable_token::token::Token;
use num_traits::ToPrimitive;
use postgres_types::{FromSql, ToSql};
use regex::Regex;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use tokio_postgres::Client;

use super::kong_update::KongUpdate;

#[derive(Debug, ToSql, FromSql)]
#[postgres(name = "token_type")]
enum TokenType {
    #[postgres(name = "IC")]
    IC,
    #[postgres(name = "LP")]
    LP,
}

pub fn serialize_token(token: &StableToken) -> serde_json::Value {
    match token {
        StableToken::IC(token) => json!({
            "IC": {
                "token_id": token.token_id,
                "name": token.name,
                "symbol": token.symbol,
                "canister_id": token.canister_id.to_string(),
                "decimals": token.decimals,
                "fee": token.fee.to_string(),
                "icrc1": token.icrc1,
                "icrc2": token.icrc2,
                "icrc3": token.icrc3,
                "is_removed": token.is_removed,
            }
        }),
        StableToken::LP(token) => json!({
            "LP": {
                "token_id": token.token_id,
                "symbol": token.symbol,
                "address": token.address,
                "decimals": token.decimals,
                "is_removed": token.is_removed,
            }
        }),
    }
}

pub async fn update_tokens_on_database(db_client: &Client) -> Result<BTreeMap<u32, u8>, Box<dyn std::error::Error>> {
    let dir_path = "./backups";
    let re_pattern = Regex::new(r"^tokens.*.json$").unwrap();
    let mut files = fs::read_dir(dir_path)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            if re_pattern.is_match(entry.file_name().to_str().unwrap()) {
                Some(entry)
            } else {
                None
            }
        })
        .map(|entry| {
            // sort by the number in the filename
            let file = entry.path();
            let filename = Path::new(&file).file_name().unwrap().to_str().unwrap();
            let number_str = filename.split('.').nth(1).unwrap();
            let number = number_str.parse::<u32>().unwrap();
            (number, file)
        })
        .collect::<Vec<_>>();
    files.sort_by(|a, b| a.0.cmp(&b.0));

    for file in files {
        let file = File::open(file.1)?;
        let reader = BufReader::new(file);
        let tokens_map: BTreeMap<StableTokenId, StableToken> = serde_json::from_reader(reader)?;

        for v in tokens_map.values() {
            insert_token_on_database(v, db_client).await?;
        }
    }

    load_tokens_from_database(db_client).await
}

pub async fn insert_token_on_database(v: &StableToken, db_client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let (token_id, type_type, name, symbol, address, canister_id, decimals, fee, icrc1, icrc2, icrc3, is_removed, raw_json) = match v {
        StableToken::IC(token) => {
            let decimals = 10_u64.pow(token.decimals as u32) as f64;
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
                token.is_removed,
                json!(serialize_token(v)),
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
            token.is_removed,
            json!(serialize_token(v)),
        ),
    };

    db_client
        .execute(
            "INSERT INTO tokens 
                (token_id, token_type, name, symbol, address, canister_id, decimals, fee, icrc1, icrc2, icrc3, is_removed, raw_json)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
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
                    is_removed = $12,
                    raw_json = $13",
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
                &is_removed,
                &raw_json,
            ],
        )
        .await?;

    println!("token_id={} saved", v.token_id());

    Ok(())
}

pub async fn load_tokens_from_database(db_client: &Client) -> Result<BTreeMap<u32, u8>, Box<dyn std::error::Error>> {
    let mut tokens_map = BTreeMap::new();
    let rows = db_client.query("SELECT token_id, decimals FROM tokens", &[]).await?;
    for row in rows {
        let token_id: i32 = row.get(0);
        let decimals: i16 = row.get(1);
        tokens_map.insert(token_id as u32, decimals as u8);
    }

    Ok(tokens_map)
}

pub async fn update_tokens<T: KongUpdate>(kong_data: &T) -> Result<(), Box<dyn std::error::Error>> {
    let dir_path = "./backups";
    let re_pattern = Regex::new(r"^tokens.*.json$").unwrap();
    let mut files = fs::read_dir(dir_path)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            if re_pattern.is_match(entry.file_name().to_str().unwrap()) {
                Some(entry)
            } else {
                None
            }
        })
        .map(|entry| {
            // sort by the number in the filename
            let file = entry.path();
            let filename = Path::new(&file).file_name().unwrap().to_str().unwrap();
            let number_str = filename.split('.').nth(1).unwrap();
            let number = number_str.parse::<u32>().unwrap();
            (number, file)
        })
        .collect::<Vec<_>>();
    files.sort_by(|a, b| a.0.cmp(&b.0));

    for file in files {
        println!("processing: {:?}", file.1.file_name().unwrap());
        let file = File::open(file.1)?;
        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        reader.read_to_string(&mut contents)?;
        kong_data.update_tokens(&contents).await?;
    }

    Ok(())
}
