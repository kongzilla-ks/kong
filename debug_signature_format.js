// Debug script to show the exact message format differences

// Frontend format (from CrossChainSwapService.ts)
const frontendMessage = {
  pay_token: "SOL",
  pay_amount: "1000000",  // Nat serializes to string
  pay_address: "HXtBm8XZbxaTt41uqaKhwUAa6Z1aPyvJdsZVENiWsetg",
  receive_token: "ksUSDT", 
  receive_amount: "1000000",  // Nat serializes to string
  receive_address: "principal_address_here",
  max_slippage: 1.0,
  timestamp: 1735000000000,  // u64 serializes to number
  referred_by: null  // Option<String> serializes to null
};

console.log("Frontend JSON message:");
console.log(JSON.stringify(frontendMessage));
console.log("\nFrontend message bytes (UTF-8):");
const frontendBytes = new TextEncoder().encode(JSON.stringify(frontendMessage));
console.log(Array.from(frontendBytes));
console.log("Length:", frontendBytes.length);

// What backend expects (based on backend code)
// The backend uses serde_json::to_string which should produce the same JSON
console.log("\n\nExpected backend format should be identical since both use JSON serialization");

// Solana offchain message format
const SOLANA_OFFCHAIN_PREFIX = [0xff, ...new TextEncoder().encode("solana offchain")];
console.log("\n\nSolana offchain message prefix bytes:");
console.log(SOLANA_OFFCHAIN_PREFIX);

// Full offchain message structure
console.log("\nFull offchain message structure:");
console.log("1. Prefix: \\xffsolana offchain (17 bytes)");
console.log("2. Version: 0 (1 byte)"); 
console.log("3. Format: 0 for RestrictedAscii, 1 for LimitedUtf8, 2 for ExtendedUtf8 (1 byte)");
console.log("4. Message length: little-endian u16 (2 bytes)");
console.log("5. Message content: UTF-8 encoded JSON");