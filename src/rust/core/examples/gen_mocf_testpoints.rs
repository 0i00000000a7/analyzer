//! Generate extra MOCF↔BOCF test points.
//!
//! Seeds a pool with `[0, 1, ω, ψ(Ω), ψ(Ω_Ω), ψ(Ω_Ω_Ω)]`, then for ROUNDS
//! rounds expands every limit ordinal in the pool by its fundamental sequence
//! at n = 0..=4, deduplicating by ordinal equality (canonical BOCF string
//! keys — the Term uses Rc and cannot be shared across threads). Each
//! surviving ordinal is rendered as BOCF (term_to_string) and MOCF (forward
//! `term_to_mocf`), sorted by value, and written to
//! `bocf vs mocf generated.csv` (the existing CSV is untouched).
//!
//! Run with:
//!   cargo run --release -p bms-core --example gen_mocf_testpoints

use std::collections::HashSet;

use bms_core::term as tm;

const ROUNDS: usize = 6;

fn main() {
    let omega1 = tm::omega1();
    let omega_omega = tm::t(omega1.clone(), tm::zero(), tm::zero()); // Ω_Ω
    let omega_omega_omega = tm::t(omega_omega.clone(), tm::zero(), tm::zero()); // Ω_Ω_Ω
    let psi_omega_omega = tm::t(tm::zero(), omega_omega.clone(), tm::zero()); // ψ(Ω_Ω) — threshold

    let seeds = [
        tm::zero(),
        tm::one(),
        tm::omega(),
        tm::t(tm::zero(), omega1, tm::zero()),                     // ψ(Ω)
        psi_omega_omega.clone(),                                   // ψ(Ω_Ω)
        tm::t(tm::zero(), omega_omega_omega, tm::zero()),          // ψ(Ω_Ω_Ω)
    ];

    let mut pool: Vec<tm::Term> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for seed in seeds {
        push_unique(&mut pool, &mut seen, seed);
    }
    println!("seed terms: {}", pool.len());

    for round in 0..ROUNDS {
        let snapshot = pool.clone();
        let total = snapshot.len();
        for (i, t) in snapshot.into_iter().enumerate() {
            if tm::is_zero(&t) || tm::is_succ(&t) {
                continue;
            }
            // Round 5+ (0-indexed 4+): stop expanding at ψ(Ω_Ω) and above.
            if round >= 4 && !tm::lt(&t, &psi_omega_omega) {
                continue;
            }
            for n in 0..=4 {
                push_unique(&mut pool, &mut seen, tm::fundamental_sequence(&t, n));
            }
            if (i + 1) % 200 == 0 {
                println!(
                    "  round {}/{}: {}/{} terms, pool {}",
                    round + 1,
                    ROUNDS,
                    i + 1,
                    total,
                    pool.len()
                );
            }
        }
        println!("round {} done: {} terms", round + 1, pool.len());
    }

    pool.sort_by(|a, b| {
        if tm::lt(a, b) {
            std::cmp::Ordering::Less
        } else if tm::eq(a, b) {
            std::cmp::Ordering::Equal
        } else {
            std::cmp::Ordering::Greater
        }
    });

    let mut out = String::from("\"Buchholz's OCF\",\"Madore's OCF\"\n");
    for t in &pool {
        let bocf = tm::term_to_string(false, &tm::standard_form(t));
        let mocf = bms_core::bocf_mocf::term_to_mocf(t);
        out.push_str(&format!("\"{}\",\"{}\"\n", bocf, mocf));
    }
    // Repo root, next to the hand-written "bocf vs mocf.csv".
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../bocf vs mocf generated.csv");
    std::fs::write(path, out).expect("write generated csv");
    println!("wrote {} rows to {}", pool.len(), path);
}

fn push_unique(pool: &mut Vec<tm::Term>, seen: &mut HashSet<String>, t: tm::Term) {
    let sf = tm::standard_form(&t);
    let key = tm::term_to_string(false, &sf);
    if seen.insert(key) {
        pool.push(sf);
    }
}
