use ethers::types::{Address, U256};
use std::fmt;

/// Newtype wrapper for blockchain addresses providing type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenAddress(pub Address);

impl TokenAddress {
    pub fn new(addr: Address) -> Self {
        Self(addr)
    }

    pub fn zero() -> Self {
        Self(Address::zero())
    }

    pub fn inner(&self) -> Address {
        self.0
    }
}

impl fmt::Display for TokenAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl From<Address> for TokenAddress {
    fn from(addr: Address) -> Self {
        Self(addr)
    }
}

/// Newtype wrapper for pool/pair addresses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoolAddress(pub Address);

impl PoolAddress {
    pub fn new(addr: Address) -> Self {
        Self(addr)
    }

    pub fn inner(&self) -> Address {
        self.0
    }
}

impl fmt::Display for PoolAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl From<Address> for PoolAddress {
    fn from(addr: Address) -> Self {
        Self(addr)
    }
}

/// Newtype wrapper for token amounts providing type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenAmount(pub U256);

impl TokenAmount {
    pub fn new(amount: U256) -> Self {
        Self(amount)
    }

    pub fn zero() -> Self {
        Self(U256::zero())
    }

    pub fn inner(&self) -> U256 {
        self.0
    }

    /// Convert to f64 with decimal adjustment
    pub fn to_f64(&self, decimals: u8) -> f64 {
        let divisor = 10_u128.pow(decimals as u32);
        self.0.as_u128() as f64 / divisor as f64
    }
}

impl fmt::Display for TokenAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<U256> for TokenAmount {
    fn from(amount: U256) -> Self {
        Self(amount)
    }
}

/// Token metadata fetched from on-chain or oracle
#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub address: TokenAddress,
    pub symbol: Option<String>,
    pub decimals: u8,
}

impl TokenMetadata {
    pub fn new(address: TokenAddress, decimals: u8) -> Self {
        Self {
            address,
            symbol: None,
            decimals,
        }
    }

    pub fn with_symbol(mut self, symbol: String) -> Self {
        self.symbol = Some(symbol);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_address_zero() {
        let addr = TokenAddress::zero();
        assert_eq!(addr.inner(), Address::zero());
    }

    #[test]
    fn test_token_amount_to_f64() {
        // 1 token with 18 decimals = 1e18 wei
        let amount = TokenAmount::new(U256::from(1_000_000_000_000_000_000_u128));
        assert_eq!(amount.to_f64(18), 1.0);

        // 0.5 token with 18 decimals = 5e17 wei
        let amount = TokenAmount::new(U256::from(500_000_000_000_000_000_u128));
        assert_eq!(amount.to_f64(18), 0.5);

        // 100 USDC with 6 decimals = 100_000_000
        let amount = TokenAmount::new(U256::from(100_000_000_u128));
        assert_eq!(amount.to_f64(6), 100.0);
    }

    #[test]
    fn test_token_metadata_builder() {
        let addr = TokenAddress::new(Address::zero());
        let metadata = TokenMetadata::new(addr, 18).with_symbol("TEST".to_string());

        assert_eq!(metadata.decimals, 18);
        assert_eq!(metadata.symbol, Some("TEST".to_string()));
    }
}
