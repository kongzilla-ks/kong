import { canisters, type CanisterType } from "$lib/config/auth.config";
import { IcrcService } from "$lib/services/icrc/IcrcService";
import { auth } from "$lib/stores/auth";
import { Principal } from "@dfinity/principal";
import { notificationsStore } from "$lib/stores/notificationsStore";

export async function getMarket(marketId: bigint) {
  const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
    canisterId: canisters.predictionMarkets.canisterId,
    idl: canisters.predictionMarkets.idl,
    anon: true,
  });
  const market = await actor.get_market(marketId);
  return market;
}

export async function getAllMarkets(
  options: {
    start?: number;
    length?: number;
    statusFilter?: "Open" | "Closed" | "Disputed" | "Voided";
    sortOption?: {
      type: "CreatedAt" | "TotalPool";
      direction: "Ascending" | "Descending";
    };
  } = {},
) {
  const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
    canisterId: canisters.predictionMarkets.canisterId,
    idl: canisters.predictionMarkets.idl,
    anon: true,
  });

  const args = {
    start: BigInt(options.start ?? 0),
    length: BigInt(options.length ?? 100),
    status_filter: options.statusFilter
      ? options.statusFilter === "Closed"
        ? [{ Closed: [] }] as [any]
        : options.statusFilter === "Open"
          ? [{ Open: null }] as [any]
          : options.statusFilter === "Voided"
            ? [{ Voided: null }] as [any]
            : [] as []
      : [] as [],
    sort_option: options.sortOption
      ? [{
          [options.sortOption.type]: { [options.sortOption.direction]: null },
        }] as [any] // Ensure it's a tuple with exactly one element
      : [] as [], // Default sorting (newest first) will be applied by the backend
  };

  const markets = await actor.get_all_markets(args);
  return markets;
}

export async function getMarketsByStatus() {
  const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
    canisterId: canisters.predictionMarkets.canisterId,
    idl: canisters.predictionMarkets.idl,
    anon: true,
  });
  const marketsByStatus = await actor.get_markets_by_status({
    start: 0n,
    length: 100n,
  });
  return marketsByStatus;
}

export async function getMarketBets(marketId: bigint) {
  const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
    canisterId: canisters.predictionMarkets.canisterId,
    idl: canisters.predictionMarkets.idl,
    anon: true,
  });
  const bets = await actor.get_market_bets(marketId);
  return bets;
}

export async function getUserHistory(principal: string) {
  const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
    canisterId: canisters.predictionMarkets.canisterId,
    idl: canisters.predictionMarkets.idl,
    anon: true,
  });
  try {
    const principalObj = Principal.fromText(principal);
    const history = await actor.get_user_history(principalObj);
    console.log("User history:", history);
    return history;
  } catch (error) {
    console.error("Error in getUserHistory:", error);
    throw error;
  }
}

export interface CreateMarketParams {
  question: string;
  category: any; // MarketCategory type from candid
  rules: string;
  outcomes: string[];
  resolutionMethod: any; // ResolutionMethod type from candid
  endTimeSpec: any; // MarketEndTime type from candid
  image_url?: string; // Optional image URL
}

export async function createMarket(params: CreateMarketParams) {
  const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
    canisterId: canisters.predictionMarkets.canisterId,
    idl: canisters.predictionMarkets.idl,
    anon: false,
    requiresSigning: false,
  });
  const result = await actor.create_market(
    params.question,
    params.category,
    params.rules,
    params.outcomes,
    params.resolutionMethod,
    params.endTimeSpec,
    params.image_url ? [params.image_url] : [], // Pass as optional array
  );

  notificationsStore.add({
    title: "Market Created",
    message: `Market "${params.question}" has been created`,
    type: "success",
  });
  return result;
}

export async function placeBet(
  token: Kong.Token,
  marketId: bigint,
  outcomeIndex: bigint,
  amount: string,
) {
  try {
    // Request a large allowance (100x the bet amount) to allow for multiple bets
    const largeAllowance = BigInt(token.metrics.total_supply);

    // Check and request allowance if needed
    await IcrcService.checkAndRequestIcrc2Allowances(
      token,
      largeAllowance,
      canisters.predictionMarkets.canisterId,
    );

    // Place the bet using an authenticated actor
    const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
      canisterId: canisters.predictionMarkets.canisterId,
      idl: canisters.predictionMarkets.idl,
      anon: false,
      requiresSigning: false,
    });

    // Convert amount string to BigInt and verify it's not zero
    const bigIntAmount = BigInt(amount);

    if (bigIntAmount === 0n) {
      throw new Error("Bet amount cannot be zero");
    }

    const result = await actor.place_bet(marketId, outcomeIndex, bigIntAmount);

    if ("Err" in result) {
      // Handle specific error cases
      if ("TransferError" in result.Err) {
        throw new Error(`Transfer failed: ${result.Err.TransferError}`);
      }

      if ("MarketNotFound" in result.Err) {
        throw new Error("Market not found");
      } else if ("MarketClosed" in result.Err) {
        throw new Error("Market is closed");
      } else if ("InvalidOutcome" in result.Err) {
        throw new Error("Invalid outcome selected");
      } else if ("InsufficientBalance" in result.Err) {
        throw new Error("Insufficient KONG balance");
      } else {
        throw new Error(`Bet failed: ${JSON.stringify(result.Err)}`);
      }
    }

    notificationsStore.add({
      title: "Bet Placed",
      message: `Bet placed successfully on market ${marketId}`,
      type: "success",
    });
    return result;
  } catch (error) {
    console.error("Place bet error:", error);
    if (error instanceof Error) {
      throw error;
    }
    throw new Error(`Failed to place bet: ${JSON.stringify(error)}`);
  }
}

export async function resolveMarketViaAdmin(
  marketId: bigint,
  winningOutcome: bigint,
): Promise<void> {
  try {
    const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
      canisterId: canisters.predictionMarkets.canisterId,
      idl: canisters.predictionMarkets.idl,
      anon: false,
      requiresSigning: false,
    });

    // Convert marketId from string to number
    const marketIdNumber = marketId;
    const result = await actor.resolve_via_admin(marketIdNumber, [
      winningOutcome,
    ]);

    if ("Err" in result) {
      if ("MarketNotFound" in result.Err) {
        throw new Error("Market not found");
      } else if ("MarketStillOpen" in result.Err) {
        throw new Error("Market is still open");
      } else if ("AlreadyResolved" in result.Err) {
        throw new Error("Market has already been resolved");
      } else if ("Unauthorized" in result.Err) {
        throw new Error("You are not authorized to resolve this market");
      } else {
        throw new Error(
          `Failed to resolve market: ${JSON.stringify(result.Err)}`,
        );
      }
    }
    notificationsStore.add({
      title: "Market Resolved",
      message: `Market ${marketId} has been resolved`,
      type: "success",
    });
  } catch (error) {
    console.error("Failed to resolve market via admin:", error);
    throw error;
  }
}

export async function voidMarketViaAdmin(marketId: bigint): Promise<void> {
  try {
    const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
      canisterId: canisters.predictionMarkets.canisterId,
      idl: canisters.predictionMarkets.idl,
      anon: false,
      requiresSigning: false,
    });

    // Convert marketId from string to number
    const marketIdNumber = marketId;
    const result = await actor.void_market(marketIdNumber);

    if ("Err" in result) {
      if ("MarketNotFound" in result.Err) {
        throw new Error("Market not found");
      } else if ("MarketStillOpen" in result.Err) {
        throw new Error("Market is still open");
      } else if ("AlreadyResolved" in result.Err) {
        throw new Error("Market has already been resolved");
      } else if ("Unauthorized" in result.Err) {
        throw new Error("You are not authorized to void this market");
      } else if ("VoidingFailed" in result.Err) {
        throw new Error("Failed to void the market");
      } else {
        throw new Error(
          `Failed to void market: ${JSON.stringify(result.Err)}`,
        );
      }
    }
    notificationsStore.add({
      title: "Market Voided",
      message: `Market ${marketId} has been voided`,
      type: "success",
    });
  } catch (error) {
    console.error("Failed to void market via admin:", error);
    throw error;
  }
}

export async function getAllBets(fromIndex: number = 0, toIndex: number = 10) {
  const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
    canisterId: canisters.predictionMarkets.canisterId,
    idl: canisters.predictionMarkets.idl,
    anon: true,
  });

  // The backend API expects start and length parameters
  const marketsByStatus = await actor.get_markets_by_status({
    start: 0n,
    length: 100n,
  });

  // Combine bets from all markets
  const allBets: any[] = [];

  // Process active markets
  if (
    marketsByStatus.markets_by_status.active &&
    marketsByStatus.markets_by_status.active.length > 0
  ) {
    for (const market of marketsByStatus.markets_by_status.active) {
      try {
        const marketBets = await actor.get_market_bets(market.id);
        allBets.push(...marketBets);
      } catch (e) {
        console.error(`Failed to get bets for market ${market.id}:`, e);
      }
    }
  }

  // Process expired unresolved markets
  if (
    marketsByStatus.markets_by_status.expired_unresolved &&
    marketsByStatus.markets_by_status.expired_unresolved.length > 0
  ) {
    for (const market of marketsByStatus.markets_by_status.expired_unresolved) {
      try {
        const marketBets = await actor.get_market_bets(market.id);
        allBets.push(...marketBets);
      } catch (e) {
        console.error(`Failed to get bets for market ${market.id}:`, e);
      }
    }
  }

  // Sort bets by timestamp (newest first)
  allBets.sort((a, b) => Number(b.timestamp) - Number(a.timestamp));

  // Return the requested slice
  return allBets.slice(fromIndex, toIndex);
}

export async function getPredictionMarketStats() {
  const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
    canisterId: canisters.predictionMarkets.canisterId,
    idl: canisters.predictionMarkets.idl,
    anon: true,
  });
  const stats = await actor.get_stats();
  return stats;
}

export async function getAllCategories() {
  const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
    canisterId: canisters.predictionMarkets.canisterId,
    idl: canisters.predictionMarkets.idl,
    anon: true,
  });
  const categories = await actor.get_all_categories();
  return categories;
}

export async function isAdmin(principal: string) {
  const actor = auth.pnp.getActor<CanisterType['PREDICTION_MARKETS']>({
    canisterId: canisters.predictionMarkets.canisterId,
    idl: canisters.predictionMarkets.idl,
    anon: true,
  });
  return await actor.is_admin(Principal.fromText(principal));
}
