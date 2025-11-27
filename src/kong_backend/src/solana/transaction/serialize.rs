//! Transaction serialization for Solana
//!
//! This module handles the serialization of transaction messages into the binary format
//! expected by Solana.

use anyhow::{anyhow, Result};
use candid::{CandidType, Nat, Principal};
use serde::Deserialize;
use std::collections::HashMap;

/// HTTP request arguments with is_replicated support
#[derive(CandidType, Debug)]
struct HttpRequestArgs {
    url: String,
    max_response_bytes: Option<u64>,
    method: HttpMethod,
    headers: Vec<HttpHeader>,
    body: Option<Vec<u8>>,
    transform: Option<TransformArgs>,
    is_replicated: Option<bool>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
struct HttpHeader {
    name: String,
    value: String,
}

#[derive(CandidType, Debug)]
struct TransformArgs {
    function: TransformFunc,
    context: Vec<u8>,
}

#[derive(CandidType, Debug)]
struct TransformFunc {
    principal: Principal,
    name: String,
}

#[derive(CandidType, Debug)]
#[allow(non_camel_case_types)]
enum HttpMethod {
    get,
    post,
    head,
}

/// HTTP response - matches IC management canister interface
#[derive(CandidType, Deserialize, Debug)]
struct HttpResponse {
    status: Nat,
    headers: Vec<HttpHeader>,
    body: Vec<u8>,
}

use super::super::error::SolanaError;
use super::super::sdk::compiled_instruction::CompiledInstruction;
use super::super::sdk::instruction::Instruction;
use super::super::utils::base58;

const HELIUS_ENDPOINT: &str =
    "https://mainnet.helius-rpc.com/?api-key=18627637-bbdf-41b5-b8cf-e9c9d92aaabe";
const HTTP_OUTCALL_CYCLES: u128 = 500_000_000;

/// Message header for Solana transactions
#[derive(Debug, Clone)]
pub struct MessageHeader {
    pub num_required_signatures: u8,
    pub num_readonly_signed_accounts: u8,
    pub num_readonly_unsigned_accounts: u8,
}

/// Transaction message structure
#[derive(Debug, Clone)]
pub struct Message {
    pub header: MessageHeader,
    pub account_keys: Vec<String>, // Pubkey strings
    pub recent_blockhash: String,
    pub instructions: Vec<CompiledInstruction>,
}

impl Message {
    /// Create a new message from instructions (Refactored for safety, performance, and clarity)
    pub fn new(instructions: Vec<Instruction>, payer: &str) -> Result<Self> {
        // 1. Collect all accounts and their properties into a HashMap
        let mut accounts_map: HashMap<String, (bool, bool)> = HashMap::new();
        instructions
            .iter()
            .flat_map(|inst| {
                // Create a single iterator over the program_id and all instruction accounts
                std::iter::once((&inst.program_id, false, false)) // Program IDs are never signers or writable
                    .chain(inst.accounts.iter().map(|acc| (&acc.pubkey, acc.is_signer, acc.is_writable)))
            })
            .for_each(|(key, is_signer, is_writable)| {
                let entry = accounts_map.entry(key.clone()).or_insert((false, false));
                entry.0 |= is_signer;
                entry.1 |= is_writable;
            });

        // Ensure the payer is in the map as a writable signer
        accounts_map.insert(payer.to_string(), (true, true));

        // 2. Sort the accounts with payer first automatically
        let mut sorted_accounts: Vec<_> = accounts_map.into_iter().collect();
        sorted_accounts.sort_by_key(|(key, (is_signer, is_writable))| {
            // Primary sort key: is this the payer? (false comes before true, so payer is first)
            // Secondary sort keys: the standard Solana account order
            (
                key != payer, // Payer gets `false`, everyone else `true`
                match (*is_signer, *is_writable) {
                    (true, true) => 0,
                    (true, false) => 1,
                    (false, true) => 2,
                    (false, false) => 3,
                },
            )
        });

        // 3. Extract final keys and calculate header values in a single pass
        let account_keys: Vec<String> = sorted_accounts.iter().map(|(key, _)| key.clone()).collect();
        let header = sorted_accounts.iter().fold(
            MessageHeader {
                num_required_signatures: 0,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            |mut acc, (_, (is_signer, is_writable))| {
                if *is_signer {
                    acc.num_required_signatures += 1;
                    if !*is_writable {
                        acc.num_readonly_signed_accounts += 1;
                    }
                } else if !*is_writable {
                    acc.num_readonly_unsigned_accounts += 1;
                }
                acc
            },
        );

        // 4. Create a reverse lookup map for efficient and safe index resolution
        let key_to_index_map: HashMap<&str, u8> = account_keys.iter().enumerate().map(|(i, key)| (key.as_str(), i as u8)).collect();

        // 5. Compile instructions using the fast lookup map
        let compiled_instructions: Result<Vec<CompiledInstruction>> = instructions
            .into_iter()
            .map(|inst| {
                // Let `?` handle the conversion from SolanaError to anyhow::Error
                let program_id_index = *key_to_index_map
                    .get(inst.program_id.as_str())
                    .ok_or_else(|| SolanaError::TransactionBuildError(format!("Program ID {} not found in key map", inst.program_id)))?;

                let account_indices: Result<Vec<u8>> = inst
                    .accounts
                    .iter()
                    .map(|acc| {
                        key_to_index_map
                            .get(acc.pubkey.as_str())
                            .copied()
                            .ok_or_else(|| SolanaError::TransactionBuildError(format!("Account {} not found in key map", acc.pubkey)))
                    })
                    .collect::<Result<Vec<u8>, _>>() // Specify the collection type to handle the inner Result
                    .map_err(anyhow::Error::from); // Convert the error type for the whole collection at once

                Ok(CompiledInstruction {
                    program_id_index,
                    accounts: account_indices?,
                    data: inst.data,
                })
            })
            .collect();

        Ok(Message {
            header,
            account_keys,
            recent_blockhash: String::new(),
            instructions: compiled_instructions?,
        })
    }

    /// Set the recent blockhash
    pub fn with_blockhash(mut self, blockhash: String) -> Self {
        self.recent_blockhash = blockhash;
        self
    }

    /// Serialize the message for signing
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut data = vec![
            self.header.num_required_signatures,
            self.header.num_readonly_signed_accounts,
            self.header.num_readonly_unsigned_accounts,
        ];

        // Account keys
        data.push(self.account_keys.len() as u8);
        for key in &self.account_keys {
            let key_bytes = base58::decode_public_key(key)
                .map_err(|e| SolanaError::InvalidPublicKeyFormat(format!("Invalid pubkey {}: {}", key, e)))?;
            data.extend_from_slice(&key_bytes);
        }

        // Recent blockhash (32 bytes)
        let blockhash_bytes = base58::decode_public_key(&self.recent_blockhash)
            .map_err(|e| SolanaError::InvalidBlockhash(format!("Invalid blockhash: {}", e)))?;
        data.extend_from_slice(&blockhash_bytes);

        // Instructions
        data.push(self.instructions.len() as u8);
        for instruction in &self.instructions {
            data.push(instruction.program_id_index);
            data.push(instruction.accounts.len() as u8);
            data.extend_from_slice(&instruction.accounts);
            data.push(instruction.data.len() as u8);
            data.extend_from_slice(&instruction.data);
        }

        Ok(data)
    }
}

/// Serialize a message from instructions, fetching blockhash via non-replicated HTTP outcall
pub async fn serialize_message(instructions: Vec<Instruction>, payer: &str) -> Result<Vec<u8>> {
    let blockhash = fetch_solana_blockhash().await?;
    let message = Message::new(instructions, payer)?.with_blockhash(blockhash);
    message.serialize()
}

/// Fetch latest Solana blockhash from Helius RPC (non-replicated HTTP outcall)
async fn fetch_solana_blockhash() -> Result<String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestBlockhash",
        "params": [{"commitment": "finalized"}]
    });

    let request = HttpRequestArgs {
        url: HELIUS_ENDPOINT.to_string(),
        max_response_bytes: Some(1024),
        method: HttpMethod::post,
        headers: vec![HttpHeader {
            name: "Content-Type".to_string(),
            value: "application/json".to_string(),
        }],
        body: Some(body.to_string().into_bytes()),
        transform: None,
        is_replicated: Some(false),
    };

    let (response,): (HttpResponse,) = ic_cdk::api::call::call_with_payment128(
        Principal::management_canister(),
        "http_request",
        (request,),
        HTTP_OUTCALL_CYCLES,
    )
    .await
    .map_err(|(code, msg)| anyhow!("HTTP outcall failed: {code:?} - {msg}"))?;

    let status_u64: u64 = response.status.0.try_into().unwrap_or(0);
    if status_u64 != 200 {
        let err_body = String::from_utf8_lossy(&response.body);
        return Err(anyhow!("Helius RPC error {status_u64}: {err_body}"));
    }

    let json: serde_json::Value = serde_json::from_slice(&response.body)
        .map_err(|e| anyhow!("JSON parse failed: {e}"))?;

    json.get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.get("blockhash"))
        .and_then(|b| b.as_str())
        .map(String::from)
        .ok_or_else(|| {
            let err_body = String::from_utf8_lossy(&response.body);
            anyhow!("Missing blockhash in response: {err_body}")
        })
}
