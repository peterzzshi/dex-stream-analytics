use anyhow::{Context, Result};
use solana_client::rpc_client::RpcClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

// Hardcoded symbols for common tokens
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

pub struct SymbolCache {
    rpc_url: String,
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl SymbolCache {
    pub fn new(rpc_url: String) -> Self {
        let mut cache = HashMap::new();

        // Pre-populate hardcoded symbols
        cache.insert(SOL_MINT.to_string(), "SOL".to_string());
        cache.insert(USDC_MINT.to_string(), "USDC".to_string());

        Self {
            rpc_url,
            cache: Arc::new(RwLock::new(cache)),
        }
    }

    /// Get token symbol, fetching from SPL Token Metadata if not cached
    pub async fn get_symbol(&self, token_mint: &str) -> Option<String> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(symbol) = cache.get(token_mint) {
                return Some(symbol.clone());
            }
        }

        // Fetch from SPL Token Metadata
        match self.fetch_metadata_symbol(token_mint).await {
            Ok(symbol) => {
                // Update cache
                let mut cache = self.cache.write().await;
                cache.insert(token_mint.to_string(), symbol.clone());
                Some(symbol)
            }
            Err(e) => {
                debug!("Failed to fetch symbol for {}: {}", token_mint, e);
                None
            }
        }
    }

    async fn fetch_metadata_symbol(&self, _token_mint: &str) -> Result<String> {
        // TODO: Implement SPL Token Metadata fetching
        //
        // Implementation notes:
        // 1. Derive metadata PDA: Pubkey::find_program_address(
        //    &[b"metadata", TOKEN_METADATA_PROGRAM_ID.as_ref(), mint.as_ref()],
        //    &TOKEN_METADATA_PROGRAM_ID
        // )
        // 2. Fetch account data via RpcClient
        // 3. Deserialize using mpl-token-metadata crate (Metadata struct)
        // 4. Return metadata.data.symbol
        //
        // For now, return error to preserve type safety

        anyhow::bail!("SPL Token Metadata fetching not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hardcoded_symbols() {
        let cache = SymbolCache::new("https://api.mainnet-beta.solana.com".to_string());

        let sol_symbol = cache.get_symbol(SOL_MINT).await;
        assert_eq!(sol_symbol, Some("SOL".to_string()));

        let usdc_symbol = cache.get_symbol(USDC_MINT).await;
        assert_eq!(usdc_symbol, Some("USDC".to_string()));
    }
}
