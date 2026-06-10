//! Model pricing
//!
//! Approximate public per-token pricing used to estimate request cost from
//! token usage. Prices are in USD per 1,000,000 tokens and are intentionally
//! kept in one table so they are easy to audit and adjust.
//!
//! These are estimates: providers change prices, and features like prompt
//! caching or batch pricing are not modelled here. Unknown models fall back to
//! zero cost (token counts are still recorded).

/// Per-token pricing for a model, in USD per 1M tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPricing {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
}

/// (substring matched against the lowercased model name, input $/Mtok, output $/Mtok).
///
/// Ordered most-specific first; the first substring match wins.
const TABLE: &[(&str, f64, f64)] = &[
    // ── Anthropic Claude ───────────────────────────────────────────────
    ("claude-3-5-haiku", 0.80, 4.0),
    ("claude-3-haiku", 0.25, 1.25),
    ("haiku", 0.80, 4.0),
    ("claude-opus", 15.0, 75.0),
    ("opus", 15.0, 75.0),
    ("claude-sonnet", 3.0, 15.0),
    ("sonnet", 3.0, 15.0),
    // ── OpenAI ─────────────────────────────────────────────────────────
    ("gpt-4o-mini", 0.15, 0.60),
    ("gpt-4o", 2.50, 10.0),
    ("gpt-4.1-nano", 0.10, 0.40),
    ("gpt-4.1-mini", 0.40, 1.60),
    ("gpt-4.1", 2.00, 8.0),
    ("gpt-4-turbo", 10.0, 30.0),
    ("gpt-4", 30.0, 60.0),
    ("gpt-3.5", 0.50, 1.50),
    ("o1-mini", 1.10, 4.40),
    ("o3-mini", 1.10, 4.40),
    ("o1", 15.0, 60.0),
    // ── DeepSeek ───────────────────────────────────────────────────────
    ("deepseek-reasoner", 0.55, 2.19),
    ("deepseek", 0.27, 1.10),
    // ── Google Gemini ──────────────────────────────────────────────────
    ("gemini-2.5-pro", 1.25, 10.0),
    ("gemini-1.5-pro", 1.25, 5.0),
    ("gemini-2.5-flash", 0.30, 2.50),
    ("gemini-2.0-flash", 0.10, 0.40),
    ("gemini-1.5-flash", 0.075, 0.30),
    ("flash", 0.10, 0.40),
    ("gemini", 1.25, 5.0),
    // ── Moonshot / Kimi (approx) ───────────────────────────────────────
    ("kimi", 0.15, 0.15),
    ("moonshot", 0.15, 0.15),
];

/// Look up pricing for a model name (substring match), if known.
pub fn pricing_for(model: &str) -> Option<ModelPricing> {
    let m = model.to_ascii_lowercase();
    for (pat, inp, out) in TABLE {
        if m.contains(pat) {
            return Some(ModelPricing {
                input_per_mtok: *inp,
                output_per_mtok: *out,
            });
        }
    }
    None
}

/// Estimate the USD cost of a request given its model and token counts.
/// Unknown models cost `0.0`.
pub fn compute_cost(model: &str, input_tokens: i64, output_tokens: i64) -> f64 {
    match pricing_for(model) {
        Some(p) => {
            (input_tokens as f64 / 1_000_000.0) * p.input_per_mtok
                + (output_tokens as f64 / 1_000_000.0) * p.output_per_mtok
        }
        None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_specific_before_generic() {
        // 3.5-haiku must not be captured by the generic "haiku" entry's value
        assert_eq!(
            pricing_for("claude-3-5-haiku-20241022")
                .unwrap()
                .input_per_mtok,
            0.80
        );
        assert_eq!(
            pricing_for("claude-3-haiku-20240307")
                .unwrap()
                .input_per_mtok,
            0.25
        );
        // gpt-4o-mini must not match the generic gpt-4 entry
        assert_eq!(pricing_for("gpt-4o-mini").unwrap().output_per_mtok, 0.60);
        assert_eq!(
            pricing_for("gpt-4o-2024-08-06").unwrap().input_per_mtok,
            2.50
        );
    }

    #[test]
    fn computes_cost() {
        // sonnet: 10 in @ $3/Mtok + 5 out @ $15/Mtok
        let cost = compute_cost("claude-3-5-sonnet-20241022", 10, 5);
        assert!((cost - (10.0 / 1e6 * 3.0 + 5.0 / 1e6 * 15.0)).abs() < 1e-12);
        // unknown model -> zero
        assert_eq!(compute_cost("some-unknown-model", 1000, 1000), 0.0);
    }
}
