use ethers::types::U256;

/// Price of the sold token quoted in whole units of the bought token.
pub(crate) fn calculate_swap_price(
    amount0_in: &U256,
    amount1_in: &U256,
    amount0_out: &U256,
    amount1_out: &U256,
    token0_decimals: u8,
    token1_decimals: u8,
) -> f64 {
    if !amount0_in.is_zero() && !amount1_out.is_zero() {
        return ratio_to_f64(amount1_out, amount0_in, token1_decimals, token0_decimals);
    }
    if !amount1_in.is_zero() && !amount0_out.is_zero() {
        return ratio_to_f64(amount1_in, amount0_out, token1_decimals, token0_decimals);
    }
    0.0
}

fn ratio_to_f64(numerator: &U256, denominator: &U256, num_decimals: u8, denom_decimals: u8) -> f64 {
    if denominator.is_zero() {
        return 0.0;
    }

    let num_f64 = u256_to_f64(numerator);
    let denom_f64 = u256_to_f64(denominator);

    if denom_f64 == 0.0 {
        return 0.0;
    }

    let decimal_diff = denom_decimals as i32 - num_decimals as i32;
    (num_f64 / denom_f64) * 10_f64.powi(decimal_diff)
}

fn u256_to_f64(value: &U256) -> f64 {
    let mut result = 0.0;
    let mut multiplier = 1.0;

    for word in value.0 {
        result += word as f64 * multiplier;
        multiplier *= 2_f64.powi(64);
    }

    result
}

/// The traded amount of one side: whichever of the two is non-zero.
pub(crate) fn non_zero_amount(amount_in: &U256, amount_out: &U256) -> U256 {
    if amount_in.is_zero() {
        *amount_out
    } else {
        *amount_in
    }
}

pub(crate) fn to_usd(amount: &U256, decimals: u8, usd_price: f64) -> f64 {
    (u256_to_f64(amount) / 10_f64.powi(decimals as i32)) * usd_price
}

/// Volume in USD for one swap, choosing the most trustworthy quote available.
///
/// Preference order is policy, not arithmetic: a stablecoin side is worth exactly $1, then
/// either priced side, and only then the swap's own ratio — which is denominated in the
/// other token, so it is the weakest signal.
pub(crate) fn calculate_volume_usd(
    volume0: &U256,
    volume1: &U256,
    token0_decimals: u8,
    token1_decimals: u8,
    token0_price: Option<f64>,
    token1_price: Option<f64>,
    swap_price: f64,
) -> Option<f64> {
    match (token0_price, token1_price) {
        (_, Some(price1)) if price1 == 1.0 => Some(to_usd(volume1, token1_decimals, 1.0)),
        (Some(price0), _) if price0 == 1.0 => Some(to_usd(volume0, token0_decimals, 1.0)),
        (Some(price0), _) if price0 > 0.0 => Some(to_usd(volume0, token0_decimals, price0)),
        (_, Some(price1)) if price1 > 0.0 => Some(to_usd(volume1, token1_decimals, price1)),
        _ if swap_price > 0.0 => Some(to_usd(volume0, token0_decimals, swap_price)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_scales_both_tokens_by_their_decimals() {
        // 1e18 of an 18-dp token bought 2e6 of a 6-dp token: both sides are worth 1.0 and 2.0.
        let price = calculate_swap_price(
            &U256::from(1_000_000_000_000_000_000u64),
            &U256::zero(),
            &U256::zero(),
            &U256::from(2_000_000u64),
            18,
            6,
        );

        assert!((price - 2.0).abs() < 0.0001, "price was {price}");
    }

    #[test]
    fn price_is_computed_from_either_direction() {
        let token0_in = calculate_swap_price(
            &U256::from(1_000_000_000_000_000_000u64),
            &U256::zero(),
            &U256::zero(),
            &U256::from(2_000_000u64),
            18,
            6,
        );
        let token1_in = calculate_swap_price(
            &U256::zero(),
            &U256::from(2_000_000u64),
            &U256::from(1_000_000_000_000_000_000u64),
            &U256::zero(),
            18,
            6,
        );

        assert!((token0_in - 2.0).abs() < 0.0001, "price was {token0_in}");
        assert!((token1_in - 2.0).abs() < 0.0001, "price was {token1_in}");
    }

    #[test]
    fn price_without_a_counterparty_amount_is_zero() {
        let price = calculate_swap_price(
            &U256::from(100u64),
            &U256::zero(),
            &U256::zero(),
            &U256::zero(),
            18,
            6,
        );

        assert_eq!(price, 0.0);
    }

    #[test]
    fn non_zero_amount_prefers_the_input_side() {
        let zero = U256::zero();
        let non_zero = U256::from(100);

        assert_eq!(non_zero_amount(&non_zero, &zero), non_zero);
        assert_eq!(non_zero_amount(&zero, &non_zero), non_zero);
        assert_eq!(non_zero_amount(&zero, &zero), zero);
    }

    #[test]
    fn to_usd_applies_decimals_and_price() {
        assert!((to_usd(&U256::from(1_000_000u64), 6, 1.0) - 1.0).abs() < 0.0001);
        assert!(
            (to_usd(&U256::from(1_000_000_000_000_000_000u64), 18, 2000.0) - 2000.0).abs() < 0.0001
        );
    }

    #[test]
    fn u256_to_f64_handles_values_wider_than_one_word() {
        // Only the high word is set, so dropping or mis-ordering the words yields 0.0 or 7.0.
        let value = U256::from(7u64) << 64;

        assert_eq!(u256_to_f64(&value), 129127208515966861312.0);
    }

    const ONE_TOKEN_6DP: u64 = 1_000_000;
    const ONE_TOKEN_18DP: u64 = 1_000_000_000_000_000_000;

    #[test]
    fn volume_prefers_a_stablecoin_side() {
        // Token1 is the stablecoin, so its raw amount is the USD volume regardless of token0's quote.
        let volume = calculate_volume_usd(
            &U256::from(ONE_TOKEN_18DP),
            &U256::from(3 * ONE_TOKEN_6DP),
            18,
            6,
            Some(2500.0),
            Some(1.0),
            2500.0,
        );

        assert!(
            (volume.unwrap() - 3.0).abs() < 0.0001,
            "volume was {volume:?}"
        );
    }

    #[test]
    fn volume_prefers_token0_stablecoin_over_token1_price() {
        let volume = calculate_volume_usd(
            &U256::from(5 * ONE_TOKEN_6DP),
            &U256::from(ONE_TOKEN_18DP),
            6,
            18,
            Some(1.0),
            Some(2500.0),
            0.0004,
        );

        assert!(
            (volume.unwrap() - 5.0).abs() < 0.0001,
            "volume was {volume:?}"
        );
    }

    #[test]
    fn volume_falls_back_to_the_priced_side_then_the_swap_ratio() {
        let priced_token0 = calculate_volume_usd(
            &U256::from(2 * ONE_TOKEN_18DP),
            &U256::from(ONE_TOKEN_18DP),
            18,
            18,
            Some(2000.0),
            Some(0.5),
            2000.0,
        );
        let ratio_priced = calculate_volume_usd(
            &U256::from(2 * ONE_TOKEN_18DP),
            &U256::from(ONE_TOKEN_18DP),
            18,
            18,
            None,
            None,
            2000.0,
        );

        assert!(
            (priced_token0.unwrap() - 4000.0).abs() < 0.001,
            "volume was {priced_token0:?}"
        );
        assert!(
            (ratio_priced.unwrap() - 4000.0).abs() < 0.001,
            "volume was {ratio_priced:?}"
        );
    }

    #[test]
    fn volume_is_none_without_any_usable_price() {
        let volume = calculate_volume_usd(
            &U256::from(ONE_TOKEN_18DP),
            &U256::from(ONE_TOKEN_6DP),
            18,
            6,
            None,
            Some(0.0),
            0.0,
        );

        assert_eq!(volume, None);
    }
}
