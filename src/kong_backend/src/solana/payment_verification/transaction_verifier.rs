use crate::stable_memory::get_solana_transaction;
use crate::ic::network::ICNetwork;
use candid::Nat;
use num_traits::ToPrimitive;

/// Extract sender from a Solana transaction based on token type
pub async fn extract_solana_sender_from_transaction(tx_signature: &str, is_spl_token: bool) -> Result<String, String> {
    let transaction = get_solana_transaction(tx_signature.to_string())
        .ok_or_else(|| format!("Solana transaction {} not found. Make sure kong_rpc has processed this transaction.", tx_signature))?;
    
    // Parse metadata to extract sender
    if let Some(metadata_json) = &transaction.metadata {
        let metadata: serde_json::Value = serde_json::from_str(metadata_json)
            .map_err(|e| format!("Failed to parse transaction metadata: {}", e))?;
        
        // Extract sender based on token type
        let sender = if is_spl_token {
            // For SPL tokens: use "authority" or "sender_wallet" (the actual wallet that signed)
            metadata.get("authority")
                .or_else(|| metadata.get("sender_wallet"))
                .and_then(|v| v.as_str())
                .ok_or("SPL transaction metadata missing authority/sender_wallet information")?
        } else {
            // For native SOL: use "sender" (the wallet address)
            metadata.get("sender")
                .and_then(|v| v.as_str())
                .ok_or("SOL transaction metadata missing sender information")?
        };
        
        Ok(sender.to_string())
    } else {
        Err("Transaction metadata is missing".to_string())
    }
}

/// Verify a Solana transaction exists and matches expected parameters
pub async fn verify_solana_transaction(
    tx_signature: &str,
    expected_sender: &str,
    expected_amount: &Nat,
    is_spl_token: bool,
) -> Result<(), String> {
    let transaction = get_solana_transaction(tx_signature.to_string())
        .ok_or_else(|| format!("Solana transaction {} not found. Make sure kong_rpc has processed this transaction.", tx_signature))?;
    
    // Check transaction status
    match transaction.status.as_str() {
        "confirmed" | "finalized" => {}, // Good statuses
        "failed" => return Err(format!("Solana transaction {} failed", tx_signature)),
        status => return Err(format!("Solana transaction {} has unexpected status: {}", tx_signature, status)),
    }
    
    // Parse metadata to verify transaction details
    if let Some(metadata_json) = &transaction.metadata {
        let metadata: serde_json::Value = serde_json::from_str(metadata_json)
            .map_err(|e| format!("Failed to parse transaction metadata: {}", e))?;
        
        // Check sender matches based on token type
        let actual_sender = if is_spl_token {
            // For SPL tokens: use "authority" or "sender_wallet"
            metadata.get("authority")
                .or_else(|| metadata.get("sender_wallet"))
                .and_then(|v| v.as_str())
                .ok_or("SPL transaction metadata missing authority/sender_wallet information")?
        } else {
            // For native SOL: use "sender"
            metadata.get("sender")
                .and_then(|v| v.as_str())
                .ok_or("SOL transaction metadata missing sender information")?
        };
            
        if actual_sender != expected_sender {
            return Err(format!(
                "Transaction sender mismatch. Expected: {}, Got: {}",
                expected_sender, actual_sender
            ));
        }
        
        // Check amount matches
        let actual_amount = metadata.get("amount")
            .and_then(|v| v.as_u64())
            .ok_or("Transaction metadata missing amount")?;
            
        // API boundary: Solana returns u64 amounts, so we must convert for comparison
        let expected_amount_u64 = expected_amount.0.to_u64().ok_or("Expected amount too large for Solana (max ~18.4e18)")?;
        if actual_amount != expected_amount_u64 {
            return Err(format!(
                "Transaction amount mismatch. Expected: {}, Got: {}",
                expected_amount_u64, actual_amount
            ));
        }
    } else {
        return Err("Transaction metadata is missing".to_string());
    }
    
    Ok(())
}

/// Verify timestamp freshness for Solana payments (5 minute window)
pub fn verify_solana_timestamp_freshness(timestamp: Option<u64>) -> Result<(), String> {
    let current_time_ms = ICNetwork::get_time() / 1_000_000;
    let message_timestamp = timestamp.unwrap_or(current_time_ms);
    let age_ms = current_time_ms.saturating_sub(message_timestamp);
    
    if age_ms > 300_000 {  // 5 minutes in milliseconds
        return Err(format!("Solana payment signature timestamp too old: {} ms", age_ms));
    }
    
    Ok(())
}