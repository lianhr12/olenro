//! TUI state management

use olenro_core::config::get_cli_database_path;
use olenro_core::provider::{AppType, ProviderCategory};
use olenro_core::provider_presets::{self, ProviderPreset};
use std::path::PathBuf;

use olenro_core::proxy::ProxyConfig;
use olenro_core::services::mcp::McpService;
use olenro_core::services::prompt::PromptService;
use olenro_core::services::provider::ProviderService;
use olenro_core::services::proxy::ProxyService;
use olenro_core::services::skill::SkillService;
use olenro_core::services::universal::UniversalService;
use olenro_core::services::usage::UsageService;
use olenro_core::session_manager::SessionManager;

/// Tabs in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Providers,
    Mcp,
    Prompts,
    Skills,
    Discover,
    Agents,
    Sessions,
    Usage,
    Proxy,
    Universal,
    // OpenClaw-specific panels (visible only when managing OpenClaw).
    Workspace,
    OpenClawEnv,
    OpenClawTools,
    OpenClawAgents,
    // Hermes-specific panel (visible only when managing Hermes).
    HermesMemory,
    Settings,
}

impl Tab {
    /// All tabs in display order.
    pub const ALL: [Tab; 17] = [
        Tab::Dashboard,
        Tab::Providers,
        Tab::Mcp,
        Tab::Prompts,
        Tab::Skills,
        Tab::Discover,
        Tab::Agents,
        Tab::Sessions,
        Tab::Usage,
        Tab::Proxy,
        Tab::Universal,
        Tab::Workspace,
        Tab::OpenClawEnv,
        Tab::OpenClawTools,
        Tab::OpenClawAgents,
        Tab::HermesMemory,
        Tab::Settings,
    ];

    /// Whether this tab is applicable to the given app.
    ///
    /// Mirrors the desktop sidebar visibility rules (`navConfig.ts` +
    /// `App.tsx` capability flags): MCP/Skills are hidden for OpenClaw,
    /// Prompts is hidden for OpenClaw and Hermes, and Sessions is hidden for
    /// Claude Desktop. Providers/Usage/Proxy/Settings are always available.
    pub fn visible_for(self, app: AppType) -> bool {
        match self {
            Tab::Mcp | Tab::Skills | Tab::Discover => app != AppType::OpenClaw,
            Tab::Prompts => app != AppType::OpenClaw && app != AppType::Hermes,
            // Subagents are a Claude-family feature (mirrors desktop isClaudeFamily).
            Tab::Agents => app == AppType::Claude || app == AppType::ClaudeDesktop,
            Tab::Sessions => app != AppType::ClaudeDesktop,
            Tab::Workspace | Tab::OpenClawEnv | Tab::OpenClawTools | Tab::OpenClawAgents => {
                app == AppType::OpenClaw
            }
            Tab::HermesMemory => app == AppType::Hermes,
            Tab::Dashboard
            | Tab::Providers
            | Tab::Usage
            | Tab::Proxy
            | Tab::Universal
            | Tab::Settings => true,
        }
    }
}

/// Apps the TUI can manage, in switcher order (mirrors the desktop AppSwitcher).
pub const APPS: [(AppType, &str); 7] = [
    (AppType::Claude, "Claude"),
    (AppType::ClaudeDesktop, "Claude Desktop"),
    (AppType::Codex, "Codex"),
    (AppType::Gemini, "Gemini"),
    (AppType::OpenCode, "OpenCode"),
    (AppType::OpenClaw, "OpenClaw"),
    (AppType::Hermes, "Hermes"),
];

/// Stage of the two-step "Add Provider" dialog: first pick a preset (or
/// "Custom"), then fill in the (possibly preset-prefilled) form fields. Mirrors
/// the desktop AddProviderDialog → ProviderForm flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddProviderStage {
    SelectPreset,
    Fields,
}

/// Dialog mode for input forms
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogMode {
    None,
    AddProvider,
    EditProvider(String),
    DeleteProvider(String),
    AddMcp,
    DeleteMcp(String),
    AddPrompt,
    EditPrompt(String),
    DeletePrompt(String),
    AddSkill,
    DeleteSkill(String),
    DeleteSession(String),
    // OpenClaw env / agents editing
    AddOpenClawEnv,
    DeleteOpenClawEnv(String),
    SetOpenClawPrimaryModel,
    // OpenClaw workspace
    DeleteOpenClawMemory(String),
    // Universal providers
    AddUniversal,
    EditUniversal(String),
    DeleteUniversal(String),
    // Claude subagents
    AddAgent,
    DeleteAgent(String, String), // (scope_label, name)
    // skills.sh discovery
    SearchSkills,
    // git-sync
    ConfigureGitSync,
}

impl DialogMode {
    /// Number of editable input fields for this dialog (used for field navigation)
    pub fn field_count(&self) -> usize {
        match self {
            DialogMode::AddProvider => 4,     // name, endpoint, api_key, category
            DialogMode::EditProvider(_) => 2, // name, endpoint
            DialogMode::AddMcp => 3,          // name, command, args
            DialogMode::AddPrompt => 2,       // name, content
            DialogMode::EditPrompt(_) => 1,   // content
            DialogMode::AddSkill => 1,        // repo
            DialogMode::AddOpenClawEnv => 2,  // key, value
            DialogMode::SetOpenClawPrimaryModel => 1, // model
            DialogMode::AddUniversal => 3,    // name, base_url, api_key
            DialogMode::EditUniversal(_) => 2, // name, base_url
            DialogMode::AddAgent => 2,        // name, description
            DialogMode::SearchSkills => 1,    // query
            DialogMode::ConfigureGitSync => 2, // remote url, branch
            _ => 0,
        }
    }

    pub fn is_confirm(&self) -> bool {
        matches!(
            self,
            DialogMode::DeleteProvider(_)
                | DialogMode::DeleteMcp(_)
                | DialogMode::DeletePrompt(_)
                | DialogMode::DeleteSkill(_)
                | DialogMode::DeleteSession(_)
                | DialogMode::DeleteOpenClawEnv(_)
                | DialogMode::DeleteOpenClawMemory(_)
                | DialogMode::DeleteUniversal(_)
                | DialogMode::DeleteAgent(_, _)
        )
    }
}

/// Provider categories selectable in the add dialog
pub const CATEGORIES: [(&str, ProviderCategory); 6] = [
    ("custom", ProviderCategory::Custom),
    ("official", ProviderCategory::Official),
    ("cn_official", ProviderCategory::CnOfficial),
    ("cloud_provider", ProviderCategory::CloudProvider),
    ("aggregator", ProviderCategory::Aggregator),
    ("third_party", ProviderCategory::ThirdParty),
];

/// TUI application state
pub struct TuiState {
    pub current_tab: Tab,
    /// The app whose configuration the TUI is currently managing.
    pub active_app: AppType,
    pub selected_index: usize,
    pub dialog_mode: DialogMode,
    pub input_field: usize,
    pub provider_service: ProviderService,
    pub mcp_service: McpService,
    pub prompt_service: PromptService,
    pub skill_service: SkillService,
    pub universal_service: UniversalService,
    pub usage_service: UsageService,
    pub proxy_service: ProxyService,
    pub session_manager: SessionManager,
    pub runtime: tokio::runtime::Handle,
    pub db_path: PathBuf,
    pub status_message: String,
    // Input fields for dialogs
    pub name_input: String,
    pub endpoint_input: String,
    pub api_key_input: String,
    pub category_index: usize,
    pub command_input: String,
    pub args_input: String,
    pub content_input: String,
    // Add Provider dialog: preset selection + dynamic template fields.
    pub add_stage: AddProviderStage,
    /// Cursor in the preset-selection list (index 0 = "Custom", then presets).
    pub preset_cursor: usize,
    /// Chosen preset index into `presets_for_app(active_app)`; None = Custom.
    pub chosen_preset: Option<usize>,
    /// One input value per template variable of the chosen preset.
    pub template_inputs: Vec<String>,
    /// Explicit OpenClaw provider key (only collected when managing OpenClaw).
    pub provider_key_input: String,
    // skills.sh discovery
    pub discover_results: Vec<olenro_core::skills_sh::SkillsShDiscoverableSkill>,
    pub discover_loaded: bool,
    pub discover_label: String,
}

impl TuiState {
    pub fn new(runtime: tokio::runtime::Handle) -> Self {
        let db_path = get_cli_database_path();
        let provider_service = ProviderService::new(db_path.clone());
        let mcp_service = McpService::new(db_path.clone());
        let prompt_service = PromptService::new(db_path.clone());
        let skill_service = SkillService::new(db_path.clone());
        let universal_service = UniversalService::new(db_path.clone());
        let usage_service = UsageService::new(db_path.clone());
        let proxy_service = ProxyService::new_with_db(ProxyConfig::default(), db_path.clone());
        let session_manager = SessionManager::new();

        Self {
            current_tab: Tab::Dashboard,
            active_app: AppType::Claude,
            selected_index: 0,
            dialog_mode: DialogMode::None,
            input_field: 0,
            provider_service,
            mcp_service,
            prompt_service,
            skill_service,
            universal_service,
            usage_service,
            proxy_service,
            session_manager,
            runtime,
            db_path,
            status_message: String::new(),
            name_input: String::new(),
            endpoint_input: String::new(),
            api_key_input: String::new(),
            category_index: 0,
            command_input: String::new(),
            args_input: String::new(),
            content_input: String::new(),
            add_stage: AddProviderStage::SelectPreset,
            preset_cursor: 0,
            chosen_preset: None,
            template_inputs: Vec::new(),
            provider_key_input: String::new(),
            discover_results: Vec::new(),
            discover_loaded: false,
            discover_label: String::new(),
        }
    }

    /// Tabs applicable to the current app, in display order.
    pub fn visible_tabs(&self) -> Vec<Tab> {
        Tab::ALL
            .into_iter()
            .filter(|t| t.visible_for(self.active_app))
            .collect()
    }

    pub fn next_tab(&mut self) {
        let tabs = self.visible_tabs();
        let pos = tabs.iter().position(|t| *t == self.current_tab).unwrap_or(0);
        self.current_tab = tabs[(pos + 1) % tabs.len()];
        self.selected_index = 0;
        self.close_dialog();
    }

    pub fn prev_tab(&mut self) {
        let tabs = self.visible_tabs();
        let pos = tabs.iter().position(|t| *t == self.current_tab).unwrap_or(0);
        self.current_tab = tabs[(pos + tabs.len() - 1) % tabs.len()];
        self.selected_index = 0;
        self.close_dialog();
    }

    /// Cycle the active app (forward/backward) and keep the current tab valid.
    pub fn cycle_app(&mut self, forward: bool) {
        let len = APPS.len();
        let pos = APPS
            .iter()
            .position(|(a, _)| *a == self.active_app)
            .unwrap_or(0);
        let next = if forward {
            (pos + 1) % len
        } else {
            (pos + len - 1) % len
        };
        self.active_app = APPS[next].0;
        // If the current tab is not applicable to the new app, fall back to the
        // first visible tab (Providers is always visible).
        if !self.current_tab.visible_for(self.active_app) {
            self.current_tab = self.visible_tabs().first().copied().unwrap_or(Tab::Providers);
        }
        self.selected_index = 0;
        self.close_dialog();
        let name = APPS[next].1;
        self.set_status(format!("Managing app: {}", name));
    }

    /// Display name for the active app.
    pub fn active_app_name(&self) -> &'static str {
        APPS.iter()
            .find(|(a, _)| *a == self.active_app)
            .map(|(_, n)| *n)
            .unwrap_or("Claude")
    }

    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_selection_down(&mut self, max: usize) {
        if max > 0 && self.selected_index + 1 < max {
            self.selected_index += 1;
        }
    }

    pub fn close_dialog(&mut self) {
        self.dialog_mode = DialogMode::None;
        self.input_field = 0;
        self.name_input.clear();
        self.endpoint_input.clear();
        self.api_key_input.clear();
        self.command_input.clear();
        self.args_input.clear();
        self.content_input.clear();
        self.category_index = 0;
        self.add_stage = AddProviderStage::SelectPreset;
        self.preset_cursor = 0;
        self.chosen_preset = None;
        self.template_inputs.clear();
        self.provider_key_input.clear();
    }

    /// Whether the Add Provider form should collect an explicit provider key.
    /// Mirrors the desktop app, where OpenClaw requires a `providerKey` form
    /// field used as `models.providers.<key>`.
    pub fn add_provider_wants_key(&self) -> bool {
        self.active_app == AppType::OpenClaw
    }

    /// Presets applicable to the app currently being managed.
    pub fn add_provider_presets(&self) -> &'static [ProviderPreset] {
        provider_presets::presets_for_app(self.active_app)
    }

    /// Number of selectable rows in the preset picker (row 0 = "Custom").
    pub fn preset_row_count(&self) -> usize {
        self.add_provider_presets().len() + 1
    }

    /// The chosen preset, if any (None = "Custom").
    pub fn chosen_preset(&self) -> Option<&'static ProviderPreset> {
        self.chosen_preset.map(|i| &self.add_provider_presets()[i])
    }

    /// Number of editable fields in the Fields stage of Add Provider.
    /// Custom: name, endpoint, api key, category. Preset: name, endpoint,
    /// api key, then one field per template variable. When the active app
    /// needs an explicit provider key (OpenClaw), one extra trailing field is
    /// appended.
    pub fn add_provider_field_count(&self) -> usize {
        let base = match self.chosen_preset() {
            None => 4,
            Some(p) => 3 + p.template_fields().len(),
        };
        base + if self.add_provider_wants_key() { 1 } else { 0 }
    }

    /// Index of the provider-key field, if present (always the last field).
    pub fn provider_key_field_index(&self) -> Option<usize> {
        if self.add_provider_wants_key() {
            Some(self.add_provider_field_count() - 1)
        } else {
            None
        }
    }

    /// Apply the picker selection: load the chosen preset and prefill the form,
    /// then advance to the Fields stage.
    pub fn select_preset(&mut self) {
        if self.preset_cursor == 0 {
            // "Custom" — blank free-form form (same as the legacy add flow).
            self.chosen_preset = None;
            self.name_input.clear();
            self.endpoint_input.clear();
            self.api_key_input.clear();
            self.template_inputs.clear();
            self.category_index = 0;
        } else {
            let idx = self.preset_cursor - 1;
            let preset = &self.add_provider_presets()[idx];
            self.name_input = preset.name().to_string();
            self.endpoint_input = preset.endpoint().unwrap_or_default();
            self.api_key_input.clear();
            self.template_inputs = preset
                .template_fields()
                .iter()
                .map(|f| f.default_value.clone())
                .collect();
            self.chosen_preset = Some(idx);
        }
        // Prefill the OpenClaw provider key with a slug of the name as a hint.
        if self.add_provider_wants_key() {
            self.provider_key_input =
                olenro_core::services::provider::slugify_provider_key(&self.name_input);
        }
        self.add_stage = AddProviderStage::Fields;
        self.input_field = 0;
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
    }

    /// Push a char into the currently active input field
    pub fn push_input_char(&mut self, c: char) {
        if self.dialog_mode == DialogMode::AddProvider {
            // The provider-key field (when present) is always the last field.
            if self.provider_key_field_index() == Some(self.input_field) {
                self.provider_key_input.push(c);
                return;
            }
            match self.input_field {
                0 => self.name_input.push(c),
                1 => self.endpoint_input.push(c),
                2 => self.api_key_input.push(c),
                // For Custom, field 3 is the category selector (left/right only).
                // For a preset, fields >= 3 are template variables.
                n => {
                    if let Some(buf) = self.template_inputs.get_mut(n - 3) {
                        buf.push(c);
                    }
                }
            }
            return;
        }
        match (&self.dialog_mode, self.input_field) {
            (DialogMode::EditProvider(_), 0) => self.name_input.push(c),
            (DialogMode::EditProvider(_), 1) => self.endpoint_input.push(c),
            (DialogMode::AddMcp, 0) => self.name_input.push(c),
            (DialogMode::AddMcp, 1) => self.command_input.push(c),
            (DialogMode::AddMcp, 2) => self.args_input.push(c),
            (DialogMode::AddPrompt, 0) => self.name_input.push(c),
            (DialogMode::AddPrompt, 1) => self.content_input.push(c),
            (DialogMode::EditPrompt(_), 0) => self.content_input.push(c),
            (DialogMode::AddSkill, 0) => self.name_input.push(c),
            (DialogMode::AddOpenClawEnv, 0) => self.name_input.push(c),
            (DialogMode::AddOpenClawEnv, 1) => self.content_input.push(c),
            (DialogMode::SetOpenClawPrimaryModel, 0) => self.content_input.push(c),
            (DialogMode::AddUniversal, 0) => self.name_input.push(c),
            (DialogMode::AddUniversal, 1) => self.endpoint_input.push(c),
            (DialogMode::AddUniversal, 2) => self.api_key_input.push(c),
            (DialogMode::EditUniversal(_), 0) => self.name_input.push(c),
            (DialogMode::EditUniversal(_), 1) => self.endpoint_input.push(c),
            (DialogMode::AddAgent, 0) => self.name_input.push(c),
            (DialogMode::AddAgent, 1) => self.content_input.push(c),
            (DialogMode::SearchSkills, 0) => self.name_input.push(c),
            (DialogMode::ConfigureGitSync, 0) => self.endpoint_input.push(c),
            (DialogMode::ConfigureGitSync, 1) => self.name_input.push(c),
            _ => {}
        }
    }

    /// Remove the last char from the currently active input field
    pub fn pop_input_char(&mut self) {
        if self.dialog_mode == DialogMode::AddProvider {
            if self.provider_key_field_index() == Some(self.input_field) {
                self.provider_key_input.pop();
                return;
            }
            match self.input_field {
                0 => {
                    self.name_input.pop();
                }
                1 => {
                    self.endpoint_input.pop();
                }
                2 => {
                    self.api_key_input.pop();
                }
                n => {
                    if let Some(buf) = self.template_inputs.get_mut(n - 3) {
                        buf.pop();
                    }
                }
            }
            return;
        }
        match (&self.dialog_mode, self.input_field) {
            (DialogMode::EditProvider(_), 0) => {
                self.name_input.pop();
            }
            (DialogMode::EditProvider(_), 1) => {
                self.endpoint_input.pop();
            }
            (DialogMode::AddMcp, 0) => {
                self.name_input.pop();
            }
            (DialogMode::AddMcp, 1) => {
                self.command_input.pop();
            }
            (DialogMode::AddMcp, 2) => {
                self.args_input.pop();
            }
            (DialogMode::AddPrompt, 0) => {
                self.name_input.pop();
            }
            (DialogMode::AddPrompt, 1) => {
                self.content_input.pop();
            }
            (DialogMode::EditPrompt(_), 0) => {
                self.content_input.pop();
            }
            (DialogMode::AddSkill, 0) => {
                self.name_input.pop();
            }
            (DialogMode::AddOpenClawEnv, 0) => {
                self.name_input.pop();
            }
            (DialogMode::AddOpenClawEnv, 1) => {
                self.content_input.pop();
            }
            (DialogMode::SetOpenClawPrimaryModel, 0) => {
                self.content_input.pop();
            }
            (DialogMode::AddUniversal, 0) => {
                self.name_input.pop();
            }
            (DialogMode::AddUniversal, 1) => {
                self.endpoint_input.pop();
            }
            (DialogMode::AddUniversal, 2) => {
                self.api_key_input.pop();
            }
            (DialogMode::EditUniversal(_), 0) => {
                self.name_input.pop();
            }
            (DialogMode::EditUniversal(_), 1) => {
                self.endpoint_input.pop();
            }
            (DialogMode::AddAgent, 0) => {
                self.name_input.pop();
            }
            (DialogMode::AddAgent, 1) => {
                self.content_input.pop();
            }
            (DialogMode::SearchSkills, 0) => {
                self.name_input.pop();
            }
            (DialogMode::ConfigureGitSync, 0) => {
                self.endpoint_input.pop();
            }
            (DialogMode::ConfigureGitSync, 1) => {
                self.name_input.pop();
            }
            _ => {}
        }
    }

    /// Cycle category selection (left=-1, right=+1) when on the category field
    pub fn cycle_category(&mut self, forward: bool) {
        if self.dialog_mode == DialogMode::AddProvider
            && self.chosen_preset.is_none()
            && self.input_field == 3
        {
            let len = CATEGORIES.len();
            if forward {
                self.category_index = (self.category_index + 1) % len;
            } else {
                self.category_index = (self.category_index + len - 1) % len;
            }
        }
    }
}
