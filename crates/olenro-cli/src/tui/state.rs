//! TUI state management

use olenro_core::config::get_cli_database_path;
use olenro_core::provider::{AppType, ProviderCategory};
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
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
    }

    /// Push a char into the currently active input field
    pub fn push_input_char(&mut self, c: char) {
        match (&self.dialog_mode, self.input_field) {
            (DialogMode::AddProvider, 0) => self.name_input.push(c),
            (DialogMode::AddProvider, 1) => self.endpoint_input.push(c),
            (DialogMode::AddProvider, 2) => self.api_key_input.push(c),
            // field 3 is the category selector (handled via left/right)
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
            _ => {}
        }
    }

    /// Remove the last char from the currently active input field
    pub fn pop_input_char(&mut self) {
        match (&self.dialog_mode, self.input_field) {
            (DialogMode::AddProvider, 0) => {
                self.name_input.pop();
            }
            (DialogMode::AddProvider, 1) => {
                self.endpoint_input.pop();
            }
            (DialogMode::AddProvider, 2) => {
                self.api_key_input.pop();
            }
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
            _ => {}
        }
    }

    /// Cycle category selection (left=-1, right=+1) when on the category field
    pub fn cycle_category(&mut self, forward: bool) {
        if self.dialog_mode == DialogMode::AddProvider && self.input_field == 3 {
            let len = CATEGORIES.len();
            if forward {
                self.category_index = (self.category_index + 1) % len;
            } else {
                self.category_index = (self.category_index + len - 1) % len;
            }
        }
    }
}
