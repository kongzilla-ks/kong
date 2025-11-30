// Service to fetch Solana token metadata including logos from Metaplex
import { Connection, PublicKey } from '@solana/web3.js';
import { SOLANA_RPC_ENDPOINT } from '$lib/config/solana.config';

// Metaplex Token Metadata Program ID
const TOKEN_METADATA_PROGRAM_ID = new PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s');

// Cache for metadata to avoid repeated RPC calls
const metadataCache = new Map<string, { uri: string | null; image: string | null; name: string | null }>();

// Known token metadata for fast lookups (avoid RPC calls for common tokens)
const KNOWN_TOKEN_METADATA: Record<string, { name: string; image: string }> = {
  // Native SOL (not actually an SPL token, but we handle it)
  '11111111111111111111111111111111': {
    name: 'Solana',
    image: 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png'
  },
  // Wrapped SOL
  'So11111111111111111111111111111111111111112': {
    name: 'Wrapped SOL',
    image: 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png'
  },
  // USDC
  'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v': {
    name: 'USD Coin',
    image: 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png'
  },
  // USDT
  'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB': {
    name: 'USDT',
    image: 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB/logo.svg'
  },
};

/**
 * Derive the Metaplex metadata PDA for a given mint address
 */
function getMetadataPDA(mintAddress: PublicKey): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from('metadata'),
      TOKEN_METADATA_PROGRAM_ID.toBuffer(),
      mintAddress.toBuffer(),
    ],
    TOKEN_METADATA_PROGRAM_ID
  );
  return pda;
}

/**
 * Parse Metaplex metadata from account data
 * The structure follows the Metaplex Token Metadata standard
 */
function parseMetaplexMetadata(data: Buffer): { name: string; symbol: string; uri: string } | null {
  try {
    // Skip the first byte (key discriminator)
    let offset = 1;

    // Skip update authority (32 bytes)
    offset += 32;

    // Skip mint (32 bytes)
    offset += 32;

    // Read name (4 byte length + string)
    const nameLength = data.readUInt32LE(offset);
    offset += 4;
    const name = data.slice(offset, offset + nameLength).toString('utf8').replace(/\0/g, '').trim();
    offset += nameLength;

    // Read symbol (4 byte length + string)
    const symbolLength = data.readUInt32LE(offset);
    offset += 4;
    const symbol = data.slice(offset, offset + symbolLength).toString('utf8').replace(/\0/g, '').trim();
    offset += symbolLength;

    // Read uri (4 byte length + string)
    const uriLength = data.readUInt32LE(offset);
    offset += 4;
    const uri = data.slice(offset, offset + uriLength).toString('utf8').replace(/\0/g, '').trim();

    return { name, symbol, uri };
  } catch (error) {
    console.error('[SolanaMetadataService] Error parsing metadata:', error);
    return null;
  }
}

export class SolanaMetadataService {
  private static connection: Connection | null = null;

  private static getConnection(): Connection {
    if (!this.connection) {
      this.connection = new Connection(SOLANA_RPC_ENDPOINT, 'confirmed');
    }
    return this.connection;
  }

  /**
   * Fetch token metadata from Metaplex for a given mint address
   */
  static async fetchTokenMetadata(mintAddress: string): Promise<{ uri: string | null; image: string | null; name: string | null }> {
    // Check cache first
    if (metadataCache.has(mintAddress)) {
      return metadataCache.get(mintAddress)!;
    }

    // Check known tokens for fast lookup
    if (KNOWN_TOKEN_METADATA[mintAddress]) {
      const known = KNOWN_TOKEN_METADATA[mintAddress];
      const result = { uri: null, image: known.image, name: known.name };
      metadataCache.set(mintAddress, result);
      return result;
    }

    try {
      console.log('[SolanaMetadataService] Fetching metadata for:', mintAddress);

      const connection = this.getConnection();
      const mintPubkey = new PublicKey(mintAddress);
      const metadataPDA = getMetadataPDA(mintPubkey);

      // Fetch the metadata account
      const accountInfo = await connection.getAccountInfo(metadataPDA);

      if (!accountInfo) {
        console.log('[SolanaMetadataService] No metadata account found for:', mintAddress);
        const result = { uri: null, image: null, name: null };
        metadataCache.set(mintAddress, result);
        return result;
      }

      // Parse the metadata
      const metadata = parseMetaplexMetadata(Buffer.from(accountInfo.data));

      if (!metadata || !metadata.uri) {
        console.log('[SolanaMetadataService] Could not parse metadata for:', mintAddress);
        const result = { uri: null, image: null, name: metadata?.name || null };
        metadataCache.set(mintAddress, result);
        return result;
      }

      console.log('[SolanaMetadataService] Found metadata URI:', metadata.uri);

      // Fetch the JSON metadata from the URI to get the image
      let image: string | null = null;
      try {
        const response = await fetch(metadata.uri);
        if (response.ok) {
          const jsonMetadata = await response.json();
          image = jsonMetadata.image || null;
          console.log('[SolanaMetadataService] Fetched image URL:', image);
        }
      } catch (fetchError) {
        console.error('[SolanaMetadataService] Error fetching URI metadata:', fetchError);
      }

      const result = { uri: metadata.uri, image, name: metadata.name };
      metadataCache.set(mintAddress, result);
      return result;
    } catch (error) {
      console.error('[SolanaMetadataService] Error fetching metadata:', error);
      const result = { uri: null, image: null, name: null };
      metadataCache.set(mintAddress, result);
      return result;
    }
  }

  /**
   * Get logo URL for a Solana token by mint address
   */
  static async getTokenLogoUrl(mintAddress: string): Promise<string | null> {
    try {
      const metadata = await this.fetchTokenMetadata(mintAddress);
      return metadata.image;
    } catch (error) {
      console.error('[SolanaMetadataService] Error getting logo URL:', error);
      return null;
    }
  }

  /**
   * Clear the metadata cache (useful for testing or refreshing)
   */
  static clearCache(): void {
    metadataCache.clear();
  }
}
