use anyhow::{bail, Context, Result};
use solana_transaction_status::{option_serializer::OptionSerializer, UiTransactionTokenBalance};

/// Resolve the transaction-wide account indices of a pool's two vaults.
///
/// Vault positions differ per instruction form: a routed 8-account swap names them at
/// positions 3 and 4, the legacy 17-account swap at 5 and 6, deposit/withdraw at 6 and 7.
/// Config positions therefore cannot identify them. They are found by identity instead:
/// token accounts that hold the pool's mint, are owned by the pool authority, and are
/// named by the instruction itself.
pub fn resolve_vault_indices(
    pre_balances: &[UiTransactionTokenBalance],
    post_balances: &[UiTransactionTokenBalance],
    instruction_accounts: &[usize],
    authority: &str,
    token0_mint: &str,
    token1_mint: &str,
) -> Result<(usize, usize)> {
    let vault0 = find_vault(
        pre_balances,
        post_balances,
        instruction_accounts,
        authority,
        token0_mint,
        "token0",
    )?;
    let vault1 = find_vault(
        pre_balances,
        post_balances,
        instruction_accounts,
        authority,
        token1_mint,
        "token1",
    )?;

    if vault0 == vault1 {
        bail!(
            "Both pool mints resolved to vault account index {}; token0 [{}] and token1 [{}] must differ",
            vault0,
            token0_mint,
            token1_mint
        );
    }

    Ok((vault0, vault1))
}

fn find_vault(
    pre_balances: &[UiTransactionTokenBalance],
    post_balances: &[UiTransactionTokenBalance],
    instruction_accounts: &[usize],
    authority: &str,
    mint: &str,
    label: &str,
) -> Result<usize> {
    let mut candidates: Vec<usize> = pre_balances
        .iter()
        .filter(|b| {
            b.mint == mint
                && balance_owner(b) == Some(authority)
                && instruction_accounts.contains(&(b.account_index as usize))
        })
        .map(|b| b.account_index as usize)
        .collect();
    candidates.sort_unstable();
    candidates.dedup();

    match candidates.as_slice() {
        [index] => {
            let index = *index;
            if !post_balances
                .iter()
                .any(|b| b.account_index as usize == index && b.mint == mint)
            {
                bail!("{label} vault {} has no post-token balance", index);
            }
            Ok(index)
        }
        [] => {
            let matching_mint: Vec<_> = pre_balances
                .iter()
                .filter(|b| b.mint == mint)
                .map(|b| (b.account_index, balance_owner(b).unwrap_or("unknown")))
                .collect();
            let matching_authority: Vec<_> = pre_balances
                .iter()
                .filter(|b| balance_owner(b) == Some(authority))
                .map(|b| (b.account_index, &b.mint))
                .collect();
            let in_instruction: Vec<_> = pre_balances
                .iter()
                .filter(|b| instruction_accounts.contains(&(b.account_index as usize)))
                .map(|b| b.account_index)
                .collect();

            bail!(
                "No {label} vault for mint {} owned by pool authority {} is named by this instruction. \
                Debug: accounts matching mint: {:?}, accounts matching authority: {:?}, \
                accounts in instruction: {:?}, instruction_accounts param: {:?}",
                mint,
                authority,
                matching_mint,
                matching_authority,
                in_instruction,
                instruction_accounts
            )
        }
        many => bail!(
            "Ambiguous {label} vault for mint {}: account indices {:?}",
            mint,
            many
        ),
    }
}

fn balance_owner(balance: &UiTransactionTokenBalance) -> Option<&str> {
    match &balance.owner {
        OptionSerializer::Some(owner) => Some(owner.as_str()),
        _ => None,
    }
}

/// Compute actual token amounts from pre/post balance diffs
/// Returns (amount0_delta, amount1_delta) where positive = tokens added to pool, negative = removed
pub fn compute_balance_diff(
    pre_balances: &[UiTransactionTokenBalance],
    post_balances: &[UiTransactionTokenBalance],
    vault0_index: usize,
    vault1_index: usize,
) -> Result<(i64, i64)> {
    let vault0_diff = compute_single_balance_diff(pre_balances, post_balances, vault0_index)
        .context("Failed to compute token0 vault balance diff")?;
    let vault1_diff = compute_single_balance_diff(pre_balances, post_balances, vault1_index)
        .context("Failed to compute token1 vault balance diff")?;

    Ok((vault0_diff, vault1_diff))
}

fn compute_single_balance_diff(
    pre_balances: &[UiTransactionTokenBalance],
    post_balances: &[UiTransactionTokenBalance],
    account_index: usize,
) -> Result<i64> {
    // Find balance entries for this account index
    let pre = pre_balances
        .iter()
        .find(|b| b.account_index as usize == account_index);
    let post = post_balances
        .iter()
        .find(|b| b.account_index as usize == account_index);

    match (pre, post) {
        (Some(pre_bal), Some(post_bal)) => {
            let pre_amount = parse_ui_amount(&pre_bal.ui_token_amount.amount)?;
            let post_amount = parse_ui_amount(&post_bal.ui_token_amount.amount)?;

            // Diff = post - pre (positive if tokens added to vault, negative if removed)
            Ok(post_amount - pre_amount)
        }
        (None, _) => {
            bail!(
                "Missing pre-balance data for account index {}. Available pre-balance indices: {:?}",
                account_index,
                pre_balances.iter().map(|b| b.account_index).collect::<Vec<_>>()
            )
        }
        (_, None) => {
            bail!(
                "Missing post-balance data for account index {}. Available post-balance indices: {:?}",
                account_index,
                post_balances.iter().map(|b| b.account_index).collect::<Vec<_>>()
            )
        }
    }
}

fn parse_ui_amount(amount_str: &str) -> Result<i64> {
    let unsigned: u64 = amount_str
        .parse()
        .with_context(|| format!("Failed to parse token amount: {}", amount_str))?;

    i64::try_from(unsigned).with_context(|| format!("Token amount {} exceeds i64::MAX", amount_str))
}

/// Extract swap fill amounts from balance diffs
/// For swaps: one vault increases (tokens in), one decreases (tokens out)
pub fn extract_swap_amounts(vault0_diff: i64, vault1_diff: i64) -> (u64, u64, u64, u64) {
    let (amount0_in, amount0_out) = if vault0_diff > 0 {
        (vault0_diff as u64, 0)
    } else {
        (0, vault0_diff.unsigned_abs())
    };

    let (amount1_in, amount1_out) = if vault1_diff > 0 {
        (vault1_diff as u64, 0)
    } else {
        (0, vault1_diff.unsigned_abs())
    };

    (amount0_in, amount1_in, amount0_out, amount1_out)
}

/// Extract liquidity amounts (always positive for mint/burn)
pub fn extract_liquidity_amounts(vault0_diff: i64, vault1_diff: i64) -> (u64, u64) {
    (vault0_diff.unsigned_abs(), vault1_diff.unsigned_abs())
}

/// Decimal places for each vault's token, as reported by the transaction itself.
pub fn vault_decimals(
    pre_balances: &[UiTransactionTokenBalance],
    vault0_index: usize,
    vault1_index: usize,
) -> Result<(u8, u8)> {
    Ok((
        decimals_at(pre_balances, vault0_index, "token0")?,
        decimals_at(pre_balances, vault1_index, "token1")?,
    ))
}

fn decimals_at(
    balances: &[UiTransactionTokenBalance],
    account_index: usize,
    label: &str,
) -> Result<u8> {
    balances
        .iter()
        .find(|b| b.account_index as usize == account_index)
        .map(|b| b.ui_token_amount.decimals)
        .with_context(|| format!("{label} vault {account_index} has no token balance entry"))
}

/// USD value of `amount` base units at `price` USD per whole token.
/// Returns `None` when either side is non-positive, so a missing oracle quote
/// never masquerades as zero volume.
pub fn volume_usd(amount: u64, decimals: u8, price: f64) -> Option<f64> {
    if amount == 0 || price <= 0.0 {
        return None;
    }
    Some(price * (amount as f64) / 10f64.powi(decimals as i32))
}

/// Price of the sold token quoted in whole units of the bought token.
/// Raw base-unit amounts must be scaled by both tokens' decimals; without that
/// the ratio is off by 10^(decimals_sold - decimals_bought).
pub fn swap_price(
    amount0_in: u64,
    amount1_in: u64,
    amount0_out: u64,
    amount1_out: u64,
    decimals0: u8,
    decimals1: u8,
) -> f64 {
    let (received, received_decimals, paid, paid_decimals) = if amount0_in > 0 {
        (amount1_out, decimals1, amount0_in, decimals0)
    } else if amount1_in > 0 {
        (amount0_out, decimals0, amount1_in, decimals1)
    } else {
        return 0.0;
    };

    if paid == 0 {
        return 0.0;
    }

    (received as f64 / paid as f64) * 10f64.powi(paid_decimals as i32 - received_decimals as i32)
}

/// Compute liquidity amounts from pre/post balance diffs
pub fn compute_liquidity_amounts(
    pre_balances: &[UiTransactionTokenBalance],
    post_balances: &[UiTransactionTokenBalance],
    vault0_index: usize,
    vault1_index: usize,
) -> Result<(u64, u64)> {
    let (v0_diff, v1_diff) =
        compute_balance_diff(pre_balances, post_balances, vault0_index, vault1_index)?;
    Ok(extract_liquidity_amounts(v0_diff, v1_diff))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_account_decoder::parse_token::UiTokenAmount;
    use solana_transaction_status::option_serializer::OptionSerializer;

    fn mock_token_balance(account_index: u8, amount: &str) -> UiTransactionTokenBalance {
        build_balance(account_index, "mock_mint", None, amount, 9)
    }

    fn build_balance(
        account_index: u8,
        mint: &str,
        owner: Option<&str>,
        amount: &str,
        decimals: u8,
    ) -> UiTransactionTokenBalance {
        UiTransactionTokenBalance {
            account_index,
            mint: mint.to_string(),
            ui_token_amount: UiTokenAmount {
                ui_amount: None,
                decimals,
                amount: amount.to_string(),
                ui_amount_string: amount.to_string(),
            },
            owner: match owner {
                Some(o) => OptionSerializer::Some(o.to_string()),
                None => OptionSerializer::None,
            },
            program_id: OptionSerializer::None,
        }
    }

    const AUTHORITY: &str = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
    const SOL: &str = "So11111111111111111111111111111111111111112";
    const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    // Layout of the routed 8-account swap form that names the vaults at positions 3 and 4.
    // The user's own USDC account (index 9) must not be mistaken for the pool vault.
    fn routed_swap_balances() -> (
        Vec<UiTransactionTokenBalance>,
        Vec<UiTransactionTokenBalance>,
    ) {
        let pre = vec![
            build_balance(9, USDC, Some("HrvUm2pX"), "2926759165", 6),
            build_balance(17, SOL, Some(AUTHORITY), "97504098920579", 9),
            build_balance(22, USDC, Some(AUTHORITY), "9069751841364", 6),
        ];
        let post = vec![
            build_balance(9, USDC, Some("HrvUm2pX"), "2876756438", 6),
            build_balance(17, SOL, Some(AUTHORITY), "97503565422545", 9),
            build_balance(22, USDC, Some(AUTHORITY), "9069801591564", 6),
        ];
        (pre, post)
    }

    const ROUTED_ACCOUNTS: [usize; 8] = [28, 6, 32, 17, 22, 9, 13, 1];

    #[test]
    fn vaults_are_resolved_by_identity_not_position() {
        let (pre, post) = routed_swap_balances();

        let (vault0, vault1) =
            resolve_vault_indices(&pre, &post, &ROUTED_ACCOUNTS, AUTHORITY, SOL, USDC).unwrap();

        assert_eq!((vault0, vault1), (17, 22));

        let (v0_diff, v1_diff) = compute_balance_diff(&pre, &post, vault0, vault1).unwrap();
        assert_eq!(v0_diff, -533498034); // SOL leaves the vault
        assert_eq!(v1_diff, 49750200); // USDC enters the vault
    }

    #[test]
    fn balance_entries_not_named_by_the_instruction_are_ignored() {
        let (pre, post) = routed_swap_balances();

        // The user's USDC account (9) is dropped, so nothing should resolve to it.
        let (vault0, _vault1) =
            resolve_vault_indices(&pre, &post, &ROUTED_ACCOUNTS, AUTHORITY, SOL, USDC).unwrap();

        assert_ne!(vault0, 9);
    }

    #[test]
    fn vault_must_be_owned_by_the_pool_authority() {
        let (pre, post) = routed_swap_balances();

        let err = resolve_vault_indices(
            &pre,
            &post,
            &ROUTED_ACCOUNTS,
            "someOtherAuthority",
            SOL,
            USDC,
        )
        .unwrap_err();

        assert!(format!("{err:#}").contains("No token0 vault"));
    }

    #[test]
    fn vault_must_be_named_by_the_instruction() {
        let (pre, post) = routed_swap_balances();
        let without_vaults = [28, 6, 32, 9, 1];

        let err =
            resolve_vault_indices(&pre, &post, &without_vaults, AUTHORITY, SOL, USDC).unwrap_err();

        assert!(format!("{err:#}").contains("No token0 vault"));
    }

    #[test]
    fn two_matching_vaults_are_reported_as_ambiguous() {
        let (pre, post) = routed_swap_balances();
        let mut pre = pre;
        pre.push(build_balance(23, SOL, Some(AUTHORITY), "1000", 9));
        let mut post = post;
        post.push(build_balance(23, SOL, Some(AUTHORITY), "2000", 9));
        let accounts = [28, 6, 32, 17, 22, 23, 9, 1];

        let err = resolve_vault_indices(&pre, &post, &accounts, AUTHORITY, SOL, USDC).unwrap_err();

        assert!(format!("{err:#}").contains("Ambiguous token0 vault"));
    }

    #[test]
    fn same_resolved_index_for_both_mints_is_rejected() {
        let pre = vec![build_balance(7, SOL, Some(AUTHORITY), "1000", 9)];
        let post = vec![build_balance(7, SOL, Some(AUTHORITY), "2000", 9)];

        let err = resolve_vault_indices(&pre, &post, &[7], AUTHORITY, SOL, SOL).unwrap_err();

        assert!(format!("{err:#}").contains("must differ"));
    }

    #[test]
    fn vault_decimals_come_from_the_transaction() {
        let (pre, _post) = routed_swap_balances();

        let (decimals0, decimals1) = vault_decimals(&pre, 17, 22).unwrap();

        assert_eq!((decimals0, decimals1), (9, 6));
    }

    #[test]
    fn missing_vault_entry_is_named_in_the_decimals_error() {
        let (pre, _post) = routed_swap_balances();

        let err = vault_decimals(&pre, 17, 99).unwrap_err();

        assert!(format!("{err:#}").contains("token1 vault 99"));
    }

    #[test]
    fn swap_price_scales_both_tokens_by_their_decimals() {
        // Routed fixture: 533_498_034 lamports of SOL (9 dp) bought 49_750_200 USDC base units (6 dp).
        let price = swap_price(533_498_034, 0, 0, 49_750_200, 9, 6);

        assert!((price - 93.25).abs() < 0.01, "price was {price}");
    }

    #[test]
    fn swap_price_is_inverted_when_token1_is_the_input() {
        let forward = swap_price(533_498_034, 0, 0, 49_750_200, 9, 6);
        let reverse = swap_price(0, 49_750_200, 533_498_034, 0, 9, 6);

        assert!((forward * reverse - 1.0).abs() < 0.001);
    }

    #[test]
    fn equal_decimal_tokens_keep_the_raw_ratio() {
        // 18-dp in, 18-dp out: scaling must cancel, leaving the plain ratio.
        assert!(
            (swap_price(
                2_000_000_000_000_000_000,
                0,
                0,
                1_000_000_000_000_000_000,
                18,
                18
            ) - 0.5)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn swap_price_without_an_input_is_zero() {
        assert_eq!(swap_price(0, 0, 500, 700, 9, 6), 0.0);
    }

    #[test]
    fn volume_usd_uses_the_sold_token_decimals() {
        // Same trade valued in either token must agree: 49_750_200 USDC base units (6 dp)
        // and 533_498_034 SOL lamports (9 dp) at SOL = 93.25 USD.
        let from_usdc = volume_usd(49_750_200, 6, 1.0).unwrap();
        let from_sol = volume_usd(533_498_034, 9, 93.25).unwrap();

        assert!(
            (from_usdc - 49.7502).abs() < 1e-6,
            "usdc leg was {from_usdc}"
        );
        assert!(
            (from_sol - from_usdc).abs() < 0.01,
            "sol leg was {from_sol}"
        );
    }

    #[test]
    fn volume_usd_is_none_without_a_quote_or_an_amount() {
        assert_eq!(volume_usd(1_000_000, 6, 0.0), None);
        assert_eq!(volume_usd(0, 6, 1.0), None);
    }

    #[test]
    fn test_swap_balance_diff() {
        // Swap: user sends 1M token0 (vault0 increases), receives 990k token1 (vault1 decreases)
        let pre = vec![
            mock_token_balance(5, "10000000"), // vault0 pre
            mock_token_balance(6, "10000000"), // vault1 pre
        ];
        let post = vec![
            mock_token_balance(5, "11000000"), // vault0 post (+1M)
            mock_token_balance(6, "9010000"),  // vault1 post (-990k)
        ];

        let (v0_diff, v1_diff) = compute_balance_diff(&pre, &post, 5, 6).unwrap();
        assert_eq!(v0_diff, 1000000); // +1M token0 into pool
        assert_eq!(v1_diff, -990000); // -990k token1 out of pool

        let (a0_in, a1_in, a0_out, a1_out) = extract_swap_amounts(v0_diff, v1_diff);
        assert_eq!(a0_in, 1000000);
        assert_eq!(a1_in, 0);
        assert_eq!(a0_out, 0);
        assert_eq!(a1_out, 990000);
    }

    #[test]
    fn test_deposit_balance_diff() {
        // Deposit: both vaults increase
        let pre = vec![
            mock_token_balance(5, "10000000"),
            mock_token_balance(6, "10000000"),
        ];
        let post = vec![
            mock_token_balance(5, "15000000"), // +5M
            mock_token_balance(6, "15000000"), // +5M
        ];

        let (v0_diff, v1_diff) = compute_balance_diff(&pre, &post, 5, 6).unwrap();
        let (amount0, amount1) = extract_liquidity_amounts(v0_diff, v1_diff);
        assert_eq!(amount0, 5000000);
        assert_eq!(amount1, 5000000);
    }
}
