use bs58;
use bs58::Alphabet;

/// Encode bytes as base58 string to match Ripple's alphabet
pub fn encode_wallet_address(bytes: &[u8]) -> String {
    bs58::encode(bytes).with_alphabet(Alphabet::RIPPLE).into_string()
}
