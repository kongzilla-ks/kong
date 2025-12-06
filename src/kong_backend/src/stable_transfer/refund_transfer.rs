use candid::Nat;
use ic_cdk::update;
use ic_ledger_types::Timestamp;

use crate::{
    ic::{
        address::Address,
        network::ICNetwork,
        transfer::{icp_transfer, icrc1_transfer},
        verify_transfer::verify_transfer,
    },
    stable_token::{stable_token::StableToken, token::Token, token_map},
    stable_transfer::{stable_transfer::StableTransfer, transfer_map, tx_id::TxId},
};

// copied from kong_lib::send
async fn send(amount: &Nat, dst_address: &Address, token: &StableToken, created_at_time: Option<u64>) -> Result<Nat, String> {
    match dst_address {
        Address::AccountId(account_identifier) => {
            let timestamp = Timestamp {
                timestamp_nanos: created_at_time.unwrap_or(0),
            };
            let timetamp_ref = if timestamp.timestamp_nanos == 0 { None } else { Some(&timestamp) };

            icp_transfer(amount, account_identifier, token, timetamp_ref)
                .await
                .map_err(|e| e.to_string())
        }
        Address::PrincipalId(account) => icrc1_transfer(amount, account, token, created_at_time)
            .await
            .map_err(|e| e.to_string()),

        Address::SolanaAddress(_) => Err("Solana is not implemented".to_string()),
    }
}

#[update]
pub async fn refund_transfer(token: String, amount: Nat, tx_id: TxId) -> Result<(), String> {
    // TODO: only kong limit can do this call?
    let token = token_map::get_by_token(&token)?;

    let block_index = match &tx_id {
        TxId::BlockIndex(nat) => nat,
        TxId::TransactionId(_) => return Err("Sol Unsupported".to_string()),
    };

    let refunded_transfer_id = {
        ic_cdk::println!("kong_backend: refund_transfer, amount={}", amount);
        match transfer_map::get_transfer_tx_id(token.token_id(), &tx_id) {
            Some(v) => v,
            None => match verify_transfer(&token, &block_index, &amount).await {
                Ok(_) => transfer_map::insert(&StableTransfer {
                    transfer_id: 0,
                    request_id: 0,
                    is_send: true,
                    amount,
                    token_id: token.token_id(),
                    tx_id,
                    ts: ICNetwork::get_time(),
                    refund_transfer_id: None,
                }),
                Err(e) => return Err(e),
            },
        }
    };

    // Returns if already refunded or refunding
    let _ = transfer_map::stable_trasnfer_set_refunding(refunded_transfer_id, Some(0))?;

    let refunded_stable_transfer = transfer_map::get_by_transfer_id(refunded_transfer_id).expect("Can't find inserted stable transfer");

    let address = Address::PrincipalId(ICNetwork::caller_id());

    let amount_to_transfer = refunded_stable_transfer.amount.clone() - token.fee();

    if amount_to_transfer == Nat::default() {
        let _ = transfer_map::stable_trasnfer_set_refunding(refunded_transfer_id, None);
        return Err("Amount transfer is too low".to_string());
    }

    match send(&amount_to_transfer, &address, &token, Some(refunded_stable_transfer.ts)).await {
        Ok(tx_id) => {
            let transfer_id = transfer_map::insert(&StableTransfer {
                transfer_id: 0,
                request_id: 0,
                is_send: false,
                token_id: token.token_id(),
                amount: amount_to_transfer.clone(),
                tx_id: TxId::BlockIndex(tx_id),
                ts: ICNetwork::get_time(),
                refund_transfer_id: None,
            });

            let _ = transfer_map::stable_trasnfer_set_refunding(refunded_transfer_id, None)?;
            let _ = transfer_map::stable_trasnfer_set_refunding(refunded_transfer_id, Some(transfer_id))?;
            Ok(())
        }
        Err(e) => {
            let _ = transfer_map::stable_trasnfer_set_refunding(refunded_transfer_id, None);
            Err(e)
        }
    }
}
