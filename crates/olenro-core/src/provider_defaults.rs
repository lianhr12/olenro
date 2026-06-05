//! Provider defaults
//!
//! Default icons and related constants

use std::collections::HashMap;

/// Default provider icons (lazy initialized)
pub fn default_provider_icons() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("openai", "openai");
    m.insert("anthropic", "anthropic");
    m.insert("claude", "claude");
    m.insert("google", "google");
    m.insert("gemini", "gemini");
    m.insert("deepseek", "deepseek");
    m.insert("kimi", "kimi");
    m.insert("moonshot", "moonshot");
    m.insert("zhipu", "zhipu");
    m.insert("minimax", "minimax");
    m.insert("baidu", "baidu");
    m.insert("alibaba", "alibaba");
    m.insert("tencent", "tencent");
    m.insert("meta", "meta");
    m.insert("microsoft", "microsoft");
    m.insert("cohere", "cohere");
    m.insert("perplexity", "perplexity");
    m.insert("mistral", "mistral");
    m.insert("huggingface", "huggingface");
    m.insert("aws", "aws");
    m.insert("azure", "azure");
    m.insert("huawei", "huawei");
    m.insert("cloudflare", "cloudflare");
    m.insert("default", "default");
    m
}

/// Infer icon from provider name
pub fn infer_provider_icon(provider_name: &str) -> &'static str {
    let lower = provider_name.to_lowercase();
    let icons = default_provider_icons();
    for (key, icon) in icons.iter() {
        if lower.contains(key) {
            return icon;
        }
    }
    "default"
}
