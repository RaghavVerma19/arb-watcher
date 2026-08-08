pub const BPS: u64 = 10_000;

pub fn swap_out_given_in(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
) -> u64 {
    if amount_in == 0 || reserve_in == 0 || reserve_out == 0 {
        return 0;
    }

    let amount_in = amount_in as u128;
    let reserve_in = reserve_in as u128;
    let reserve_out = reserve_out as u128;
    let fee_bps = fee_bps.min((BPS - 1) as u16) as u128;

    let fee = amount_in * fee_bps / BPS as u128;
    let effective_in = amount_in - fee;

    ((effective_in * reserve_out) / (reserve_in + effective_in)) as u64
}

pub fn swap_in_given_out(
    amount_out: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
) -> u64 {
    if amount_out == 0 || reserve_out <= amount_out {
        return 0;
    }

    let amount_out = amount_out as u128;
    let reserve_in = reserve_in as u128;
    let reserve_out = reserve_out as u128;
    let fee_bps = fee_bps.min((BPS - 1) as u16) as u128;

    let effective_in = (reserve_in * amount_out) / (reserve_out - amount_out);

    let denominator = BPS as u128 - fee_bps;
    let fee_charge = (effective_in * fee_bps) / denominator;

    (effective_in + fee_charge) as u64
}

pub fn spot_price(reserve_base: u64, reserve_quote: u64) -> f64 {
    reserve_quote as f64 / reserve_base as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_fee_constant_product_output() {
        let out = swap_out_given_in(1, 100, 10_000, 0);
        assert_eq!(out, 99);
    }

    #[test]
    fn fee_reduces_output() {
        let with_fee = swap_out_given_in(1_000_000, 100_000_000, 100_000_000, 30);
        let no_fee = swap_out_given_in(1_000_000, 100_000_000, 100_000_000, 0);
        assert_eq!(no_fee, 990_099);
        assert_eq!(with_fee, 987_158);
    }

    #[test]
    fn zero_amount_or_reserve_yields_zero() {
        assert_eq!(swap_out_given_in(0, 100, 10_000, 30), 0);
        assert_eq!(swap_out_given_in(5, 0, 10_000, 30), 0);
        assert_eq!(swap_out_given_in(5, 100, 0, 30), 0);
    }

    #[test]
    fn swap_in_out_round_trip() {
        let reserve_in = 1_000_000_000u64;
        let reserve_out = 10_000_000_000u64;
        let fee_bps = 30;

        let out = swap_out_given_in(100_000, reserve_in, reserve_out, fee_bps);
        let back_in = swap_in_given_out(out, reserve_in, reserve_out, fee_bps);

        assert!(back_in <= 100_000, "got {back_in}");
        assert!(100_000 - back_in < 5, "round-trip drift too big: {back_in}");
    }

    #[test]
    fn spot_price_is_ratio() {
        assert!((spot_price(100, 10_000) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn fee_bps_clamped_to_bps_minus_one() {
        let out = swap_out_given_in(1_000, 100_000, 100_000, 10_000);
        assert_eq!(out, 0, "100% fee consumes entire input");
        let out2 = swap_in_given_out(1, 100_000, 100_000, 10_000);
        assert!(out2 > 0, "99.99% fee should still produce output");
    }

    #[test]
    fn round_trip_never_exceeds_input() {
        for reserve in [1_000, 1_000_000, 1_000_000_000u64] {
            for amount in [1, 100, 10_000, reserve / 100] {
                for fee in [0, 1, 30, 100, 500, 9_999] {
                    let out = swap_out_given_in(amount, reserve, reserve, fee);
                    if out == 0 {
                        continue;
                    }
                    let back = swap_in_given_out(out, reserve, reserve, fee);
                    assert!(back <= amount, "fee={fee} amount={amount} back={back}");
                }
            }
        }
    }

    #[test]
    fn product_invariant_under_swap() {
        for reserve_in in [1_000_000u64, 1_000_000_000, 10_000_000_000] {
            for reserve_out in [1_000_000u64, 1_000_000_000, 10_000_000_000] {
                let k_before = reserve_in as u128 * reserve_out as u128;
                for amount_in in [1u64, 100, 1_000, 10_000].iter().cloned() {
                    if amount_in >= reserve_in {
                        continue;
                    }
                    let amount_out = swap_out_given_in(amount_in, reserve_in, reserve_out, 30);
                    if amount_out == 0 || amount_out >= reserve_out {
                        continue;
                    }
                    let fee = (amount_in as u128 * 30) / 10_000;
                    let effective_in = amount_in as u128 - fee;
                    let new_reserve_in = reserve_in as u128 + effective_in;
                    let new_reserve_out = reserve_out as u128 - amount_out as u128;
                    let k_after = new_reserve_in * new_reserve_out;
                    // k_after >= k_before because amount_out is rounded down.
                    assert!(
                        k_after >= k_before,
                        "k decreased: {} -> {} for amount_in={amount_in}",
                        k_before,
                        k_after
                    );
                }
            }
        }
    }

    #[test]
    fn spot_price_monotonic_with_reserves() {
        for reserve_in in [100u64, 1_000, 10_000, 1_000_000] {
            for reserve_out in [100u64, 1_000, 10_000, 1_000_000] {
                let p = spot_price(reserve_in, reserve_out);
                assert!(p > 0.0);
                let p2 = spot_price(reserve_in * 2, reserve_out);
                assert!(p2 < p, "doubling base should lower quote price");
            }
        }
    }
}
