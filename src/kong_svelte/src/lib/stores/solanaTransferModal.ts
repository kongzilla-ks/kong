import { writable } from 'svelte/store';

interface SolanaTransferModalState {
  show: boolean;
  payToken: Kong.Token | null;
  payAmount: string;
  receiveToken: Kong.Token | null;
  receiveAmount: string;
  maxSlippage: number;
  onConfirm: ((data: {
    transactionId: string;
    pay_signature: string;
    timestamp: bigint;
    canonicalMessage: string;
  }) => void) | null;
}

function createSolanaTransferModalStore() {
  const { subscribe, set, update } = writable<SolanaTransferModalState>({
    show: false,
    payToken: null,
    payAmount: '',
    receiveToken: null,
    receiveAmount: '',
    maxSlippage: 0.5,
    onConfirm: null,
  });

  return {
    subscribe,
    show: (params: {
      payToken: Kong.Token;
      payAmount: string;
      receiveToken: Kong.Token;
      receiveAmount: string;
      maxSlippage: number;
      onConfirm: (data: {
        transactionId: string;
        pay_signature: string;
        timestamp: bigint;
        canonicalMessage: string;
      }) => void;
    }) => {
      set({
        show: true,
        ...params,
      });
    },
    hide: () => {
      update(state => ({ ...state, show: false }));
    },
  };
}

export const solanaTransferModalStore = createSolanaTransferModalStore();