import { writable } from 'svelte/store';

interface SolanaLiquidityModalData {
  operation: 'add' | 'remove';
  token0: Kong.Token;
  amount0: string;
  token1: Kong.Token;
  amount1: string;
  lpAmount: string; // for remove liquidity
  onConfirm: (data: {
    solTransactionId?: string;
    icrcTransactionId?: bigint;
    signature: string;
    timestamp: bigint;
    canonicalMessage: string;
  }) => void;
  onCancel?: () => void; // Add cancellation callback
}

interface SolanaLiquidityModalStore {
  isVisible: boolean;
  data: SolanaLiquidityModalData | null;
}

function createSolanaLiquidityModalStore() {
  const { subscribe, set, update } = writable<SolanaLiquidityModalStore>({
    isVisible: false,
    data: null,
  });

  return {
    subscribe,
    show: (data: SolanaLiquidityModalData) => {
      set({
        isVisible: true,
        data,
      });
    },
    hide: () => {
      update(store => {
        // Call onCancel if the modal is being hidden and we have data
        if (store.data?.onCancel) {
          store.data.onCancel();
        }
        return {
          isVisible: false,
          data: null,
        };
      });
    },
    handleConfirm: (confirmData: {
      transactionId?: string;
      signature: string;
      timestamp: bigint;
      canonicalMessage: string;
    }) => {
      update(store => {
        if (store.data?.onConfirm) {
          store.data.onConfirm(confirmData);
        }
        return {
          isVisible: false,
          data: null,
        };
      });
    },
  };
}

export const solanaLiquidityModalStore = createSolanaLiquidityModalStore();