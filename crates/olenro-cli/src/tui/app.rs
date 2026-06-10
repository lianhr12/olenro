//! TUI application and main loop

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

use olenro_core::app_config::AppType;
use olenro_core::prompt::CreatePromptInput;
use olenro_core::provider::AppType as ProviderAppType;

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
    use olenro_core::provider::AppType as ProviderAppType;
    let currently = state.proxy_service.is_taken_over(ProviderAppType::Claude);
    let enable = !currently;
    match state
        .proxy_service
        .set_takeover(ProviderAppType::Claude, enable)
    {
        Ok(_) => {
            if enable {
                state.set_status(format!(
                    "Claude takeover ON → routed through {}",
                    state.proxy_service.proxy_url()
                ));
            } else {
                state.set_status("Claude takeover OFF → restored previous endpoint");
            }
        }
        Err(e) => state.set_status(format!("Takeover failed: {}", e)),
    }
}

/// Surface the command to resume the selected Claude session.
fn session_resume(state: &mut TuiState) {
    use olenro_core::provider::AppType as ProviderAppType;
    let sessions = state
        .session_manager
        .list_sessions(&ProviderAppType::Claude)
        .unwrap_or_default();
    if let Some(s) = sessions.get(state.selected_index) {
        let dir = s.cwd.clone().unwrap_or_else(|| ".".to_string());
        state.set_status(format!("Resume: (cd {}) && claude --resume {}", dir, s.id));
    } else {
        state.set_status("No session selected");
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
        Tab::Sessions => state
            .session_manager
            .list_sessions(&olenro_core::provider::AppType::Claude)
            .map(|v| v.len())
            .unwrap_or(0),
        _ => 0,
    }
}

/// Enter / `s` on a selected list item performs the tab's primary action.
fn handle_enter_list(state: &mut TuiState) {
    match state.current_tab {
        Tab::Providers => {
            let providers = state.provider_service.list_providers().unwrap_or_default();
            if let Some(p) = providers.get(state.selected_index) {
                match state
                    .provider_service
                    .switch_provider(&p.id, ProviderAppType::Claude)
                {
                    Ok(_) => state.set_status(format!(
                        "Switched Claude → {} (wrote ~/.claude/settings.json)",
                        p.name
                    )),
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
                match state.prompt_service.enable_prompt(&p.id, &AppType::Claude) {
                    Ok(_) => state.set_status(format!("Enabled prompt: {}", p.name)),
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
                .list_sessions(&olenro_core::provider::AppType::Claude)
                .unwrap_or_default();
            if let Some(s) = sessions.get(state.selected_index) {
                let cwd = s.cwd.clone().unwrap_or_else(|| "?".to_string());
                state.set_status(format!(
                    "{} · {} msgs · {} · {}",
                    s.name, s.message_count, cwd, s.path
                ));
            }
        }
        _ => {}
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
                .list_sessions(&olenro_core::provider::AppType::Claude)
                .unwrap_or_default();
            if let Some(s) = sessions.get(state.selected_index) {
                state.dialog_mode = DialogMode::DeleteSession(s.id.clone());
            }
        }
        _ => state.set_status("Delete is not available on this tab"),
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
                .delete_session(&olenro_core::provider::AppType::Claude, &id)
            {
                Ok(_) => {
                    state.selected_index = 0;
                    state.set_status("Session deleted");
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
    }
}
