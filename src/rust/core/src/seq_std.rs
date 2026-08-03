//! Standardness check for sequence notations (0-Y, 1-Y, ω-Y, LPrSS, HPrSS).
//!
//! A sequence `S = 1, m, #` is checked by building a chain of standard
//! expressions starting from `[1, m+1]`. Each step expands the current
//! expression along its fundamental sequence and either jumps (using the
//! property that `1,#,k` standard implies `1,#,j` standard for `1 <= j < k`)
//! or, if `S` turns out to be a prefix of a standard expression, accepts.
//!
//! Fundamental-sequence elements are prefix-stable: once a position is
//! visible in the chain, its value never changes, so `S[i] > sa_[i]` means
//! `S` lies above the whole chain and is rejected immediately.

pub type SeqExpand = fn(&[i32], i32) -> Vec<i32>;

const MAX_N: i32 = 15;
const MAX_GUARD: usize = 100_000;

/// Check whether `s` is standard in the notation given by `expand`.
pub fn is_standard_sequence(s: &[i32], expand: SeqExpand) -> bool {
    if s.iter().any(|&x| x < 1) {
        return false;
    }
    if s.is_empty() {
        return true;
    }
    if s[0] != 1 {
        return false;
    }
    if s.len() == 1 || s.len() == 2 {
        return true;
    }

    let mut sa: Vec<i32> = vec![1, s[1] + 1];
    let mut guard = 0usize;

    loop {
        guard += 1;
        if guard > MAX_GUARD {
            return false;
        }

        // Cap the expansion index so the exponential chain values stay in i32.
        let n_max = if sa.len() == 2 {
            let base = (sa[1] as f64).log2();
            (30.0 / base).floor().max(1.0) as i32
        } else {
            MAX_N
        };
        let n_max = n_max.min(MAX_N);

        let mut jumped = false;
        for n in 1..=n_max {
            let sa_ = expand(&sa, n);

            let mut i = 1usize;
            while i < s.len() && i < sa_.len() && s[i] == sa_[i] {
                i += 1;
            }
            if i == s.len() {
                // `s` is a prefix of the standard `sa_`, hence standard.
                return true;
            }
            if i >= sa_.len() {
                // `sa_` too short; a larger `n` extends the chain further.
                continue;
            }
            if s[i] > sa_[i] {
                // Above the whole fundamental-sequence chain.
                return false;
            }
            // `s[i] < sa_[i] = k`.
            if i == s.len() - 1 {
                // `s = 1,#,s[i]` with `s[i] < k`, standard by the property.
                return true;
            }
            if s[i] + 1 < sa_[i] {
                // Jump to `1,#,s[i]+1`, standard since `s[i]+1 < k`.
                let mut new_sa: Vec<i32> = sa_[..i].to_vec();
                new_sa.push(s[i] + 1);
                sa = new_sa;
                jumped = true;
                break;
            }
            // `s[i] + 1 == k`: jump to `1,#,k` (the standard prefix of `sa_`).
            // Its fundamental sequence has position `i = k-1 = s[i]`, so the
            // comparison continues with the tail.
            sa = sa_[..i + 1].to_vec();
            jumped = true;
            break;
        }

        if !jumped {
            return false;
        }
    }
}

/// 0-Y sequence standardness.
pub fn zero_y_is_standard(s: &[i32]) -> bool {
    is_standard_sequence(s, crate::zero_y::zero_y_expand)
}

/// 1-Y sequence standardness.
pub fn one_y_is_standard(s: &[i32]) -> bool {
    is_standard_sequence(s, crate::wy::expand_1y)
}

/// ω-Y sequence standardness.
pub fn wy_is_standard(s: &[i32]) -> bool {
    is_standard_sequence(s, crate::wy::expand_wy_seq)
}

/// HPrSS standardness.
pub fn hprss_is_standard(s: &[i32]) -> bool {
    is_standard_sequence(s, crate::hydra::expand_hprss)
}

/// LPrSS standardness.
pub fn lprss_is_standard(s: &[i32]) -> bool {
    is_standard_sequence(s, crate::hydra::expand_lprss)
}
