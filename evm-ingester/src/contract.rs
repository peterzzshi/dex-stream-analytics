use crate::types::{PoolAddress, TokenAddress, TokenMetadata};
use anyhow::{Context, Result};
use ethers::abi::{Abi, Token};
use ethers::prelude::*;
use ethers::types::{Address, Bytes};
use std::sync::Arc;

/// Minimal Uniswap V2 Pair ABI for fetching metadata
const PAIR_ABI: &str = r#"[
    {
        "constant": true,
        "inputs": [],
        "name": "token0",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [],
        "name": "token1",
        "outputs": [{"name": "", "type": "address"}],
        "type": "function"
    }
]"#;

/// Minimal ERC20 ABI for fetching token metadata
const ERC20_ABI: &str = r#"[
    {
        "constant": true,
        "inputs": [],
        "name": "symbol",
        "outputs": [{"name": "", "type": "string"}],
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [],
        "name": "decimals",
        "outputs": [{"name": "", "type": "uint8"}],
        "type": "function"
    }
]"#;

/// Pair contract information
#[derive(Debug, Clone)]
pub struct PairInfo {
    pub token0: TokenAddress,
    pub token1: TokenAddress,
}

/// Contract interface for fetching on-chain metadata
pub struct ContractInterface<M> {
    provider: Arc<M>,
    pair_abi: Abi,
    erc20_abi: Abi,
}

impl<M> ContractInterface<M>
where
    M: Middleware,
    M::Error: 'static,
{
    pub fn new(provider: Arc<M>) -> Result<Self> {
        let pair_abi: Abi = serde_json::from_str(PAIR_ABI).context("invalid pair ABI")?;
        let erc20_abi: Abi = serde_json::from_str(ERC20_ABI).context("invalid ERC20 ABI")?;

        Ok(Self {
            provider,
            pair_abi,
            erc20_abi,
        })
    }

    /// Fetch token0 and token1 addresses from a Uniswap V2 pair contract
    pub async fn fetch_pair_tokens(&self, pair_address: PoolAddress) -> Result<PairInfo> {
        let token0 = self.call_pair_token0(pair_address.inner()).await?;
        let token1 = self.call_pair_token1(pair_address.inner()).await?;

        Ok(PairInfo {
            token0: TokenAddress::new(token0),
            token1: TokenAddress::new(token1),
        })
    }

    async fn call_pair_token0(&self, pair_address: Address) -> Result<Address> {
        let function = self
            .pair_abi
            .function("token0")
            .context("token0 function not found in ABI")?;

        let call_data = function
            .encode_input(&[])
            .context("failed to encode input")?;

        let tx = TransactionRequest::new().to(pair_address).data(call_data);

        let result: Bytes = self
            .provider
            .call(&tx.into(), None)
            .await
            .context("token0 call failed")?;

        let decoded = function
            .decode_output(&result)
            .context("failed to decode token0 output")?;

        if let Some(Token::Address(addr)) = decoded.first() {
            Ok(*addr)
        } else {
            anyhow::bail!("unexpected token0 output format");
        }
    }

    async fn call_pair_token1(&self, pair_address: Address) -> Result<Address> {
        let function = self
            .pair_abi
            .function("token1")
            .context("token1 function not found in ABI")?;

        let call_data = function
            .encode_input(&[])
            .context("failed to encode input")?;

        let tx = TransactionRequest::new().to(pair_address).data(call_data);

        let result: Bytes = self
            .provider
            .call(&tx.into(), None)
            .await
            .context("token1 call failed")?;

        let decoded = function
            .decode_output(&result)
            .context("failed to decode token1 output")?;

        if let Some(Token::Address(addr)) = decoded.first() {
            Ok(*addr)
        } else {
            anyhow::bail!("unexpected token1 output format");
        }
    }

    /// Fetch token metadata (symbol and decimals) from an ERC20 contract
    pub async fn fetch_token_metadata(&self, token_address: TokenAddress) -> Result<TokenMetadata> {
        let symbol = self.fetch_token_symbol(token_address.inner()).await.ok();
        let decimals = self
            .fetch_token_decimals(token_address.inner())
            .await
            .unwrap_or(18); // Default to 18 decimals if call fails

        let mut metadata = TokenMetadata::new(token_address, decimals);
        if let Some(s) = symbol {
            metadata = metadata.with_symbol(s);
        }

        Ok(metadata)
    }

    async fn fetch_token_symbol(&self, token_address: Address) -> Result<String> {
        let function = self
            .erc20_abi
            .function("symbol")
            .context("symbol function not found in ABI")?;

        let call_data = function
            .encode_input(&[])
            .context("failed to encode input")?;

        let tx = TransactionRequest::new().to(token_address).data(call_data);

        let result: Bytes = self
            .provider
            .call(&tx.into(), None)
            .await
            .context("symbol call failed")?;

        let decoded = function
            .decode_output(&result)
            .context("failed to decode symbol output")?;

        if let Some(Token::String(symbol)) = decoded.first() {
            Ok(symbol.clone())
        } else {
            anyhow::bail!("unexpected symbol output format");
        }
    }

    async fn fetch_token_decimals(&self, token_address: Address) -> Result<u8> {
        let function = self
            .erc20_abi
            .function("decimals")
            .context("decimals function not found in ABI")?;

        let call_data = function
            .encode_input(&[])
            .context("failed to encode input")?;

        let tx = TransactionRequest::new().to(token_address).data(call_data);

        let result: Bytes = self
            .provider
            .call(&tx.into(), None)
            .await
            .context("decimals call failed")?;

        let decoded = function
            .decode_output(&result)
            .context("failed to decode decimals output")?;

        if let Some(Token::Uint(decimals)) = decoded.first() {
            Ok(decimals.as_u32() as u8)
        } else {
            anyhow::bail!("unexpected decimals output format");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi_parsing() {
        let pair_abi: Result<Abi, _> = serde_json::from_str(PAIR_ABI);
        assert!(pair_abi.is_ok());

        let erc20_abi: Result<Abi, _> = serde_json::from_str(ERC20_ABI);
        assert!(erc20_abi.is_ok());
    }
}
