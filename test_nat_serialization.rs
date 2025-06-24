use candid::Nat;
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Debug, Serialize, Deserialize)]
struct TestMessage {
    amount: Nat,
}

fn main() {
    let msg = TestMessage {
        amount: Nat::from(1000000u64),
    };
    
    let json = serde_json::to_string(&msg).unwrap();
    println!("Serialized: {}", json);
    
    // Test with custom serializer that converts to u64
    let amount_u64 = msg.amount.0.to_u64().unwrap();
    println!("As u64: {}", amount_u64);
}