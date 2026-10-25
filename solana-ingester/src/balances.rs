use anyhow::{bail, Context, Result};
use solana_transaction_status::UiTransactionTokenBalance;

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
        _ => bail!("Missing balance data for account index {}", account_index),
    }
}

fn parse_ui_amount(amount_str: &str) -> Result<i64> {
    amount_str
        .parse::<i64>()
        .with_context(|| format!("Failed to parse token amount: {}", amount_str))
}

/// Extract swap fill amounts from balance diffs
/// For swaps: one vault increases (tokens in), one decreases (tokens out)
pub fn extract_swap_amounts(vault0_diff: i64, vault1_diff: i64) -> (u64, u64, u64, u64) {
    let (amount0_in, amount0_out) = if vault0_diff > 0 {
        (vault0_diff as u64, 0)
    } else {
        (0, vault0_diff.abs() as u64)
    };

    let (amount1_in, amount1_out) = if vault1_diff > 0 {
        (vault1_diff as u64, 0)
    } else {
        (0, vault1_diff.abs() as u64)
    };

    (amount0_in, amount1_in, amount0_out, amount1_out)
}

/// Extract liquidity amounts (always positive for mint/burn)
pub fn extract_liquidity_amounts(vault0_diff: i64, vault1_diff: i64) -> (u64, u64) {
    (vault0_diff.abs() as u64, vault1_diff.abs() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_transaction_status::UiTokenAmount;

    fn mock_token_balance(account_index: u8, amount: &str) -> UiTransactionTokenBalance {
        UiTransactionTokenBalance {
            account_index,
            mint: "mock_mint".to_string(),
            ui_token_amount: UiTokenAmount {
                ui_amount: None,
                decimals: 9,
                amount: amount.to_string(),
                ui_amount_string: amount.to_string(),
            },
            owner: None,
            program_id: None,
        }
    }

    #[test]
    fn test_swap_balance_diff() {
        // Swap: user sends 1M token0 (vault0 increases), receives 990k token1 (vault1 decreases)
        let pre = vec![
            mock_token_balance(5, "10000000"),  // vault0 pre
            mock_token_balance(6, "10000000"),  // vault1 pre
        ];
        let post = vec![
            mock_token_balance(5, "11000000"),  // vault0 post (+1M)
            mock_token_balance(6, "9010000"),   // vault1 post (-990k)
        ];

        let (v0_diff, v1_diff) = compute_balance_diff(&pre, &post, 5, 6).unwrap();
        assert_eq!(v0_diff, 1000000);   // +1M token0 into pool
        assert_eq!(v1_diff, -990000);   // -990k token1 out of pool

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
            mock_token_balance(5, "15000000"),  // +5M
            mock_token_balance(6, "15000000"),  // +5M
        ];

        let (v0_diff, v1_diff) = compute_balance_diff(&pre, &post, 5, 6).unwrap();
        let (amount0, amount1) = extract_liquidity_amounts(v0_diff, v1_diff);
        assert_eq!(amount0, 5000000);
        assert_eq!(amount1, 5000000);
    }
}
