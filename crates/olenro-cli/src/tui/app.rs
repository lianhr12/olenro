//! TUI application and main loop

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

use olenro_core::prompt::CreatePromptInput;

use crate::tui::state::{DialogMode, Tab, TuiState, CATEGORIES};
use crate::tui::ui;

/// Run the TUI application.
///
/// `runtime` is a handle to the Tokio runtime so the (synchronous) event loop
/// can drive async service calls such as starting/stopping the proxy. This must
/// be invoked from outside a runtime worker thread (e.g. via `spawn_blocking`)
/// so `Handle::block_on` does not panic.
pub fn run(runtime: tokio::runtime::Handle) -> io::Result<()> {
    // Enter raw mode + alternate screen
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = run_loop(&mut terminal, runtime);

    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    result
}

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    runtime: tokio::runtime::Handle,
) -> io::Result<()> {
    let mut state = TuiState::new(runtime);
    let mut running = true;

    while running {
        // Lazily load skills.sh popular list the first time the Discover tab is shown.
        if state.current_tab == Tab::Discover && !state.discover_loaded {
            load_discover_popular(&mut state);
        }

        terminal.draw(|f| ui::render(f, &mut state))?;

        if let Ok(Event::Key(key)) = event::read() {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if state.dialog_mode == DialogMode::None {
                running = handle_list_key(&mut state, key.code);
            } else {
                handle_dialog_key(&mut state, key.code);
            }
        }
    }

    Ok(())
}

/// Handle a key in list-navigation mode. Returns false to quit.
fn handle_list_key(state: &mut TuiState, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') | KeyCode::Char('Q') => return false,
        KeyCode::Tab => state.next_tab(),
        KeyCode::BackTab => state.prev_tab(),
        // Switch the app being managed (mirrors the desktop AppSwitcher).
        KeyCode::Char('[') => state.cycle_app(false),
        KeyCode::Char(']') => state.cycle_app(true),
        KeyCode::Up => state.move_selection_up(),
        KeyCode::Down => {
            let len = current_list_len(state);
            state.move_selection_down(len);
        }
        KeyCode::Enter => handle_enter_list(state),
        KeyCode::Char('+') | KeyCode::Char('a') => handle_add(state),
        KeyCode::Char('e') => handle_edit(state),
        KeyCode::Char('d') => handle_delete(state),
        // Proxy controls (only meaningful on the Proxy tab)
        KeyCode::Char('s') if state.current_tab == Tab::Proxy => proxy_start(state),
        KeyCode::Char('x') if state.current_tab == Tab::Proxy => proxy_stop(state),
        KeyCode::Char('t') if state.current_tab == Tab::Proxy => proxy_toggle_takeover(state),
        // Sync MCP servers to every app's config (MCP tab).
        KeyCode::Char('s') if state.current_tab == Tab::Mcp => mcp_sync_all(state),
        // Resume the selected session (Sessions tab).
        KeyCode::Char('r') if state.current_tab == Tab::Sessions => session_resume(state),
        // Cycle the OpenClaw tools profile.
        KeyCode::Char('p') if state.current_tab == Tab::OpenClawTools => {
            openclaw_cycle_profile(state)
        }
        // skills.sh discovery: search / refresh popular.
        KeyCode::Char('s') if state.current_tab == Tab::Discover => {
            state.dialog_mode = DialogMode::SearchSkills;
        }
        KeyCode::Char('r') if state.current_tab == Tab::Discover => {
            state.discover_loaded = false;
            load_discover_popular(state);
        }
        KeyCode::Char('i') if state.current_tab == Tab::Discover => discover_install(state),
        KeyCode::Char('s') => handle_enter_list(state), // switch/enable shortcut elsewhere
        _ => {}
    }
    true
}

/// Sync all stored MCP servers into every supported app's config file.
fn mcp_sync_all(state: &mut TuiState) {
    use olenro_core::app_config::AppType;
    let apps = [
        ("claude", AppType::Claude),
        ("codex", AppType::Codex),
        ("gemini", AppType::Gemini),
        ("opencode", AppType::OpenCode),
        ("hermes", AppType::Hermes),
    ];
    let mut ok = Vec::new();
    let mut failed = Vec::new();
    for (name, app) in apps.iter() {
        match state.mcp_service.sync_to_app(app) {
            Ok(_) => ok.push(*name),
            Err(_) => failed.push(*name),
        }
    }
    if failed.is_empty() {
        state.set_status(format!("Synced MCP servers → {}", ok.join(", ")));
    } else {
        state.set_status(format!(
            "Synced → {} | failed → {}",
            ok.join(", "),
            failed.join(", ")
        ));
    }
}

/// Toggle "takeover" of Claude: point its live config at the local proxy (or
/// restore the previous endpoint). Requires the proxy to be running to be useful.
fn proxy_toggle_takeover(state: &mut TuiState) {
    let app = state.active_app;
    let name = state.active_app_name();
    let currently = state.proxy_service.is_taken_over(app);
    let enable = !currently;
    match state.proxy_service.set_takeover(app, enable) {
        Ok(_) => {
            if enable {
                state.set_status(format!(
                    "{} takeover ON → routed through {}",
                    name,
                    state.proxy_service.proxy_url()
                ));
            } else {
                state.set_status(format!("{} takeover OFF → restored previous endpoint", name));
            }
        }
        Err(e) => state.set_status(format!("Takeover failed: {}", e)),
    }
}

/// Surface the command to resume the selected Claude session.
fn session_resume(state: &mut TuiState) {
    let app = state.active_app;
    let sessions = state.session_manager.list_sessions(&app).unwrap_or_default();
    if let Some(s) = sessions.get(state.selected_index) {
        let dir = s.cwd.clone().unwrap_or_else(|| ".".to_string());
        state.set_status(format!(
            "Resume: (cd {}) && {}",
            dir,
            resume_command(app, &s.id)
        ));
    } else {
        state.set_status("No session selected");
    }
}

/// Build the shell command that resumes a session for the given app.
fn resume_command(app: olenro_core::provider::AppType, id: &str) -> String {
    use olenro_core::provider::AppType;
    match app {
        AppType::Claude | AppType::ClaudeDesktop => format!("claude --resume {}", id),
        AppType::Codex => format!("codex resume {}", id),
        AppType::Gemini => format!("gemini --resume {}", id),
        AppType::OpenCode => format!("opencode --session {}", id),
        AppType::OpenClaw => format!("openclaw --resume {}", id),
        AppType::Hermes => format!("hermes --resume {}", id),
    }
}

/// Start the local proxy from within the TUI (drives the async call on the runtime).
fn proxy_start(state: &mut TuiState) {
    let handle = state.runtime.clone();
    match handle.block_on(state.proxy_service.start()) {
        Ok(_) => {
            let cfg = state.proxy_service.config();
            state.set_status(format!(
                "Proxy started on http://{}:{}",
                cfg.address, cfg.port
            ));
        }
        Err(e) => state.set_status(format!("Proxy start failed: {}", e)),
    }
}

/// Stop the local proxy from within the TUI.
fn proxy_stop(state: &mut TuiState) {
    let handle = state.runtime.clone();
    match handle.block_on(state.proxy_service.stop()) {
        Ok(_) => state.set_status("Proxy stopped"),
        Err(e) => state.set_status(format!("Proxy stop failed: {}", e)),
    }
}

/// Handle a key while a dialog is open.
fn handle_dialog_key(state: &mut TuiState, code: KeyCode) {
    // Confirmation dialogs only accept Enter (confirm) / Esc (cancel)
    if state.dialog_mode.is_confirm() {
        match code {
            KeyCode::Enter => submit_dialog(state),
            KeyCode::Esc => state.close_dialog(),
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Esc => state.close_dialog(),
        KeyCode::Tab => {
            let count = state.dialog_mode.field_count();
            if count > 0 {
                state.input_field = (state.input_field + 1) % count;
            }
        }
        KeyCode::BackTab => {
            let count = state.dialog_mode.field_count();
            if count > 0 {
                state.input_field = (state.input_field + count - 1) % count;
            }
        }
        KeyCode::Left => state.cycle_category(false),
        KeyCode::Right => state.cycle_category(true),
        KeyCode::Enter => {
            // Enter on the last field submits; otherwise advance to next field.
            let count = state.dialog_mode.field_count();
            if count > 0 && state.input_field + 1 < count {
                state.input_field += 1;
            } else {
                submit_dialog(state);
            }
        }
        KeyCode::Backspace => state.pop_input_char(),
        KeyCode::Char(c) => state.push_input_char(c),
        _ => {}
    }
}

/// The number of rows in the current tab's list (for selection bounds).
fn current_list_len(state: &TuiState) -> usize {
    match state.current_tab {
        Tab::Providers => state
            .provider_service
            .list_providers()
            .map(|v| v.len())
            .unwrap_or(0),
        Tab::Mcp => state
            .mcp_service
            .list_servers()
            .map(|v| v.len())
            .unwrap_or(0),
        Tab::Prompts => state
            .prompt_service
            .list_prompts()
            .map(|v| v.len())
            .unwrap_or(0),
        Tab::Skills => state
            .skill_service
            .list_skills()
            .map(|v| v.len())
            .unwrap_or(0),
        Tab::Agents => olenro_core::claude_agents::list_agents().len(),
        Tab::Discover => state.discover_results.len(),
        Tab::Sessions => state
            .session_manager
            .list_sessions(&state.active_app)
            .map(|v| v.len())
            .unwrap_or(0),
        Tab::OpenClawEnv => openclaw_env_keys().len(),
        Tab::Workspace => olenro_core::openclaw_workspace::list_daily_memory_files()
            .map(|v| v.len())
            .unwrap_or(0),
        Tab::HermesMemory => 2, // MEMORY.md + USER.md
        Tab::Universal => state
            .universal_service
            .list()
            .map(|v| v.len())
            .unwrap_or(0),
        _ => 0,
    }
}

/// The two Hermes memory kinds, in display/selection order.
const HERMES_MEMORY_KINDS: [olenro_core::app_config_writers::hermes_config::MemoryKind; 2] = [
    olenro_core::app_config_writers::hermes_config::MemoryKind::Memory,
    olenro_core::app_config_writers::hermes_config::MemoryKind::User,
];

/// Sorted env-var keys from the live OpenClaw config (stable order for selection).
fn openclaw_env_keys() -> Vec<String> {
    use olenro_core::app_config_writers::openclaw_config as oc;
    let mut keys: Vec<String> = oc::get_env_config()
        .map(|c| c.vars.into_keys().collect())
        .unwrap_or_default();
    keys.sort();
    keys
}

/// Enter / `s` on a selected list item performs the tab's primary action.
fn handle_enter_list(state: &mut TuiState) {
    match state.current_tab {
        Tab::Providers => {
            let providers = state.provider_service.list_providers().unwrap_or_default();
            if let Some(p) = providers.get(state.selected_index) {
                let app = state.active_app;
                let app_name = state.active_app_name();
                match state.provider_service.switch_provider(&p.id, app) {
                    Ok(_) => state.set_status(format!("Switched {} → {}", app_name, p.name)),
                    Err(e) => state.set_status(format!("Switch failed: {}", e)),
                }
            }
        }
        Tab::Mcp => {
            let servers = state.mcp_service.list_servers().unwrap_or_default();
            if let Some(s) = servers.get(state.selected_index) {
                let new_enabled = !s.enabled;
                match state.mcp_service.set_enabled(&s.id, new_enabled) {
                    Ok(_) => state.set_status(format!(
                        "{} MCP server: {}",
                        if new_enabled { "Enabled" } else { "Disabled" },
                        s.name
                    )),
                    Err(e) => state.set_status(format!("Toggle failed: {}", e)),
                }
            }
        }
        Tab::Prompts => {
            let prompts = state.prompt_service.list_prompts().unwrap_or_default();
            if let Some(p) = prompts.get(state.selected_index) {
                match state.prompt_service.enable_prompt(&p.id, &state.active_app) {
                    Ok(_) => state.set_status(format!(
                        "Enabled prompt for {}: {}",
                        state.active_app_name(),
                        p.name
                    )),
                    Err(e) => state.set_status(format!("Enable failed: {}", e)),
                }
            }
        }
        Tab::Skills => {
            let skills = state.skill_service.list_skills().unwrap_or_default();
            if let Some(s) = skills.get(state.selected_index) {
                match state.skill_service.update(&s.id) {
                    Ok(_) => state.set_status(format!("Updated skill: {}", s.name)),
                    Err(e) => state.set_status(format!("Update failed: {}", e)),
                }
            }
        }
        Tab::Sessions => {
            let sessions = state
                .session_manager
                .list_sessions(&state.active_app)
                .unwrap_or_default();
            if let Some(s) = sessions.get(state.selected_index) {
                let cwd = s.cwd.clone().unwrap_or_else(|| "?".to_string());
                state.set_status(format!(
                    "{} · {} msgs · {} · {}",
                    s.name, s.message_count, cwd, s.path
                ));
            }
        }
        Tab::Workspace => {
            let files =
                olenro_core::openclaw_workspace::list_daily_memory_files().unwrap_or_default();
            if let Some(f) = files.get(state.selected_index) {
                let preview = f.preview.replace('\n', " ");
                state.set_status(format!("{} · {} bytes · {}", f.filename, f.size_bytes, preview));
            }
        }
        Tab::HermesMemory => {
            use olenro_core::app_config_writers::hermes_config as hc;
            let Some(kind) = HERMES_MEMORY_KINDS.get(state.selected_index).copied() else {
                return;
            };
            let limits = hc::read_memory_limits().unwrap_or_default();
            let currently = match kind {
                hc::MemoryKind::Memory => limits.memory_enabled,
                hc::MemoryKind::User => limits.user_enabled,
            };
            let label = match kind {
                hc::MemoryKind::Memory => "MEMORY.md",
                hc::MemoryKind::User => "USER.md",
            };
            match hc::set_memory_enabled(kind, !currently) {
                Ok(_) => state.set_status(format!(
                    "{} memory: {}",
                    if !currently { "Enabled" } else { "Disabled" },
                    label
                )),
                Err(e) => state.set_status(format!("Toggle failed: {}", e)),
            }
        }
        Tab::Universal => universal_cycle_apps(state),
        Tab::Discover => discover_install(state),
        Tab::Agents => {
            let agents = olenro_core::claude_agents::list_agents();
            if let Some(a) = agents.get(state.selected_index) {
                let tools = if a.tools.is_empty() {
                    "all tools".to_string()
                } else {
                    a.tools.join(", ")
                };
                state.set_status(format!(
                    "{} [{}] · model: {} · tools: {} · {}",
                    a.name,
                    a.scope.label(),
                    a.model.as_deref().unwrap_or("inherit"),
                    tools,
                    a.path
                ));
            }
        }
        _ => {}
    }
}

/// Cycle the target apps of the selected universal provider through presets:
/// all → claude → codex → gemini → none → all.
fn universal_cycle_apps(state: &mut TuiState) {
    let providers = state.universal_service.list().unwrap_or_default();
    let Some(mut p) = providers.into_iter().nth(state.selected_index) else {
        return;
    };
    // Preset as (claude, codex, gemini).
    const PRESETS: [(bool, bool, bool); 5] = [
        (true, true, true),
        (true, false, false),
        (false, true, false),
        (false, false, true),
        (false, false, false),
    ];
    let cur = (p.apps.claude, p.apps.codex, p.apps.gemini);
    let pos = PRESETS.iter().position(|x| *x == cur).unwrap_or(0);
    let next = PRESETS[(pos + 1) % PRESETS.len()];
    p.apps.claude = next.0;
    p.apps.codex = next.1;
    p.apps.gemini = next.2;
    let label = universal_apps_label(&p.apps);
    match state.universal_service.save(&p) {
        Ok(_) => state.set_status(format!("{} → apps: {}", p.name, label)),
        Err(e) => state.set_status(format!("Save failed: {}", e)),
    }
}

/// Load the skills.sh popular list into discover state (blocking on the runtime).
fn load_discover_popular(state: &mut TuiState) {
    state.discover_loaded = true;
    state.set_status("Loading popular skills from skills.sh…");
    let handle = state.runtime.clone();
    match handle.block_on(olenro_core::skills_sh::popular(40)) {
        Ok(res) => {
            state.selected_index = 0;
            state.discover_results = res.skills;
            state.discover_label = format!("Popular ({})", state.discover_results.len());
            state.set_status(format!(
                "Loaded {} popular skills from skills.sh",
                state.discover_results.len()
            ));
        }
        Err(e) => {
            state.discover_label = "Popular (unavailable)".to_string();
            state.set_status(format!("skills.sh load failed: {}", e));
        }
    }
}

/// Search skills.sh for `query` and replace the discover results.
fn skills_search(state: &mut TuiState, query: &str) {
    let handle = state.runtime.clone();
    match handle.block_on(olenro_core::skills_sh::search(query, 40, 0)) {
        Ok(res) => {
            state.selected_index = 0;
            state.discover_results = res.skills;
            state.discover_loaded = true;
            state.discover_label = format!("\"{}\" ({})", query, state.discover_results.len());
            state.set_status(format!(
                "Found {} skills for \"{}\"",
                state.discover_results.len(),
                query
            ));
        }
        Err(e) => state.set_status(format!("Search failed: {}", e)),
    }
}

/// Install the selected discoverable skill from its GitHub repo.
fn discover_install(state: &mut TuiState) {
    let Some(skill) = state.discover_results.get(state.selected_index).cloned() else {
        state.set_status("No skill selected");
        return;
    };
    let slug = skill.repo_slug();
    state.set_status(format!("Installing {}…", slug));
    match state.skill_service.install_from_github(&slug) {
        Ok(s) => state.set_status(format!("Installed {} from {}", s.name, slug)),
        Err(e) => state.set_status(format!("Install failed ({}): {}", slug, e)),
    }
}

/// Short label for which apps a universal provider targets.
fn universal_apps_label(apps: &olenro_core::provider::UniversalProviderApps) -> String {
    let mut parts = Vec::new();
    if apps.claude {
        parts.push("claude");
    }
    if apps.codex {
        parts.push("codex");
    }
    if apps.gemini {
        parts.push("gemini");
    }
    if parts.is_empty() {
        "(none)".to_string()
    } else {
        parts.join("+")
    }
}

/// `+` / `a` opens the add dialog for the current tab.
fn handle_add(state: &mut TuiState) {
    state.close_dialog();
    match state.current_tab {
        Tab::Providers => state.dialog_mode = DialogMode::AddProvider,
        Tab::Mcp => state.dialog_mode = DialogMode::AddMcp,
        Tab::Prompts => state.dialog_mode = DialogMode::AddPrompt,
        Tab::Skills => state.dialog_mode = DialogMode::AddSkill,
        Tab::Agents => state.dialog_mode = DialogMode::AddAgent,
        Tab::OpenClawEnv => state.dialog_mode = DialogMode::AddOpenClawEnv,
        Tab::Universal => state.dialog_mode = DialogMode::AddUniversal,
        _ => state.set_status("Add is not available on this tab"),
    }
}

/// `e` opens the edit dialog for the selected item (Providers / Prompts).
fn handle_edit(state: &mut TuiState) {
    match state.current_tab {
        Tab::Providers => {
            let providers = state.provider_service.list_providers().unwrap_or_default();
            if let Some(p) = providers.get(state.selected_index) {
                let id = p.id.clone();
                state.name_input = p.name.clone();
                state.endpoint_input = p
                    .settings_config
                    .get("base_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                state.dialog_mode = DialogMode::EditProvider(id);
                state.input_field = 0;
            }
        }
        Tab::Prompts => {
            let prompts = state.prompt_service.list_prompts().unwrap_or_default();
            if let Some(p) = prompts.get(state.selected_index) {
                let id = p.id.clone();
                state.content_input = p.content.clone();
                state.dialog_mode = DialogMode::EditPrompt(id);
                state.input_field = 0;
            }
        }
        Tab::OpenClawAgents => {
            use olenro_core::app_config_writers::openclaw_config as oc;
            // Prefill with the current primary model, if any.
            state.content_input = oc::get_default_model()
                .ok()
                .flatten()
                .map(|m| m.primary)
                .unwrap_or_default();
            state.dialog_mode = DialogMode::SetOpenClawPrimaryModel;
            state.input_field = 0;
        }
        Tab::Universal => {
            let providers = state.universal_service.list().unwrap_or_default();
            if let Some(p) = providers.get(state.selected_index) {
                state.name_input = p.name.clone();
                state.endpoint_input = p.base_url.clone();
                state.dialog_mode = DialogMode::EditUniversal(p.id.clone());
                state.input_field = 0;
            }
        }
        _ => state.set_status("Edit is not available on this tab"),
    }
}

/// `d` opens a delete-confirmation dialog for the selected item.
fn handle_delete(state: &mut TuiState) {
    match state.current_tab {
        Tab::Providers => {
            let providers = state.provider_service.list_providers().unwrap_or_default();
            if let Some(p) = providers.get(state.selected_index) {
                state.dialog_mode = DialogMode::DeleteProvider(p.id.clone());
            }
        }
        Tab::Mcp => {
            let servers = state.mcp_service.list_servers().unwrap_or_default();
            if let Some(s) = servers.get(state.selected_index) {
                state.dialog_mode = DialogMode::DeleteMcp(s.id.clone());
            }
        }
        Tab::Prompts => {
            let prompts = state.prompt_service.list_prompts().unwrap_or_default();
            if let Some(p) = prompts.get(state.selected_index) {
                state.dialog_mode = DialogMode::DeletePrompt(p.id.clone());
            }
        }
        Tab::Skills => {
            let skills = state.skill_service.list_skills().unwrap_or_default();
            if let Some(s) = skills.get(state.selected_index) {
                state.dialog_mode = DialogMode::DeleteSkill(s.id.clone());
            }
        }
        Tab::Sessions => {
            let sessions = state
                .session_manager
                .list_sessions(&state.active_app)
                .unwrap_or_default();
            if let Some(s) = sessions.get(state.selected_index) {
                state.dialog_mode = DialogMode::DeleteSession(s.id.clone());
            }
        }
        Tab::OpenClawEnv => {
            let keys = openclaw_env_keys();
            if let Some(k) = keys.get(state.selected_index) {
                state.dialog_mode = DialogMode::DeleteOpenClawEnv(k.clone());
            }
        }
        Tab::Workspace => {
            let files =
                olenro_core::openclaw_workspace::list_daily_memory_files().unwrap_or_default();
            if let Some(f) = files.get(state.selected_index) {
                state.dialog_mode = DialogMode::DeleteOpenClawMemory(f.filename.clone());
            }
        }
        Tab::Universal => {
            let providers = state.universal_service.list().unwrap_or_default();
            if let Some(p) = providers.get(state.selected_index) {
                state.dialog_mode = DialogMode::DeleteUniversal(p.id.clone());
            }
        }
        Tab::Agents => {
            let agents = olenro_core::claude_agents::list_agents();
            if let Some(a) = agents.get(state.selected_index) {
                state.dialog_mode =
                    DialogMode::DeleteAgent(a.scope.label().to_string(), a.name.clone());
            }
        }
        _ => state.set_status("Delete is not available on this tab"),
    }
}

/// Cycle the OpenClaw tools profile through the supported presets (and "none").
fn openclaw_cycle_profile(state: &mut TuiState) {
    use olenro_core::app_config_writers::openclaw_config as oc;
    const PROFILES: [&str; 5] = ["minimal", "coding", "messaging", "full", ""];
    let mut tools = match oc::get_tools_config() {
        Ok(t) => t,
        Err(e) => {
            state.set_status(format!("Read tools failed: {}", e));
            return;
        }
    };
    let current = tools.profile.as_deref().unwrap_or("");
    let pos = PROFILES.iter().position(|p| *p == current).unwrap_or(0);
    let next = PROFILES[(pos + 1) % PROFILES.len()];
    tools.profile = if next.is_empty() {
        None
    } else {
        Some(next.to_string())
    };
    match oc::set_tools_config(&tools) {
        Ok(_) => state.set_status(format!(
            "Tools profile → {}",
            if next.is_empty() { "(none)" } else { next }
        )),
        Err(e) => state.set_status(format!("Set profile failed: {}", e)),
    }
}

/// Execute the CRUD action backing the current dialog, then close it.
fn submit_dialog(state: &mut TuiState) {
    let mode = state.dialog_mode.clone();
    match mode {
        DialogMode::AddProvider => {
            if state.name_input.trim().is_empty() || state.endpoint_input.trim().is_empty() {
                state.set_status("Name and endpoint are required");
                return;
            }
            let category = CATEGORIES[state.category_index].1;
            let api_key = if state.api_key_input.trim().is_empty() {
                None
            } else {
                Some(state.api_key_input.trim())
            };
            match state.provider_service.add_provider(
                state.name_input.trim(),
                state.endpoint_input.trim(),
                category,
                api_key,
            ) {
                Ok(p) => state.set_status(format!("Added provider: {}", p.name)),
                Err(e) => state.set_status(format!("Add failed: {}", e)),
            }
        }
        DialogMode::EditProvider(id) => match state.provider_service.get_provider(&id) {
            Ok(Some(mut p)) => {
                if !state.name_input.trim().is_empty() {
                    p.name = state.name_input.trim().to_string();
                }
                if let Some(obj) = p.settings_config.as_object_mut() {
                    obj.insert(
                        "base_url".to_string(),
                        serde_json::Value::String(state.endpoint_input.trim().to_string()),
                    );
                }
                match state.provider_service.update_provider(p) {
                    Ok(_) => state.set_status("Provider updated"),
                    Err(e) => state.set_status(format!("Update failed: {}", e)),
                }
            }
            Ok(None) => state.set_status("Provider not found"),
            Err(e) => state.set_status(format!("Update failed: {}", e)),
        },
        DialogMode::DeleteProvider(id) => match state.provider_service.delete_provider(&id) {
            Ok(_) => {
                state.selected_index = 0;
                state.set_status("Provider deleted");
            }
            Err(e) => state.set_status(format!("Delete failed: {}", e)),
        },
        DialogMode::AddMcp => {
            if state.name_input.trim().is_empty() || state.command_input.trim().is_empty() {
                state.set_status("Name and command are required");
                return;
            }
            let args: Vec<String> = state
                .args_input
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            match state.mcp_service.add_server(
                state.name_input.trim(),
                state.command_input.trim(),
                args,
                std::collections::HashMap::new(),
                None,
            ) {
                Ok(s) => state.set_status(format!("Added MCP server: {}", s.name)),
                Err(e) => state.set_status(format!("Add failed: {}", e)),
            }
        }
        DialogMode::DeleteMcp(id) => match state.mcp_service.remove_server(&id) {
            Ok(_) => {
                state.selected_index = 0;
                state.set_status("MCP server deleted");
            }
            Err(e) => state.set_status(format!("Delete failed: {}", e)),
        },
        DialogMode::AddPrompt => {
            if state.name_input.trim().is_empty() || state.content_input.trim().is_empty() {
                state.set_status("Name and content are required");
                return;
            }
            let input = CreatePromptInput {
                name: state.name_input.trim().to_string(),
                content: state.content_input.clone(),
                description: None,
            };
            match state.prompt_service.create_prompt(input) {
                Ok(p) => state.set_status(format!("Added prompt: {}", p.name)),
                Err(e) => state.set_status(format!("Add failed: {}", e)),
            }
        }
        DialogMode::EditPrompt(id) => {
            match state
                .prompt_service
                .update_prompt(&id, &state.content_input)
            {
                Ok(_) => state.set_status("Prompt updated"),
                Err(e) => state.set_status(format!("Update failed: {}", e)),
            }
        }
        DialogMode::DeletePrompt(id) => match state.prompt_service.delete_prompt(&id) {
            Ok(_) => {
                state.selected_index = 0;
                state.set_status("Prompt deleted");
            }
            Err(e) => state.set_status(format!("Delete failed: {}", e)),
        },
        DialogMode::AddSkill => {
            if state.name_input.trim().is_empty() {
                state.set_status("Repository (owner/name) is required");
                return;
            }
            match state
                .skill_service
                .install_from_github(state.name_input.trim())
            {
                Ok(s) => state.set_status(format!("Installed skill: {}", s.name)),
                Err(e) => state.set_status(format!("Install failed: {}", e)),
            }
        }
        DialogMode::DeleteSkill(id) => match state.skill_service.uninstall(&id) {
            Ok(_) => {
                state.selected_index = 0;
                state.set_status("Skill uninstalled");
            }
            Err(e) => state.set_status(format!("Uninstall failed: {}", e)),
        },
        DialogMode::DeleteSession(id) => {
            match state
                .session_manager
                .delete_session(&state.active_app, &id)
            {
                Ok(_) => {
                    state.selected_index = 0;
                    state.set_status("Session deleted");
                }
                Err(e) => state.set_status(format!("Delete failed: {}", e)),
            }
        }
        DialogMode::AddOpenClawEnv => {
            use olenro_core::app_config_writers::openclaw_config as oc;
            if state.name_input.trim().is_empty() {
                state.set_status("Variable name is required");
                return;
            }
            let key = state.name_input.trim().to_string();
            // Store the value as a string; JSON values can be entered as-is and
            // are coerced to a string if not valid JSON.
            let value = serde_json::from_str::<serde_json::Value>(state.content_input.trim())
                .unwrap_or_else(|_| serde_json::Value::String(state.content_input.clone()));
            match oc::get_env_config() {
                Ok(mut env) => {
                    env.vars.insert(key.clone(), value);
                    match oc::set_env_config(&env) {
                        Ok(_) => state.set_status(format!("Set env var: {}", key)),
                        Err(e) => state.set_status(format!("Set env failed: {}", e)),
                    }
                }
                Err(e) => state.set_status(format!("Read env failed: {}", e)),
            }
        }
        DialogMode::DeleteOpenClawEnv(key) => {
            use olenro_core::app_config_writers::openclaw_config as oc;
            match oc::get_env_config() {
                Ok(mut env) => {
                    env.vars.remove(&key);
                    match oc::set_env_config(&env) {
                        Ok(_) => {
                            state.selected_index = 0;
                            state.set_status(format!("Removed env var: {}", key));
                        }
                        Err(e) => state.set_status(format!("Remove env failed: {}", e)),
                    }
                }
                Err(e) => state.set_status(format!("Read env failed: {}", e)),
            }
        }
        DialogMode::SetOpenClawPrimaryModel => {
            use olenro_core::app_config_writers::openclaw_config as oc;
            if state.content_input.trim().is_empty() {
                state.set_status("Primary model is required (e.g. provider/model)");
                return;
            }
            // Preserve any existing fallbacks/extra fields.
            let mut model = oc::get_default_model().ok().flatten().unwrap_or(
                oc::OpenClawDefaultModel {
                    primary: String::new(),
                    fallbacks: Vec::new(),
                    extra: std::collections::HashMap::new(),
                },
            );
            model.primary = state.content_input.trim().to_string();
            match oc::set_default_model(&model) {
                Ok(_) => state.set_status(format!("Default model → {}", model.primary)),
                Err(e) => state.set_status(format!("Set model failed: {}", e)),
            }
        }
        DialogMode::DeleteOpenClawMemory(filename) => {
            match olenro_core::openclaw_workspace::delete_daily_memory_file(&filename) {
                Ok(_) => {
                    state.selected_index = 0;
                    state.set_status(format!("Deleted memory file: {}", filename));
                }
                Err(e) => state.set_status(format!("Delete failed: {}", e)),
            }
        }
        DialogMode::AddUniversal => {
            if state.name_input.trim().is_empty() || state.endpoint_input.trim().is_empty() {
                state.set_status("Name and base URL are required");
                return;
            }
            match state.universal_service.add(
                state.name_input.trim(),
                "custom",
                state.endpoint_input.trim(),
                state.api_key_input.trim(),
            ) {
                Ok(p) => state.set_status(format!(
                    "Added universal provider: {} (Enter to pick target apps)",
                    p.name
                )),
                Err(e) => state.set_status(format!("Add failed: {}", e)),
            }
        }
        DialogMode::EditUniversal(id) => match state.universal_service.get(&id) {
            Ok(Some(mut p)) => {
                if !state.name_input.trim().is_empty() {
                    p.name = state.name_input.trim().to_string();
                }
                p.base_url = state.endpoint_input.trim().to_string();
                match state.universal_service.save(&p) {
                    Ok(_) => state.set_status("Universal provider updated"),
                    Err(e) => state.set_status(format!("Update failed: {}", e)),
                }
            }
            Ok(None) => state.set_status("Universal provider not found"),
            Err(e) => state.set_status(format!("Update failed: {}", e)),
        },
        DialogMode::DeleteUniversal(id) => match state.universal_service.delete(&id) {
            Ok(_) => {
                state.selected_index = 0;
                state.set_status("Universal provider deleted");
            }
            Err(e) => state.set_status(format!("Delete failed: {}", e)),
        },
        DialogMode::AddAgent => {
            use olenro_core::claude_agents::{self, AgentScope};
            if state.name_input.trim().is_empty() {
                state.set_status("Agent name is required");
                return;
            }
            let name = state.name_input.trim();
            let desc = state.content_input.trim();
            // Start with a minimal valid system prompt; the user edits the .md
            // for the full agent behaviour.
            let body = if desc.is_empty() {
                format!("You are the {name} agent.")
            } else {
                format!("You are the {name} agent. {desc}")
            };
            match claude_agents::create_agent(
                AgentScope::User,
                name,
                if desc.is_empty() { None } else { Some(desc) },
                &[],
                None,
                &body,
            ) {
                Ok(a) => state.set_status(format!("Created agent: {} ({})", a.name, a.path)),
                Err(e) => state.set_status(format!("Create failed: {}", e)),
            }
        }
        DialogMode::SearchSkills => {
            let query = state.name_input.trim().to_string();
            if query.len() < 2 {
                state.set_status("Search needs at least 2 characters");
                return;
            }
            // close_dialog (below) clears inputs; search now with the captured query.
            state.close_dialog();
            skills_search(state, &query);
            return;
        }
        DialogMode::DeleteAgent(scope_label, name) => {
            use olenro_core::claude_agents::{self, AgentScope};
            let scope = if scope_label == "project" {
                AgentScope::Project
            } else {
                AgentScope::User
            };
            match claude_agents::delete_agent(scope, &name) {
                Ok(_) => {
                    state.selected_index = 0;
                    state.set_status(format!("Deleted agent: {}", name));
                }
                Err(e) => state.set_status(format!("Delete failed: {}", e)),
            }
        }
        DialogMode::None => {}
    }

    state.close_dialog();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::state::TuiState;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn draw(state: &mut TuiState) {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| crate::tui::ui::render(f, state)).unwrap();
    }

    #[tokio::test]
    async fn renders_every_tab_and_dialog_without_panic() {
        let mut state = TuiState::new(tokio::runtime::Handle::current());
        for _ in 0..8 {
            draw(&mut state);
            state.next_tab();
        }

        // Every app, with its visible tabs, must render without panicking.
        for _ in 0..crate::tui::state::APPS.len() {
            for _ in 0..state.visible_tabs().len() {
                draw(&mut state);
                state.next_tab();
            }
            state.cycle_app(true);
        }
        state.active_app = olenro_core::provider::AppType::Claude;
        state.current_tab = Tab::Providers;
        // Open each dialog and render
        state.current_tab = Tab::Providers;
        handle_add(&mut state);
        draw(&mut state);
        state.push_input_char('x');
        draw(&mut state);
        state.close_dialog();

        // Confirm-delete dialog rendering
        state.dialog_mode = DialogMode::DeleteProvider("nope".into());
        draw(&mut state);
        state.close_dialog();

        // OpenClaw dialogs render without panic.
        for mode in [
            DialogMode::AddOpenClawEnv,
            DialogMode::SetOpenClawPrimaryModel,
            DialogMode::DeleteOpenClawEnv("KEY".into()),
            DialogMode::DeleteOpenClawMemory("2026-06-10.md".into()),
            DialogMode::AddUniversal,
            DialogMode::EditUniversal("up_1".into()),
            DialogMode::DeleteUniversal("up_1".into()),
            DialogMode::AddAgent,
            DialogMode::DeleteAgent("user".into(), "code-reviewer".into()),
            DialogMode::SearchSkills,
        ] {
            state.dialog_mode = mode;
            draw(&mut state);
            state.close_dialog();
        }
    }
}
