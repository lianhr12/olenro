//! TUI state management

use olenro_core::config::get_cli_database_path;
use olenro_core::provider::ProviderCategory;
use std::path::PathBuf;

use olenro_core::proxy::ProxyConfig;
use olenro_core::services::mcp::McpService;
use olenro_core::services::prompt::PromptService;
use olenro_core::services::provider::ProviderService;
use olenro_core::services::proxy::ProxyService;
use olenro_core::services::skill::SkillService;
use olenro_core::services::usage::UsageService;
use olenro_core::session_manager::SessionManager;

/// Tabs in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Providers,
    Mcp,
    Prompts,
    Skills,
    Sessions,
    Usage,
    Proxy,
    Settings,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::Providers => Tab::Mcp,
            Tab::Mcp => Tab::Prompts,
            Tab::Prompts => Tab::Skills,
            Tab::Skills => Tab::Sessions,
            Tab::Sessions => Tab::Usage,
            Tab::Usage => Tab::Proxy,
            Tab::Proxy => Tab::Settings,
            Tab::Settings => Tab::Providers,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Tab::Providers => Tab::Settings,
            Tab::Mcp => Tab::Providers,
            Tab::Prompts => Tab::Mcp,
            Tab::Skills => Tab::Prompts,
            Tab::Sessions => Tab::Skills,
            Tab::Usage => Tab::Sessions,
            Tab::Proxy => Tab::Usage,
            Tab::Settings => Tab::Proxy,
        }
    }
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
    pub selected_index: usize,
    pub dialog_mode: DialogMode,
    pub input_field: usize,
    pub provider_service: ProviderService,
    pub mcp_service: McpService,
    pub prompt_service: PromptService,
    pub skill_service: SkillService,
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
}

impl TuiState {
    pub fn new(runtime: tokio::runtime::Handle) -> Self {
        let db_path = get_cli_database_path();
        let provider_service = ProviderService::new(db_path.clone());
        let mcp_service = McpService::new(db_path.clone());
        let prompt_service = PromptService::new(db_path.clone());
        let skill_service = SkillService::new(db_path.clone());
        let usage_service = UsageService::new(db_path.clone());
        let proxy_service = ProxyService::new_with_db(ProxyConfig::default(), db_path.clone());
        let session_manager = SessionManager::new();

        Self {
            current_tab: Tab::Providers,
            selected_index: 0,
            dialog_mode: DialogMode::None,
            input_field: 0,
            provider_service,
            mcp_service,
            prompt_service,
            skill_service,
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
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = self.current_tab.next();
        self.selected_index = 0;
        self.close_dialog();
    }

    pub fn prev_tab(&mut self) {
        self.current_tab = self.current_tab.prev();
        self.selected_index = 0;
        self.close_dialog();
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
