use anyhow::Result;
use mpl_token_metadata::accounts::Metadata;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

use crate::config::TokensConfig;

pub struct SymbolCache {
    rpc_url: String,
    token_metadata_program_id: String,
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl SymbolCache {
    pub fn new(
        rpc_url: String,
        tokens_config: TokensConfig,
        token_metadata_program_id: String,
    ) -> Self {
        let cache = tokens_config.known_symbols.clone();

        Self {
            rpc_url,
            token_metadata_program_id,
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

    async fn fetch_metadata_symbol(&self, token_mint: &str) -> Result<String> {
        // Parse mint pubkey
        let mint_pubkey = Pubkey::from_str(token_mint)
            .map_err(|e| anyhow::anyhow!("Invalid mint pubkey {}: {}", token_mint, e))?;

        // Parse metadata program ID
        let metadata_program_id = Pubkey::from_str(&self.token_metadata_program_id)
            .map_err(|e| anyhow::anyhow!("Invalid metadata program ID: {}", e))?;

        // Derive metadata PDA
        let (metadata_pda, _bump) = Pubkey::find_program_address(
            &[
                b"metadata",
                metadata_program_id.as_ref(),
                mint_pubkey.as_ref(),
            ],
            &metadata_program_id,
        );

        debug!(
            "Fetching metadata for mint {} at PDA {}",
            token_mint, metadata_pda
        );

        // Fetch account data (blocking call, run in tokio blocking thread)
        let rpc_url = self.rpc_url.clone();
        let account_data = tokio::task::spawn_blocking(move || {
            let client = RpcClient::new(rpc_url);
            client.get_account_data(&metadata_pda)
        })
        .await
        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))??;

        // Deserialize metadata
        let metadata = Metadata::safe_deserialize(&account_data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize metadata: {}", e))?;

        // Extract symbol, trimming null bytes and whitespace
        let symbol = metadata.symbol.trim_end_matches('\0').trim().to_string();

        if symbol.is_empty() {
            anyhow::bail!("Empty symbol in metadata for {}", token_mint);
        }

        debug!("Fetched symbol '{}' for mint {}", symbol, token_mint);
        Ok(symbol)
    }
}
