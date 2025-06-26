import { formatBalance, formatToNonZeroDecimal } from "$lib/utils/numberFormatUtils";
import { IcrcService } from "$lib/services/icrc/IcrcService";
import { Principal } from "@dfinity/principal";
import { get } from 'svelte/store';
import { auth } from '$lib/stores/auth';
import { pnp } from '$lib/config/auth.config';

// Constants
const BATCH_SIZE = 40;
const BATCH_DELAY_MS = 100;
const DEFAULT_PRICE = 0;

// Helper functions
function convertPrincipalId(principalId: string | Principal): Principal {
  return typeof principalId === "string" ? Principal.fromText(principalId) : principalId;
}

// Fetch Solana balance for a specific token
async function fetchSolanaBalance(token: Kong.Token): Promise<TokenBalance> {
  const authData = get(auth);
  
  // Check if user is connected and provider is available
  if (!authData.isConnected || !pnp.provider) {
    console.warn('Solana provider not available or user not connected');
    return {
      in_tokens: BigInt(0),
      in_usd: formatToNonZeroDecimal(0),
    };
  }

  try {
    // For native SOL
    if (token.symbol === 'SOL') {
      const solBalance = await pnp.provider.getSolBalance?.();
      if (solBalance && typeof solBalance.amount === 'number') {
        const balanceInLamports = BigInt(Math.floor(solBalance.amount * Math.pow(10, token.decimals)));
        return formatTokenBalance(balanceInLamports, token.decimals, token?.metrics?.price ?? DEFAULT_PRICE);
      }
    } else {
      // For SPL tokens
      const splBalances = await pnp.provider.getSplTokenBalances?.();
      if (Array.isArray(splBalances)) {
        const tokenBalance = splBalances.find(t => t.mint === token.address);
        if (tokenBalance) {
          const balanceInSmallestUnit = BigInt(tokenBalance.amount || 0);
          return formatTokenBalance(balanceInSmallestUnit, token.decimals, token?.metrics?.price ?? DEFAULT_PRICE);
        }
      }
    }
  } catch (error) {
    console.error(`Error fetching Solana balance for ${token.symbol}:`, error);
  }

  return {
    in_tokens: BigInt(0),
    in_usd: formatToNonZeroDecimal(0),
  };
}

function calculateUsdValue(balance: string, price: number | string = DEFAULT_PRICE): number {
  const tokenAmount = parseFloat(balance.replace(/,/g, ''));
  return tokenAmount * Number(price);
}

function formatTokenBalance(balance: bigint | { default: bigint }, decimals: number, price: number | string): TokenBalance {
  const finalBalance = typeof balance === "object" ? balance.default : balance;
  const actualBalance = formatBalance(finalBalance.toString(), decimals)?.replace(/,/g, '');
  const usdValue = calculateUsdValue(actualBalance, price);

  return {
    in_tokens: finalBalance,
    in_usd: usdValue.toString(),
  };
}

// Main functions
export async function fetchBalance(
  token: Kong.Token,
  principalId?: string,
  forceRefresh = false,
): Promise<TokenBalance> {
  try {
    if (!token?.address || !principalId) {
      return {
        in_tokens: BigInt(0),
        in_usd: formatToNonZeroDecimal(0),
      };
    }

    // Handle Solana tokens differently
    if (token.chain === 'Solana') {
      return fetchSolanaBalance(token);
    }

    const principal = convertPrincipalId(principalId);
    const balance = await IcrcService.getIcrc1Balance(token, principal);
    return formatTokenBalance(balance, token.decimals, token?.metrics?.price ?? DEFAULT_PRICE);
  } catch (error) {
    console.error(`Error fetching balance for token ${token.address}:`, error);
    return {
      in_tokens: BigInt(0),
      in_usd: formatToNonZeroDecimal(0),
    };
  }
}

async function processBatch(
  batch: Kong.Token[],
  principal: string,
): Promise<Map<string, bigint>> {
  try {
    const batchBalances = await IcrcService.batchGetBalances(batch, principal);
    return new Map(
      Array.from(batchBalances.entries())
        .filter(([_, balance]) => balance !== undefined && balance !== null)
    );
  } catch (error) {
    console.error(`Error processing batch:`, error);
    return new Map();
  }
}

export async function fetchBalances(
  tokens?: Kong.Token[],
  principalId?: string,
  forceRefresh = false,
): Promise<Record<string, TokenBalance>> {
  if (!principalId || !tokens?.length) {
    return {};
  }

  try {
    const principal = principalId;
    const results = new Map<string, bigint>();

    // Filter out Solana tokens and null/undefined tokens
    const icTokens = tokens.filter(token => token && token.chain !== 'Solana');
    
    // Process only IC tokens in batches
    for (let i = 0; i < icTokens.length; i += BATCH_SIZE) {
      const batch = icTokens
        .slice(i, i + BATCH_SIZE)
        .map(t => ({ ...t, timestamp: Date.now() }));

      const batchResults = await processBatch(batch, principal);
      for (const [canisterId, balance] of batchResults) {
        results.set(canisterId, balance);
      }

      // Add delay between batches if not the last batch
      if (i + BATCH_SIZE < icTokens.length) {
        await new Promise(resolve => setTimeout(resolve, BATCH_DELAY_MS));
      }
    }

    // Process IC token results first
    const balanceResults = tokens.reduce((acc, token) => {
      // Skip null/undefined tokens
      if (!token) {
        return acc;
      }
      
      // Skip Solana tokens for now
      if (token.chain === 'Solana') {
        return acc;
      }
      
      const balance = results.get(token.address);
      if (balance !== undefined) {
        const tokenBalance = formatTokenBalance(
          balance,
          token.decimals,
          token?.metrics?.price ?? DEFAULT_PRICE
        );
        acc[token.address] = tokenBalance;
      }
      return acc;
    }, {} as Record<string, TokenBalance>);

    // Now fetch Solana token balances
    const solanaTokens = tokens.filter(token => token && token.chain === 'Solana');
    if (solanaTokens.length > 0) {
      const solanaBalances = await Promise.all(
        solanaTokens.map(async (token) => {
          const balance = await fetchSolanaBalance(token);
          return { address: token.address, balance };
        })
      );
      
      // Add Solana balances to results
      solanaBalances.forEach(({ address, balance }) => {
        balanceResults[address] = balance;
      });
    }

    return balanceResults;

  } catch (error) {
    console.error('Error in fetchBalances:', error);
    return {};
  }
}