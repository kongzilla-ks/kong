// Service to fetch ICRC token metadata including logos
import { IcrcLedgerCanister } from '@dfinity/ledger-icrc';
import { createAgent } from '@dfinity/utils';
import { Principal } from '@dfinity/principal';

export class IcrcMetadataService {
  private static metadataCache = new Map<string, any>();
  
  /**
   * Fetch metadata for an ICRC token
   */
  static async fetchTokenMetadata(canisterId: string): Promise<Record<string, any>> {
    // Check cache first
    if (this.metadataCache.has(canisterId)) {
      return this.metadataCache.get(canisterId)!;
    }
    
    try {
      console.log('[IcrcMetadataService] Fetching metadata for:', canisterId);
      
      // Create anonymous agent
      const agent = await createAgent({
        host: process.env.DFX_NETWORK === 'local' ? 'http://127.0.0.1:8000' : 'https://icp0.io',
        fetchRootKey: process.env.DFX_NETWORK === 'local',
      });
      
      // Create ICRC ledger canister instance
      const { metadata: getMetadata } = IcrcLedgerCanister.create({
        agent,
        canisterId,
      });
      
      // Fetch metadata
      const metadataArray = await getMetadata({});
      
      // Convert array of tuples to object
      const metadata: Record<string, any> = {};
      for (const [key, value] of metadataArray) {
        if ('Text' in value) {
          metadata[key] = value.Text;
        } else if ('Nat' in value) {
          metadata[key] = value.Nat.toString();
        } else if ('Int' in value) {
          metadata[key] = value.Int.toString();
        } else if ('Blob' in value) {
          metadata[key] = value.Blob;
        }
      }
      
      console.log('[IcrcMetadataService] Metadata for', canisterId, ':', metadata);
      
      // Cache the result
      this.metadataCache.set(canisterId, metadata);
      
      return metadata;
    } catch (error) {
      console.error('[IcrcMetadataService] Error fetching metadata:', error);
      return {};
    }
  }
  
  /**
   * Extract logo from metadata
   */
  static extractLogoFromMetadata(metadata: Record<string, any>): string | null {
    // Look for logo in various possible keys
    const logoKeys = ['icrc1:logo', 'logo'];
    
    for (const key of logoKeys) {
      if (metadata[key]) {
        const logoData = metadata[key];
        console.log('[IcrcMetadataService] Found logo data for key:', key, 'type:', typeof logoData);
        
        // If it's already a data URL, return it
        if (typeof logoData === 'string' && logoData.startsWith('data:')) {
          return logoData;
        }
        
        // If it's a string, assume it's base64 (ICRC standard stores logos as text)
        if (typeof logoData === 'string' && logoData.length > 0) {
          // Clean up any whitespace
          const cleanBase64 = logoData.trim();
          
          // Most ICRC logos are PNG format
          return `data:image/png;base64,${cleanBase64}`;
        }
        
        // If it's a Uint8Array or number array (blob), convert to base64
        if (logoData instanceof Uint8Array || Array.isArray(logoData)) {
          try {
            const bytes = logoData instanceof Uint8Array ? logoData : new Uint8Array(logoData);
            const base64 = btoa(String.fromCharCode.apply(null, Array.from(bytes)));
            return `data:image/png;base64,${base64}`;
          } catch (e) {
            console.error('[IcrcMetadataService] Error converting blob to base64:', e);
          }
        }
      }
    }
    
    return null;
  }
  
  /**
   * Get logo URL for a token
   */
  static async getTokenLogoUrl(canisterId: string): Promise<string | null> {
    try {
      const metadata = await this.fetchTokenMetadata(canisterId);
      return this.extractLogoFromMetadata(metadata);
    } catch (error) {
      console.error('[IcrcMetadataService] Error getting logo URL:', error);
      return null;
    }
  }
}