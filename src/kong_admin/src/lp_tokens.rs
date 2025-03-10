use kong_lib::stable_lp_token::stable_lp_token::{StableLPToken, StableLPTokenId};
use num_traits::ToPrimitive;
use regex::Regex;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use tokio_postgres::Client;

use super::kong_update::KongUpdate;
use super::math_helpers::round_f64;

pub fn serialize_lp_tokens(lp_token: &StableLPToken) -> serde_json::Value {
    json!({
        "StableLPToken": {
            "lp_token_id": lp_token.lp_token_id,
            "user_id": lp_token.user_id,
            "token_id": lp_token.token_id,
            "amount": lp_token.amount.to_string(),
            "ts": lp_token.ts,
        }
    })
}

pub async fn update_lp_tokens_on_database(db_client: &Client, tokens_map: &BTreeMap<u32, u8>) -> Result<(), Box<dyn std::error::Error>> {
    let dir_path = "./backups";
    let re_pattern = Regex::new(r"^lp_tokens.*.json$").unwrap();
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
        let lp_token_ledger_map: BTreeMap<StableLPTokenId, StableLPToken> = serde_json::from_reader(reader)?;

        for v in lp_token_ledger_map.values() {
            insert_lp_token_on_database(v, db_client, tokens_map).await?;
        }
    }

    Ok(())
}

pub async fn insert_lp_token_on_database(
    v: &StableLPToken,
    db_client: &Client,
    tokens_map: &BTreeMap<u32, u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let lp_token_id = v.lp_token_id as i64;
    let user_id = v.user_id as i32;
    let token_id = v.token_id as i32;
    let decimals = tokens_map.get(&v.token_id).ok_or(format!("token_id={} not found", v.token_id))?;
    let amount = round_f64(v.amount.0.to_f64().unwrap() / 10_u64.pow(*decimals as u32) as f64, *decimals);
    let ts = v.ts as f64 / 1_000_000_000.0;
    let raw_json = serialize_lp_tokens(v);

    db_client
        .execute(
            "INSERT INTO lp_tokens 
                (lp_token_id, user_id, token_id, amount, ts, raw_json)
                VALUES ($1, $2, $3, $4, to_timestamp($5), $6)
                ON CONFLICT (lp_token_id) DO UPDATE SET
                    user_id = $2,
                    token_id = $3,
                    amount = $4,
                    ts = to_timestamp($5),
                    raw_json = $6",
            &[&lp_token_id, &user_id, &token_id, &amount, &ts, &raw_json],
        )
        .await?;

    println!("lp_token_id={} saved", v.lp_token_id);

    Ok(())
}

pub async fn update_lp_tokens<T: KongUpdate>(kong_update: &T) -> Result<(), Box<dyn std::error::Error>> {
    let dir_path = "./backups";
    let re_pattern = Regex::new(r"^lp_tokens.*.json$").unwrap();
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
        kong_update.update_lp_tokens(&contents).await?;
    }

    Ok(())
}
