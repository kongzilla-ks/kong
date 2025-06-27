// Solana Network Configuration
export const SOLANA_RPC_ENDPOINT = 'https://mainnet.solana.validationcloud.io/v1/vEH0znkrmOFeAxXCJAyIGLQFSPVY38NzDe2NkNvNQt4';
export const SOLANA_WS_ENDPOINT = 'wss://mainnet.solana.validationcloud.io/v1/vEH0znkrmOFeAxXCJAyIGLQFSPVY38NzDe2NkNvNQt4';

// WebSocket configuration
export const WS_CONFIG = {
  reconnectDelay: 1000, // Start with 1 second
  maxReconnectDelay: 30000, // Max 30 seconds
  reconnectBackoffMultiplier: 1.5,
  maxReconnectAttempts: 10,
  heartbeatInterval: 30000, // 30 seconds
  subscriptionTimeout: 5000, // 5 seconds to establish subscription
};

// Commitment levels for different operations
export const COMMITMENT_CONFIG = {
  balance: 'confirmed' as const,
  transaction: 'finalized' as const,
};