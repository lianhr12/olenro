//! Built-in provider presets
//!
//! The desktop app ships large per-app preset catalogs (`src/config/
//! *ProviderPresets.ts`) that pre-fill the "add provider" form with known
//! endpoints, model routes and API-key fields. Those arrays are the single
//! source of truth; a vitest generator (`scripts/genProviderPresets.test.ts`)
//! evaluates them — including the entries whose config is built by helper
//! functions — and serializes the result to `assets/provider_presets.json`,
//! which we embed here. This keeps the CLI/TUI preset selector 1:1 with the
//! desktop app without hand-transcribing ~400 entries.
//!
//! Regenerate the snapshot after editing any `*ProviderPresets.ts`:
//!
//! ```sh
//! npx vitest run scripts/genProviderPresets.test.ts
//! ```

use crate::provider::{AppType, Provider, ProviderCategory, ProviderMeta};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Embedded JSON snapshot: `{ "claude": [...], "codex": [...], ... }`.
const PRESETS_JSON: &str = include_str!("../assets/provider_presets.json");

/// Parsed snapshot, keyed by AppType string (`claude`, `claude-desktop`, ...).
fn snapshot() -> &'static HashMap<String, Vec<ProviderPreset>> {
    static CACHE: OnceLock<HashMap<String, Vec<ProviderPreset>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let root: HashMap<String, Vec<Value>> =
            serde_json::from_str(PRESETS_JSON).expect("embedded provider_presets.json is valid");
        root.into_iter()
            .map(|(app, list)| {
                let presets = list.into_iter().map(ProviderPreset::from_value).collect();
                (app, presets)
            })
            .collect()
    })
}

/// A single provider preset. Wraps the raw JSON entry so no field from the
/// desktop catalog is lost, while exposing typed accessors for the handful of
/// fields the CLI/TUI form needs.
#[derive(Debug, Clone)]
pub struct ProviderPreset {
    raw: Value,
}

/// One editable template variable (e.g. an endpoint id substituted into the
/// config via `${VAR}` placeholders).
#[derive(Debug, Clone)]
pub struct TemplateField {
    pub key: String,
    pub label: String,
    pub placeholder: String,
    pub default_value: String,
}

impl ProviderPreset {
    fn from_value(raw: Value) -> Self {
        Self { raw }
    }

    fn str_field(&self, key: &str) -> Option<&str> {
        self.raw.get(key).and_then(Value::as_str)
    }

    fn bool_field(&self, key: &str) -> bool {
        self.raw.get(key).and_then(Value::as_bool).unwrap_or(false)
    }

    /// Display name (English `name`; localization keys are ignored in the CLI).
    pub fn name(&self) -> &str {
        self.str_field("name").unwrap_or("")
    }

    pub fn name_key(&self) -> Option<&str> {
        self.str_field("nameKey")
    }

    pub fn website_url(&self) -> Option<&str> {
        self.str_field("websiteUrl")
    }

    /// Dedicated "get an API key" link, when the preset provides one.
    pub fn api_key_url(&self) -> Option<&str> {
        self.str_field("apiKeyUrl")
    }

    pub fn category(&self) -> ProviderCategory {
        self.str_field("category")
            .and_then(parse_category)
            .unwrap_or(ProviderCategory::Custom)
    }

    pub fn is_official(&self) -> bool {
        self.bool_field("isOfficial")
    }

    pub fn is_partner(&self) -> bool {
        self.bool_field("isPartner")
    }

    pub fn icon(&self) -> Option<&str> {
        self.str_field("icon")
    }

    pub fn icon_color(&self) -> Option<&str> {
        self.str_field("iconColor")
    }

    pub fn api_format(&self) -> Option<&str> {
        self.str_field("apiFormat")
    }

    /// Env var the API key is written to (Claude family). Defaults to
    /// `ANTHROPIC_AUTH_TOKEN` when unspecified.
    pub fn api_key_field(&self) -> &str {
        self.str_field("apiKeyField")
            .unwrap_or("ANTHROPIC_AUTH_TOKEN")
    }

    pub fn provider_type(&self) -> Option<&str> {
        self.str_field("providerType")
    }

    pub fn requires_oauth(&self) -> bool {
        self.bool_field("requiresOAuth")
    }

    /// Whether the preset authenticates via OAuth / a managed account rather
    /// than a pasted API key (GitHub Copilot, Codex-via-ChatGPT, ...).
    pub fn uses_oauth(&self) -> bool {
        self.requires_oauth()
            || matches!(self.provider_type(), Some("github_copilot") | Some("codex_oauth"))
    }

    /// Candidate request endpoints (for address management / speed test).
    pub fn endpoint_candidates(&self) -> Vec<&str> {
        self.raw
            .get("endpointCandidates")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// Editable template variables, in declaration order.
    pub fn template_fields(&self) -> Vec<TemplateField> {
        let Some(obj) = self.raw.get("templateValues").and_then(Value::as_object) else {
            return Vec::new();
        };
        obj.iter()
            .map(|(key, cfg)| TemplateField {
                key: key.clone(),
                label: cfg
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or(key)
                    .to_string(),
                placeholder: cfg
                    .get("placeholder")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                default_value: cfg
                    .get("defaultValue")
                    .and_then(Value::as_str)
                    .or_else(|| cfg.get("editorValue").and_then(Value::as_str))
                    .unwrap_or("")
                    .to_string(),
            })
            .collect()
    }

    /// Best-effort endpoint to display / prefill, resolved across the differing
    /// per-app shapes (`baseUrl` / `baseURL` / first candidate / env base url).
    pub fn endpoint(&self) -> Option<String> {
        if let Some(v) = self.str_field("baseUrl").or_else(|| self.str_field("baseURL")) {
            return Some(v.to_string());
        }
        if let Some(first) = self.endpoint_candidates().first() {
            return Some((*first).to_string());
        }
        // Claude / Gemini family keep the base url inside the env block.
        let env = self.raw.get("settingsConfig").and_then(|s| s.get("env"))?;
        for key in ["ANTHROPIC_BASE_URL", "GOOGLE_GEMINI_BASE_URL", "OPENAI_BASE_URL"] {
            if let Some(v) = env.get(key).and_then(Value::as_str) {
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
        None
    }

    /// The whole raw preset entry (for callers that need fields not surfaced
    /// by the typed accessors).
    pub fn raw(&self) -> &Value {
        &self.raw
    }

    /// Build a [`Provider`] from this preset, substituting `${VAR}` template
    /// placeholders and injecting the user-supplied API key into the
    /// app-appropriate slot. `sort_index` is supplied by the caller.
    pub fn to_provider(
        &self,
        api_key: Option<&str>,
        templates: &HashMap<String, String>,
        sort_index: i32,
    ) -> Provider {
        // Start from the per-app config blob, preferring the Claude-style
        // `settingsConfig`; Codex stores `auth` + `config` instead.
        let mut settings = self
            .raw
            .get("settingsConfig")
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        // Substitute ${VAR} placeholders throughout all string leaves.
        if !templates.is_empty() {
            substitute_templates(&mut settings, templates);
        }

        // Inject the API key.
        if let Some(key) = api_key.filter(|k| !k.is_empty()) {
            inject_api_key(&mut settings, self.api_key_field(), key);
        }

        // Carry Codex's auth/config and Claude Desktop's routing data so the
        // entry is not silently lossy, even though the CLI does not yet act on
        // every app's live config.
        if let Some(obj) = settings.as_object_mut() {
            for carry in ["auth", "config", "modelRoutes", "mode", "model"] {
                if let Some(v) = self.raw.get(carry) {
                    let mut v = v.clone();
                    if !templates.is_empty() {
                        substitute_templates(&mut v, templates);
                    }
                    if carry == "auth" {
                        if let (Some(key), Some(auth)) =
                            (api_key.filter(|k| !k.is_empty()), v.as_object_mut())
                        {
                            auth.insert(
                                "OPENAI_API_KEY".to_string(),
                                Value::String(key.to_string()),
                            );
                        }
                    }
                    obj.insert(carry.to_string(), v);
                }
            }
            // Surface a top-level base_url for list display / Claude live switch.
            if !obj.contains_key("base_url") {
                if let Some(ep) = self.endpoint() {
                    obj.insert("base_url".to_string(), Value::String(ep));
                }
            }
        }

        let mut meta = default_meta();
        meta.api_format = self.api_format().and_then(parse_api_format);
        meta.api_key_field = Some(self.api_key_field().to_string());
        meta.provider_type = self.provider_type().and_then(parse_provider_type);

        Provider {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.name().to_string(),
            settings_config: settings,
            website_url: self.website_url().map(str::to_string),
            category: self.category(),
            created_at: chrono::Utc::now().timestamp(),
            sort_index,
            notes: None,
            is_partner: self.is_partner(),
            meta,
            icon: self.icon().map(str::to_string),
            icon_color: self.icon_color().map(str::to_string),
            in_failover_queue: false,
        }
    }
}

/// Presets applicable to the given app, in catalog order.
pub fn presets_for_app(app: AppType) -> &'static [ProviderPreset] {
    snapshot()
        .get(app.as_str())
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Replace every `${KEY}` occurrence inside string leaves of `value`.
fn substitute_templates(value: &mut Value, templates: &HashMap<String, String>) {
    match value {
        Value::String(s) => {
            for (k, v) in templates {
                let needle = format!("${{{}}}", k);
                if s.contains(&needle) {
                    *s = s.replace(&needle, v);
                }
            }
        }
        Value::Array(arr) => arr.iter_mut().for_each(|v| substitute_templates(v, templates)),
        Value::Object(obj) => obj
            .values_mut()
            .for_each(|v| substitute_templates(v, templates)),
        _ => {}
    }
}

/// Write `key` into the env block (Claude family) under `field`. If no env
/// block exists, fall back to a top-level `api_key`.
fn inject_api_key(settings: &mut Value, field: &str, key: &str) {
    if let Some(env) = settings.get_mut("env").and_then(Value::as_object_mut) {
        env.insert(field.to_string(), Value::String(key.to_string()));
        return;
    }
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("api_key".to_string(), Value::String(key.to_string()));
    }
}

fn parse_category(s: &str) -> Option<ProviderCategory> {
    Some(match s {
        "official" => ProviderCategory::Official,
        "cn_official" => ProviderCategory::CnOfficial,
        "cloud_provider" => ProviderCategory::CloudProvider,
        "aggregator" => ProviderCategory::Aggregator,
        "third_party" => ProviderCategory::ThirdParty,
        "custom" => ProviderCategory::Custom,
        "omo" => ProviderCategory::Omo,
        "omo-slim" => ProviderCategory::OmoSlim,
        _ => return None,
    })
}

fn parse_api_format(s: &str) -> Option<crate::provider::ClaudeApiFormat> {
    use crate::provider::ClaudeApiFormat::*;
    Some(match s {
        "anthropic" => Anthropic,
        "openai_chat" => OpenAiChat,
        "openai_responses" => OpenAiResponses,
        "gemini_native" => GeminiNative,
        _ => return None,
    })
}

fn parse_provider_type(s: &str) -> Option<crate::provider::ProviderType> {
    use crate::provider::ProviderType::*;
    Some(match s {
        "claude" => Claude,
        "claude_auth" => ClaudeAuth,
        "codex" => Codex,
        "gemini" => Gemini,
        "gemini_cli" => GeminiCli,
        "openrouter" => OpenRouter,
        "github_copilot" => GitHubCopilot,
        "codex_oauth" => CodexOauth,
        _ => return None,
    })
}

fn default_meta() -> ProviderMeta {
    ProviderMeta {
        custom_endpoints: None,
        common_config_enabled: true,
        claude_desktop_mode: None,
        claude_desktop_model_routes: None,
        usage_script: None,
        endpoint_auto_select: None,
        is_partner: None,
        partner_promotion_key: None,
        cost_multiplier: None,
        pricing_model_source: None,
        api_format: None,
        auth_binding: None,
        api_key_field: None,
        is_full_url: true,
        prompt_cache_key: None,
        codex_fast_mode: None,
        codex_chat_reasoning: None,
        provider_type: None,
        github_account_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_app_group_loads_non_empty() {
        for app in [
            AppType::Claude,
            AppType::ClaudeDesktop,
            AppType::Codex,
            AppType::Gemini,
            AppType::OpenCode,
            AppType::OpenClaw,
            AppType::Hermes,
        ] {
            assert!(
                !presets_for_app(app).is_empty(),
                "presets for {:?} should be non-empty",
                app
            );
        }
    }

    #[test]
    fn claude_official_is_first_and_official() {
        let p = &presets_for_app(AppType::Claude)[0];
        assert_eq!(p.name(), "Claude Official");
        assert!(p.is_official());
        assert_eq!(p.category(), ProviderCategory::Official);
    }

    #[test]
    fn deepseek_preset_prefills_endpoint_and_injects_key() {
        let deepseek = presets_for_app(AppType::Claude)
            .iter()
            .find(|p| p.name() == "DeepSeek")
            .expect("DeepSeek preset present");
        assert_eq!(
            deepseek.endpoint().as_deref(),
            Some("https://api.deepseek.com/anthropic")
        );

        let provider = deepseek.to_provider(Some("sk-test"), &HashMap::new(), 1);
        assert_eq!(provider.name, "DeepSeek");
        assert_eq!(provider.category, ProviderCategory::CnOfficial);
        assert_eq!(
            provider.settings_config["env"]["ANTHROPIC_AUTH_TOKEN"].as_str(),
            Some("sk-test")
        );
        assert_eq!(
            provider.settings_config["env"]["ANTHROPIC_BASE_URL"].as_str(),
            Some("https://api.deepseek.com/anthropic")
        );
    }

    #[test]
    fn template_placeholders_are_substituted() {
        let kat = presets_for_app(AppType::Claude)
            .iter()
            .find(|p| p.name() == "KAT-Coder")
            .expect("KAT-Coder preset present");
        let fields = kat.template_fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].key, "ENDPOINT_ID");

        let mut tmpl = HashMap::new();
        tmpl.insert("ENDPOINT_ID".to_string(), "ep-abc-123".to_string());
        let provider = kat.to_provider(Some("sk-k"), &tmpl, 1);
        let base = provider.settings_config["env"]["ANTHROPIC_BASE_URL"]
            .as_str()
            .unwrap();
        assert!(base.contains("ep-abc-123"), "got {base}");
        assert!(!base.contains("${ENDPOINT_ID}"));
    }

    #[test]
    fn codex_preset_injects_openai_key_into_auth() {
        let preset = presets_for_app(AppType::Codex)
            .iter()
            .find(|p| p.raw().get("auth").is_some())
            .expect("a codex preset with auth block");
        let provider = preset.to_provider(Some("sk-oai"), &HashMap::new(), 1);
        assert_eq!(
            provider.settings_config["auth"]["OPENAI_API_KEY"].as_str(),
            Some("sk-oai")
        );
    }
}
