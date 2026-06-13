//! TUI rendering functions

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::state::{AddProviderStage, DialogMode, Tab, TuiState, CATEGORIES};

/// Solid panel background so the terminal wallpaper/transparency does not
/// bleed through and hurt readability.
const BG: Color = Color::Rgb(18, 20, 28);

/// A bordered panel block with a filled background and a title.
fn panel(title: String) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().bg(BG).fg(Color::White))
}

/// Main render function
pub fn render(f: &mut Frame, state: &mut TuiState) {
    // Paint a solid background across the whole screen first.
    f.render_widget(Block::default().style(Style::default().bg(BG)), f.size());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Status bar
        ])
        .split(f.size());

    render_title(f, chunks[0], state);
    render_tabs(f, chunks[1], state);
    render_content(f, chunks[2], state);
    render_status(f, chunks[3], state);

    // Overlay dialog on top of everything
    if state.dialog_mode != DialogMode::None {
        render_dialog(f, f.size(), state);
    }
}

fn render_title(f: &mut Frame, area: Rect, state: &TuiState) {
    let line = Line::from(vec![
        Span::styled(
            // Track the real crate version so the title never drifts from
            // `olenro version`. ("v2.0.0" was a codename, not the package version.)
            format!("Olenro TUI  v{}   ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled("App: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("< {} >", state.active_app_name()),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  [/] app   [Tab] panel   [q] quit",
            Style::default().fg(Color::Cyan),
        ),
    ]);
    let title = Paragraph::new(line)
        .style(Style::default().bg(BG))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

/// Short label for a tab in the navigation bar.
fn tab_label(tab: Tab) -> &'static str {
    match tab {
        Tab::Dashboard => "Dashboard",
        Tab::Providers => "Providers",
        Tab::Mcp => "MCP",
        Tab::Prompts => "Prompts",
        Tab::Skills => "Skills",
        Tab::Discover => "Discover",
        Tab::Agents => "Agents",
        Tab::Sessions => "Sessions",
        Tab::Usage => "Usage",
        Tab::Proxy => "Proxy",
        Tab::Universal => "Universal",
        Tab::Workspace => "Workspace",
        Tab::OpenClawEnv => "Env",
        Tab::OpenClawTools => "Tools",
        Tab::OpenClawAgents => "Agent Defaults",
        Tab::HermesMemory => "Memory",
        Tab::Settings => "Settings",
    }
}

fn render_tabs(f: &mut Frame, area: Rect, state: &TuiState) {
    let mut spans: Vec<Span> = Vec::new();
    for tab in state.visible_tabs() {
        let name = tab_label(tab);
        let style = if state.current_tab == tab {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        spans.push(Span::styled(format!(" {} ", name), style));
        spans.push(Span::raw(" "));
    }

    let tabs = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(BG))
        .block(Block::default().borders(Borders::ALL).title(" Navigation "));
    f.render_widget(tabs, area);
}

fn render_content(f: &mut Frame, area: Rect, state: &mut TuiState) {
    match state.current_tab {
        Tab::Dashboard => render_dashboard(f, area, state),
        Tab::Providers => render_providers(f, area, state),
        Tab::Mcp => render_mcp(f, area, state),
        Tab::Prompts => render_prompts(f, area, state),
        Tab::Skills => render_skills(f, area, state),
        Tab::Discover => render_discover(f, area, state),
        Tab::Agents => render_agents(f, area, state),
        Tab::Sessions => render_sessions(f, area, state),
        Tab::Usage => render_usage(f, area, state),
        Tab::Proxy => render_proxy(f, area, state),
        Tab::Universal => render_universal(f, area, state),
        Tab::Workspace => render_workspace(f, area, state),
        Tab::OpenClawEnv => render_openclaw_env(f, area, state),
        Tab::OpenClawTools => render_openclaw_tools(f, area, state),
        Tab::OpenClawAgents => render_openclaw_agents(f, area, state),
        Tab::HermesMemory => render_hermes_memory(f, area, state),
        Tab::Settings => render_settings(f, area, state),
    }
}

/// Style for the currently-selected list row.
fn row_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}

/// Activity overview: live snapshot of resources, proxy and usage for the
/// app currently being managed (mirrors the desktop Dashboard view).
fn render_dashboard(f: &mut Frame, area: Rect, state: &mut TuiState) {
    use olenro_core::proxy::ProxyStatus;

    let providers = state.provider_service.list_providers().unwrap_or_default();
    let servers = state.mcp_service.list_servers().unwrap_or_default();
    let prompts = state.prompt_service.list_prompts().unwrap_or_default();
    let skills = state.skill_service.list_skills().unwrap_or_default();
    let mcp_enabled = servers.iter().filter(|s| s.enabled).count();
    let prompts_enabled = prompts.iter().filter(|p| p.enabled).count();

    let summary = state.usage_service.summary(30).unwrap_or_default();

    let status = state.proxy_service.status();
    let cfg = state.proxy_service.config();
    let (dot, label, color) = match status {
        ProxyStatus::Running => ("●", "Running", Color::Green),
        ProxyStatus::Starting => ("◐", "Starting", Color::Yellow),
        ProxyStatus::Stopping => ("◑", "Stopping", Color::Yellow),
        ProxyStatus::Stopped => ("○", "Stopped", Color::Gray),
        ProxyStatus::Error => ("✗", "Error", Color::Red),
    };
    let taken = state.proxy_service.is_taken_over(state.active_app);

    let forwarding = match state.proxy_service.resolved_target() {
        Some(t) => providers
            .iter()
            .find(|p| p.id == t.provider_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| t.provider_id.clone()),
        None => "—".to_string(),
    };

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Managing app:  "),
            Span::styled(
                state.active_app_name(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   ([/] to switch)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Resources",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("    Providers:  {}", providers.len())),
        Line::from(format!(
            "    MCP:        {} enabled / {} total",
            mcp_enabled,
            servers.len()
        )),
        Line::from(format!(
            "    Prompts:    {} enabled / {} total",
            prompts_enabled,
            prompts.len()
        )),
        Line::from(format!("    Skills:     {}", skills.len())),
        Line::from(""),
        Line::from(Span::styled(
            "  Proxy",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::raw("    Status:     "),
            Span::styled(
                format!("{} {}", dot, label),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("   http://{}:{}", cfg.address, cfg.port)),
        ]),
        Line::from(format!("    Forwarding: {}", forwarding)),
        Line::from(format!(
            "    Takeover:   {} {}",
            state.active_app_name(),
            if taken { "ON" } else { "off" }
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Usage (last 30 days)",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "    Requests:   {}    Cost: {}",
            summary.requests,
            fmt_cost(summary.cost)
        )),
        Line::from(format!(
            "    Tokens:     in {}  /  out {}",
            summary.input_tokens, summary.output_tokens
        )),
    ];

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(" Dashboard ".to_string()))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn render_providers(f: &mut Frame, area: Rect, state: &mut TuiState) {
    let providers = state.provider_service.list_providers().unwrap_or_default();

    if providers.is_empty() {
        render_empty(
            f,
            area,
            " Providers ",
            "No providers. Press [+]/[a] to add one.",
        );
        return;
    }

    let items: Vec<ListItem> = providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let selected = i == state.selected_index;
            let prefix = if selected { "> " } else { "  " };
            let endpoint = p
                .settings_config
                .get("base_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let fo = if p.in_failover_queue { "⚡" } else { " " };
            let content = format!("{}{}{} [{:?}]  {}", prefix, fo, p.name, p.category, endpoint);
            ListItem::new(content).style(row_style(selected))
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(format!(
            " Providers ({})  [Enter/s] switch  [e] edit  [f] failover  [+] add  [d] delete ",
            providers.len()
        )));
    f.render_widget(list, area);
}

fn render_mcp(f: &mut Frame, area: Rect, state: &mut TuiState) {
    let servers = state.mcp_service.list_servers().unwrap_or_default();

    if servers.is_empty() {
        render_empty(
            f,
            area,
            " MCP Servers ",
            "No MCP servers. Press [+]/[a] to add one.",
        );
        return;
    }

    let items: Vec<ListItem> = servers
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let selected = i == state.selected_index;
            let prefix = if selected { "> " } else { "  " };
            let status = if s.enabled { "✓" } else { "✗" };
            let args_str = s.args.join(" ");
            let content = format!(
                "{}{} {} - {} {}",
                prefix, status, s.name, s.command, args_str
            );
            ListItem::new(content).style(row_style(selected))
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(format!(
            " MCP Servers ({})  [Enter] toggle  [s] sync  [+] add  [d] delete ",
            servers.len()
        )));
    f.render_widget(list, area);
}

fn render_prompts(f: &mut Frame, area: Rect, state: &mut TuiState) {
    let prompts = state.prompt_service.list_prompts().unwrap_or_default();

    if prompts.is_empty() {
        render_empty(
            f,
            area,
            " Prompts ",
            "No prompts. Press [+]/[a] to add one.",
        );
        return;
    }

    let items: Vec<ListItem> = prompts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let selected = i == state.selected_index;
            let prefix = if selected { "> " } else { "  " };
            let status = if p.enabled { "[ON] " } else { "[off]" };
            let content = format!(
                "{}{} {} ({} chars)",
                prefix,
                status,
                p.name,
                p.content.len()
            );
            ListItem::new(content).style(row_style(selected))
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(format!(
            " Prompts ({})  [Enter/s] enable  [e] edit  [+] add  [d] delete ",
            prompts.len()
        )));
    f.render_widget(list, area);
}

fn render_skills(f: &mut Frame, area: Rect, state: &mut TuiState) {
    let skills = state.skill_service.list_skills().unwrap_or_default();

    if skills.is_empty() {
        render_empty(
            f,
            area,
            " Skills ",
            "No skills. Press [+]/[a] to install one (owner/name).",
        );
        return;
    }

    let items: Vec<ListItem> = skills
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let selected = i == state.selected_index;
            let prefix = if selected { "> " } else { "  " };
            let content = format!("{}{} v{}  ({})", prefix, s.name, s.version, s.path);
            ListItem::new(content).style(row_style(selected))
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(format!(
            " Skills ({})  [Enter] update  [s] sync→app  [+] install  [d] uninstall ",
            skills.len()
        )));
    f.render_widget(list, area);
}

/// skills.sh marketplace discovery: popular list or search results.
fn render_discover(f: &mut Frame, area: Rect, state: &mut TuiState) {
    if state.discover_results.is_empty() {
        let msg = if state.discover_loaded {
            "No skills found. Press [s] to search skills.sh, [r] for popular."
        } else {
            "Loading skills.sh… (if it stays empty, press [r] to retry — needs network)"
        };
        render_empty(f, area, " Discover (skills.sh) ", msg);
        return;
    }

    let items: Vec<ListItem> = state
        .discover_results
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let selected = i == state.selected_index;
            let prefix = if selected { "> " } else { "  " };
            ListItem::new(format!(
                "{}{:<26} {:>7} ↓  {}/{}",
                prefix,
                truncate(&s.name, 26),
                s.installs,
                s.repo_owner,
                s.repo_name
            ))
            .style(row_style(selected))
        })
        .collect();

    let label = if state.discover_label.is_empty() {
        format!("({})", state.discover_results.len())
    } else {
        state.discover_label.clone()
    };
    let list = List::new(items)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(format!(
            " Discover · skills.sh · {}  [Enter/i] install  [s] search  [r] popular ",
            label
        )));
    f.render_widget(list, area);
}

/// Claude subagents from `~/.claude/agents/` (user) and `.claude/agents/` (project).
fn render_agents(f: &mut Frame, area: Rect, state: &mut TuiState) {
    let agents = olenro_core::claude_agents::list_agents();
    if agents.is_empty() {
        render_empty(
            f,
            area,
            " Agents ",
            "No subagents. Press [+]/[a] to create one in ~/.claude/agents.",
        );
        return;
    }

    let items: Vec<ListItem> = agents
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let selected = i == state.selected_index;
            let prefix = if selected { "> " } else { "  " };
            let scope = match a.scope {
                olenro_core::claude_agents::AgentScope::User => "usr",
                olenro_core::claude_agents::AgentScope::Project => "prj",
            };
            let model = a.model.as_deref().unwrap_or("-");
            let desc = a.description.as_deref().unwrap_or("");
            ListItem::new(format!(
                "{}[{}] {:<20} {:<8} {}",
                prefix,
                scope,
                truncate(&a.name, 20),
                truncate(model, 8),
                truncate(desc, 40)
            ))
            .style(row_style(selected))
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(format!(
            " Agents ({})  [Enter] details  [+] add  [d] delete ",
            agents.len()
        )));
    f.render_widget(list, area);
}

fn render_sessions(f: &mut Frame, area: Rect, state: &mut TuiState) {
    let sessions = state
        .session_manager
        .list_sessions(&state.active_app)
        .unwrap_or_default();

    if sessions.is_empty() {
        render_empty(
            f,
            area,
            " Sessions ",
            &format!("No {} sessions found.", state.active_app_name()),
        );
        return;
    }

    let items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let selected = i == state.selected_index;
            let prefix = if selected { "> " } else { "  " };
            let cwd = s
                .cwd
                .as_deref()
                .map(|c| truncate(c, 30))
                .unwrap_or_default();
            let content = format!(
                "{}{:<40} {:>4} msgs  {}",
                prefix,
                truncate(&s.name, 40),
                s.message_count,
                cwd
            );
            ListItem::new(content).style(row_style(selected))
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(format!(
            " Sessions ({})  [Enter] details  [r] resume  [d] delete ",
            sessions.len()
        )));
    f.render_widget(list, area);
}

fn render_empty(f: &mut Frame, area: Rect, title: &str, msg: &str) {
    let p = Paragraph::new(vec![Line::from(""), Line::from(format!("  {}", msg))])
        .block(panel(title.to_string()))
        .style(Style::default().bg(BG).fg(Color::DarkGray));
    f.render_widget(p, area);
}

/// Render a horizontal ASCII bar of `count` units scaled against `max`.
fn ascii_bar(count: usize, max: usize, width: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let filled = (count * width) / max.max(1);
    "█".repeat(filled.max(if count > 0 { 1 } else { 0 }))
}

fn render_usage(f: &mut Frame, area: Rect, state: &mut TuiState) {
    const DAYS: i32 = 30;
    let summary = state.usage_service.summary(DAYS).unwrap_or_default();

    let text: Vec<Line> = if summary.requests > 0 {
        usage_with_data(state, DAYS, &summary)
    } else {
        usage_empty_state(state)
    };

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(format!(" Usage - Last {} Days ", DAYS)))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(paragraph, area);
}

/// Real usage rollups exist: render totals + per-provider request bars.
fn usage_with_data<'a>(
    state: &TuiState,
    days: i32,
    summary: &olenro_core::services::usage::UsageSummary,
) -> Vec<Line<'a>> {
    // Map provider id -> display name for nicer labels.
    let providers = state.provider_service.list_providers().unwrap_or_default();
    let name_of = |id: &str| -> String {
        providers
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| id.to_string())
    };

    let mut text = vec![
        Line::from(""),
        Line::from("  Totals"),
        Line::from(format!(
            "  Requests: {}   Cost: {}   Input: {}   Output: {}",
            summary.requests,
            fmt_cost(summary.cost),
            summary.input_tokens,
            summary.output_tokens
        )),
        Line::from(""),
        Line::from("  By Provider (requests / cost)"),
    ];

    let by_provider = state.usage_service.by_provider(days).unwrap_or_default();
    let max = by_provider
        .iter()
        .map(|p| p.requests as usize)
        .max()
        .unwrap_or(0);
    for p in &by_provider {
        let bar = ascii_bar(p.requests as usize, max, 28);
        let pct = if summary.requests > 0 {
            (p.requests * 100) / summary.requests
        } else {
            0
        };
        text.push(Line::from(format!(
            "  {:<16} {:<24} {} ({}%)  {}",
            truncate(&name_of(&p.provider_id), 16),
            bar,
            p.requests,
            pct,
            fmt_cost(p.cost)
        )));
    }

    // Daily trend: aggregate the per-provider/day rollups by date.
    let rollups = state.usage_service.list_recent(days).unwrap_or_default();
    if !rollups.is_empty() {
        use std::collections::BTreeMap;
        let mut by_date: BTreeMap<String, (i64, f64)> = BTreeMap::new();
        for r in &rollups {
            let e = by_date.entry(r.date.clone()).or_insert((0, 0.0));
            e.0 += r.requests;
            e.1 += r.cost;
        }
        // Show the most recent 14 days.
        let recent: Vec<(String, (i64, f64))> = by_date
            .into_iter()
            .rev()
            .take(14)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let day_max = recent.iter().map(|(_, (req, _))| *req as usize).max().unwrap_or(0);

        text.push(Line::from(""));
        text.push(Line::from("  Daily Trend (requests / cost)"));
        for (date, (req, cost)) in &recent {
            let bar = ascii_bar(*req as usize, day_max, 24);
            text.push(Line::from(format!(
                "  {:<10} {:<24} {:>5}  {}",
                date,
                bar,
                req,
                fmt_cost(*cost)
            )));
        }
    }

    text
}

/// Format a USD cost with adaptive precision so small amounts stay visible.
fn fmt_cost(cost: f64) -> String {
    if cost >= 1.0 {
        format!("${:.2}", cost)
    } else if cost >= 0.01 {
        format!("${:.4}", cost)
    } else {
        format!("${:.6}", cost)
    }
}

/// No usage recorded yet: show live resource counts instead of fabricated data.
fn usage_empty_state<'a>(state: &TuiState) -> Vec<Line<'a>> {
    let providers = state.provider_service.list_providers().unwrap_or_default();
    let servers = state.mcp_service.list_servers().unwrap_or_default();
    let prompts = state.prompt_service.list_prompts().unwrap_or_default();
    let skills = state.skill_service.list_skills().unwrap_or_default();

    let counts = [
        ("Providers", providers.len()),
        ("MCP Servers", servers.len()),
        ("Prompts", prompts.len()),
        ("Skills", skills.len()),
    ];
    let max = counts.iter().map(|(_, c)| *c).max().unwrap_or(0);

    let mut text: Vec<Line> = vec![
        Line::from(""),
        Line::from("  No API usage recorded yet."),
        Line::from(""),
        Line::from("  Configured resources (live from database):"),
        Line::from(""),
    ];
    for (label, count) in counts {
        let bar = ascii_bar(count, max, 28);
        text.push(Line::from(format!("  {:<12} {:<28} {}", label, bar, count)));
    }
    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "  Token/cost stats accrue once the local proxy records usage.",
        Style::default().fg(Color::DarkGray),
    )));
    text
}

/// Truncate a string to `max` chars, appending an ellipsis when cut.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

fn render_proxy(f: &mut Frame, area: Rect, state: &mut TuiState) {
    use olenro_core::proxy::ProxyStatus;

    let status = state.proxy_service.status();
    let cfg = state.proxy_service.config();
    let (dot, label, color) = match status {
        ProxyStatus::Running => ("●", "Running", Color::Green),
        ProxyStatus::Starting => ("◐", "Starting", Color::Yellow),
        ProxyStatus::Stopping => ("◑", "Stopping", Color::Yellow),
        ProxyStatus::Stopped => ("○", "Stopped", Color::Gray),
        ProxyStatus::Error => ("✗", "Error", Color::Red),
    };

    let mut text = vec![
        Line::from(""),
        Line::from("  Local HTTP Proxy"),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Status:  "),
            Span::styled(
                format!("{} {}", dot, label),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(format!("  Port:    {}", cfg.port)),
        Line::from(format!("  Address: {}", cfg.address)),
        Line::from(format!("  URL:     http://{}:{}", cfg.address, cfg.port)),
        Line::from(""),
        {
            let providers = state.provider_service.list_providers().unwrap_or_default();
            match state.proxy_service.resolved_target() {
                Some(t) => {
                    let name = providers
                        .iter()
                        .find(|p| p.id == t.provider_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| t.provider_id.clone());
                    Line::from(format!("  Forwarding to: {}  ({})", name, t.base_url))
                }
                None if status == ProxyStatus::Running => Line::from(Span::styled(
                    "  No provider with an API key — add one in Providers to forward.",
                    Style::default().fg(Color::Red),
                )),
                None => Line::from(Span::styled(
                    "  Forwards to the first provider that has an API key.",
                    Style::default().fg(Color::DarkGray),
                )),
            }
        },
        {
            let app = state.active_app;
            let name = state.active_app_name();
            let taken = state.proxy_service.is_taken_over(app);
            if taken {
                Line::from(vec![
                    Span::raw("  Takeover: "),
                    Span::styled(
                        format!("{} ON", name),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "  (live config points at this proxy)",
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  Takeover: "),
                    Span::styled(format!("{} off", name), Style::default().fg(Color::Gray)),
                ])
            }
        },
    ];

    // Failover queue (providers tried in order when the active one fails).
    let queue = state.provider_service.list_failover_queue().unwrap_or_default();
    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        format!("  Failover Queue ({})", queue.len()),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    if queue.is_empty() {
        text.push(Line::from(Span::styled(
            "    (empty — add providers with [f] on the Providers tab)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, p) in queue.iter().enumerate() {
            text.push(Line::from(format!("    {}. {}", i + 1, p.name)));
        }
    }

    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("  [s] ", Style::default().fg(Color::Cyan)),
        Span::raw("start proxy    "),
        Span::styled("[x] ", Style::default().fg(Color::Cyan)),
        Span::raw("stop proxy    "),
        Span::styled("[t] ", Style::default().fg(Color::Cyan)),
        Span::raw("toggle takeover"),
    ]));

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(" Proxy ".to_string()))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(paragraph, area);
}

/// Universal providers (cross-app shared base_url/api_key), selectable list.
fn render_universal(f: &mut Frame, area: Rect, state: &mut TuiState) {
    let providers = state.universal_service.list().unwrap_or_default();
    if providers.is_empty() {
        render_empty(
            f,
            area,
            " Universal Providers ",
            "No universal providers. Press [+]/[a] to add one (shared across apps).",
        );
        return;
    }

    let items: Vec<ListItem> = providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let selected = i == state.selected_index;
            let prefix = if selected { "> " } else { "  " };
            // App badges: lit letter when targeted, dim dot otherwise.
            let badge = |on: bool, ch: char| if on { ch } else { '·' };
            let apps = format!(
                "[{}{}{}]",
                badge(p.apps.claude, 'C'),
                badge(p.apps.codex, 'X'),
                badge(p.apps.gemini, 'G'),
            );
            ListItem::new(format!(
                "{}{} {:<18} {}",
                prefix,
                apps,
                truncate(&p.name, 18),
                truncate(&p.base_url, 44)
            ))
            .style(row_style(selected))
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(format!(
            " Universal ({})  [C]laude [X]codex [G]emini  [Enter] cycle apps  [e] edit  [+] add  [d] delete ",
            providers.len()
        )));
    f.render_widget(list, area);
}

/// OpenClaw workspace: whitelisted markdown files (top) + daily memory files
/// as a selectable list (bottom).
fn render_workspace(f: &mut Frame, area: Rect, state: &mut TuiState) {
    use olenro_core::openclaw_workspace as ws;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(0)])
        .split(area);

    // --- Workspace files (info) ---
    let files = ws::list_workspace_files();
    let mut text = vec![Line::from("")];
    for fi in &files {
        let (mark, color) = if fi.exists {
            ("✓", Color::Green)
        } else {
            ("·", Color::DarkGray)
        };
        let detail = if fi.exists {
            format!("{} bytes", fi.size_bytes)
        } else {
            "not created".to_string()
        };
        text.push(Line::from(vec![
            Span::styled(format!("  {} ", mark), Style::default().fg(color)),
            Span::raw(format!("{:<14} ", fi.filename)),
            Span::styled(detail, Style::default().fg(Color::DarkGray)),
        ]));
    }
    let info = Paragraph::new(text)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(" Workspace Files (~/.openclaw/workspace) ".to_string()));
    f.render_widget(info, chunks[0]);

    // --- Daily memory files (selectable) ---
    let memory = ws::list_daily_memory_files().unwrap_or_default();
    if memory.is_empty() {
        render_empty(
            f,
            chunks[1],
            " Daily Memory ",
            "No daily memory files under workspace/memory.",
        );
        return;
    }
    let items: Vec<ListItem> = memory
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let selected = i == state.selected_index;
            let prefix = if selected { "> " } else { "  " };
            let preview = m.preview.replace('\n', " ");
            ListItem::new(format!(
                "{}{:<14} {:>7} B  {}",
                prefix,
                m.date,
                m.size_bytes,
                truncate(&preview, 50)
            ))
            .style(row_style(selected))
        })
        .collect();
    let list = List::new(items)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(format!(
            " Daily Memory ({})  [Enter] preview  [d] delete ",
            memory.len()
        )));
    f.render_widget(list, chunks[1]);
}

/// OpenClaw env vars (`env` section of openclaw.json), selectable list.
fn render_openclaw_env(f: &mut Frame, area: Rect, state: &mut TuiState) {
    use olenro_core::app_config_writers::openclaw_config as oc;
    let vars = oc::get_env_config().map(|c| c.vars).unwrap_or_default();
    if vars.is_empty() {
        render_empty(
            f,
            area,
            " OpenClaw Env ",
            "No env vars. Press [+]/[a] to add one (writes ~/.openclaw/openclaw.json).",
        );
        return;
    }

    let mut keys: Vec<&String> = vars.keys().collect();
    keys.sort();
    let items: Vec<ListItem> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let selected = i == state.selected_index;
            let prefix = if selected { "> " } else { "  " };
            let val = match vars.get(*k) {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(v) => v.to_string(),
                None => String::new(),
            };
            ListItem::new(format!("{}{} = {}", prefix, k, truncate(&val, 60)))
                .style(row_style(selected))
        })
        .collect();

    let list = List::new(items)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(format!(
            " OpenClaw Env ({})  [+] add  [d] delete ",
            vars.len()
        )));
    f.render_widget(list, area);
}

/// OpenClaw tools config (`tools` section): profile + allow/deny lists.
fn render_openclaw_tools(f: &mut Frame, area: Rect, _state: &mut TuiState) {
    use olenro_core::app_config_writers::openclaw_config as oc;
    let tools = oc::get_tools_config().unwrap_or(oc::OpenClawToolsConfig {
        profile: None,
        allow: Vec::new(),
        deny: Vec::new(),
        extra: Default::default(),
    });

    let mut text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Profile:  "),
            Span::styled(
                tools.profile.clone().unwrap_or_else(|| "(none)".to_string()),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "   [p] cycle (minimal/coding/messaging/full/none)",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("  Allow ({})", tools.allow.len()),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )),
    ];
    if tools.allow.is_empty() {
        text.push(Line::from(Span::styled(
            "    (empty)",
            Style::default().fg(Color::DarkGray),
        )));
    }
    for entry in &tools.allow {
        text.push(Line::from(format!("    + {}", entry)));
    }
    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        format!("  Deny ({})", tools.deny.len()),
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    )));
    if tools.deny.is_empty() {
        text.push(Line::from(Span::styled(
            "    (empty)",
            Style::default().fg(Color::DarkGray),
        )));
    }
    for entry in &tools.deny {
        text.push(Line::from(format!("    - {}", entry)));
    }

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(" OpenClaw Tools ".to_string()))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(paragraph, area);
}

/// OpenClaw agent defaults (`agents.defaults`): default model + catalog.
fn render_openclaw_agents(f: &mut Frame, area: Rect, _state: &mut TuiState) {
    use olenro_core::app_config_writers::openclaw_config as oc;
    let model = oc::get_default_model().ok().flatten();
    let catalog = oc::get_model_catalog().ok().flatten().unwrap_or_default();

    let mut text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Default Model",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
    ];
    match &model {
        Some(m) => {
            text.push(Line::from(vec![
                Span::raw("    Primary:   "),
                Span::styled(
                    m.primary.clone(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("   [e] edit", Style::default().fg(Color::DarkGray)),
            ]));
            if m.fallbacks.is_empty() {
                text.push(Line::from("    Fallbacks: (none)"));
            } else {
                text.push(Line::from(format!(
                    "    Fallbacks: {}",
                    m.fallbacks.join(", ")
                )));
            }
        }
        None => text.push(Line::from(vec![
            Span::styled(
                "    Not set",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("   [e] set primary", Style::default().fg(Color::DarkGray)),
        ])),
    }

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        format!("  Model Catalog ({})", catalog.len()),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    if catalog.is_empty() {
        text.push(Line::from(Span::styled(
            "    (none)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let mut entries: Vec<(&String, &oc::OpenClawModelCatalogEntry)> = catalog.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        for (id, entry) in entries {
            let alias = entry
                .alias
                .as_deref()
                .map(|a| format!("  (alias: {})", a))
                .unwrap_or_default();
            text.push(Line::from(format!("    {}{}", id, alias)));
        }
    }

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(" OpenClaw Agent Defaults ".to_string()))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(paragraph, area);
}

/// Hermes memory blobs (MEMORY.md / USER.md): enabled state, char budget, preview.
fn render_hermes_memory(f: &mut Frame, area: Rect, state: &mut TuiState) {
    use olenro_core::app_config_writers::hermes_config as hc;

    let limits = hc::read_memory_limits().unwrap_or_default();
    let mem = hc::read_memory(hc::MemoryKind::Memory).unwrap_or_default();
    let user = hc::read_memory(hc::MemoryKind::User).unwrap_or_default();

    let rows = [
        ("MEMORY.md", mem.chars().count(), limits.memory, limits.memory_enabled, &mem),
        ("USER.md", user.chars().count(), limits.user, limits.user_enabled, &user),
    ];

    let mut text: Vec<Line> = vec![Line::from("")];
    for (i, (name, used, budget, enabled, content)) in rows.iter().enumerate() {
        let selected = i == state.selected_index;
        let prefix = if selected { "> " } else { "  " };
        let toggle = if *enabled { "[ON] " } else { "[off]" };
        let toggle_color = if *enabled { Color::Green } else { Color::DarkGray };
        let over = used > budget;
        let budget_color = if over { Color::Red } else { Color::White };
        text.push(Line::from(vec![
            Span::styled(
                format!("{}{} ", prefix, toggle),
                Style::default().fg(toggle_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<10} ", name),
                row_style(selected),
            ),
            Span::styled(
                format!("{} / {} chars", used, budget),
                Style::default().fg(budget_color),
            ),
            if over {
                Span::styled("  (over budget)", Style::default().fg(Color::Red))
            } else {
                Span::raw("")
            },
        ]));
        let preview = content.replace('\n', " ");
        text.push(Line::from(Span::styled(
            format!("      {}", truncate(&preview, 80)),
            Style::default().fg(Color::DarkGray),
        )));
        text.push(Line::from(""));
    }
    text.push(Line::from(Span::styled(
        "  [Enter] toggle enabled.  Edit content via `olenro` / the Hermes web UI.",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(" Hermes Memory ".to_string()))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn render_settings(f: &mut Frame, area: Rect, state: &mut TuiState) {
    let db_path = state.db_path.to_string_lossy();
    let config_dir = state
        .db_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut text = vec![
        Line::from(""),
        Line::from("  Configuration"),
        Line::from(""),
        Line::from(format!("  Database:   {}", db_path)),
        Line::from(format!("  Config Dir: {}", config_dir)),
        Line::from(format!("  App:        {}", state.active_app_name())),
        Line::from(""),
    ];

    // Git sync status.
    let git = olenro_core::git_sync::status();
    text.push(Line::from(Span::styled(
        "  Git Sync",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    if git.is_repo {
        let dirty = if git.dirty { "dirty" } else { "clean" };
        let dirty_color = if git.dirty { Color::Yellow } else { Color::Green };
        text.push(Line::from(vec![
            Span::raw(format!(
                "    Branch: {}   Remote: {}   ",
                git.branch.as_deref().unwrap_or("-"),
                git.remote.as_deref().unwrap_or("(none)"),
            )),
            Span::styled(dirty, Style::default().fg(dirty_color)),
        ]));
    } else {
        text.push(Line::from(Span::styled(
            "    Not a git repo — press [i] to configure (set remote + branch).",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Backups.
    let backups = olenro_core::git_sync::list_backups();
    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        format!("  Backups ({})", backups.len()),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    for b in backups.iter().take(3) {
        text.push(Line::from(format!(
            "    {}  ({} B)",
            b.filename, b.size_bytes
        )));
    }

    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled("  [g] ", Style::default().fg(Color::Cyan)),
        Span::raw("push    "),
        Span::styled("[p] ", Style::default().fg(Color::Cyan)),
        Span::raw("pull    "),
        Span::styled("[b] ", Style::default().fg(Color::Cyan)),
        Span::raw("backup    "),
        Span::styled("[i] ", Style::default().fg(Color::Cyan)),
        Span::raw("configure"),
    ]));

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(BG).fg(Color::White))
        .block(panel(" Settings ".to_string()))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn render_status(f: &mut Frame, area: Rect, state: &TuiState) {
    let help: &str = if state.dialog_mode != DialogMode::None {
        "[Tab/Shift+Tab] field  [Enter] submit  [Esc] cancel"
    } else {
        match state.current_tab {
            Tab::Providers => {
                "[↑↓] select  [Enter/s] switch  [e] edit  [f] failover  [+] add  [d] del  [Tab] panel  [q] quit"
            }
            Tab::Mcp => {
                "[↑↓] select  [Enter] toggle  [s] sync  [+] add  [d] delete  [Tab] panel  [q] quit"
            }
            Tab::Prompts => {
                "[↑↓] select  [Enter/s] enable  [e] edit  [+] add  [d] delete  [Tab] panel  [q] quit"
            }
            Tab::Skills => {
                "[↑↓] select  [Enter] update  [s] sync→app  [+] install  [d] uninstall  [Tab] panel  [q] quit"
            }
            Tab::Agents => {
                "[↑↓] select  [Enter] details  [+] add  [d] delete  [Tab] panel  [q] quit"
            }
            Tab::Discover => {
                "[↑↓] select  [Enter/i] install  [s] search  [r] popular  [Tab] panel  [q] quit"
            }
            Tab::Sessions => {
                "[↑↓] select  [Enter] details  [r] resume  [d] delete  [Tab] panel  [q] quit"
            }
            Tab::Proxy => "[s] start  [x] stop  [t] takeover  [Tab] panel  [q] quit",
            Tab::Universal => {
                "[↑↓] select  [Enter] cycle apps  [e] edit  [+] add  [d] delete  [Tab] panel  [q] quit"
            }
            Tab::Workspace => {
                "[↑↓] select  [Enter] preview  [d] delete memory  [/] app  [Tab] panel  [q] quit"
            }
            Tab::OpenClawEnv => {
                "[↑↓] select  [+] add  [d] delete  [/] app  [Tab] panel  [q] quit"
            }
            Tab::OpenClawTools => "[p] cycle profile  [/] app  [Tab] panel  [q] quit",
            Tab::OpenClawAgents => "[e] set primary model  [/] app  [Tab] panel  [q] quit",
            Tab::HermesMemory => {
                "[↑↓] select  [Enter] toggle enabled  [/] app  [Tab] panel  [q] quit"
            }
            Tab::Settings => {
                "[g] push  [p] pull  [b] backup  [i] configure  [/] app  [Tab] panel  [q] quit"
            }
            // Read-only panels: don't advertise add/edit/delete.
            Tab::Dashboard | Tab::Usage => {
                "[/] switch app  [Tab/Shift+Tab] switch panel  [q] quit"
            }
        }
    };

    let line = if state.status_message.is_empty() {
        Line::from(Span::styled(
            format!(" {}", help),
            Style::default().fg(Color::Cyan),
        ))
    } else {
        Line::from(vec![
            Span::styled(
                format!(" {} ", state.status_message),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" | {}", help), Style::default().fg(Color::Cyan)),
        ])
    };

    let status = Paragraph::new(line)
        .style(Style::default().bg(BG))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, area);
}

/// Compute a centered rectangle of the given size within `area`.
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width: popup_width,
        height: height.min(area.height),
    }
}

/// Render a labeled input field; highlights when active.
fn field_line<'a>(label: &'a str, value: &'a str, active: bool) -> Line<'a> {
    let label_style = if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let value_display = if active {
        format!("{}_", value)
    } else {
        value.to_string()
    };
    Line::from(vec![
        Span::styled(format!("  {:<10} ", label), label_style),
        Span::raw(value_display),
    ])
}

/// Like [`field_line`] but for a dynamically-labeled template field (owned
/// label that does not borrow from state).
fn template_field_line(label: &str, value: &str, active: bool) -> Line<'static> {
    let label_style = if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let value_display = if active {
        format!("{}_", value)
    } else {
        value.to_string()
    };
    Line::from(vec![
        Span::styled(format!("  {:<14} ", format!("{}:", label)), label_style),
        Span::raw(value_display),
    ])
}

/// Short label for a provider category (reuses the add-dialog category table).
fn category_label(cat: olenro_core::provider::ProviderCategory) -> &'static str {
    CATEGORIES
        .iter()
        .find(|(_, c)| *c == cat)
        .map(|(label, _)| *label)
        .unwrap_or("custom")
}

fn render_dialog(f: &mut Frame, area: Rect, state: &TuiState) {
    let (title, lines, height): (&str, Vec<Line>, u16) = match &state.dialog_mode {
        DialogMode::AddProvider => match state.add_stage {
            // Stage 1: choose a preset (or "Custom") for the active app.
            AddProviderStage::SelectPreset => {
                let presets = state.add_provider_presets();
                let total = presets.len() + 1;
                let cursor = state.preset_cursor;
                const WINDOW: usize = 12;
                let start = cursor
                    .saturating_sub(WINDOW / 2)
                    .min(total.saturating_sub(WINDOW.min(total)));
                let end = (start + WINDOW).min(total);

                let mut lines = vec![Line::from(Span::styled(
                    format!("  Provider preset for {}:", state.active_app_name()),
                    Style::default().fg(Color::Gray),
                ))];
                for row in start..end {
                    let selected = row == cursor;
                    let marker = if selected { "› " } else { "  " };
                    let (label, tag) = if row == 0 {
                        ("Custom (blank form)".to_string(), String::new())
                    } else {
                        let p = &presets[row - 1];
                        let mut tag = format!("[{}]", category_label(p.category()));
                        if p.is_official() {
                            tag.push_str(" official");
                        } else if p.is_partner() {
                            tag.push_str(" partner");
                        }
                        (p.name().to_string(), tag)
                    };
                    let style = if selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("{}{}", marker, label), style),
                        Span::styled(format!("  {}", tag), Style::default().fg(Color::DarkGray)),
                    ]));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!(
                        "  {}/{}   [↑/↓] move  [Enter] select  [Esc] cancel",
                        cursor + 1,
                        total
                    ),
                    Style::default().fg(Color::DarkGray),
                )));
                let height = (lines.len() as u16) + 2;
                (" Add Provider — Preset ", lines, height)
            }
            // Stage 2: fill the (preset-prefilled) form fields.
            AddProviderStage::Fields => {
                let mut lines = vec![Line::from("")];
                lines.push(field_line("Name:", &state.name_input, state.input_field == 0));
                lines.push(field_line(
                    "Endpoint:",
                    &state.endpoint_input,
                    state.input_field == 1,
                ));
                lines.push(field_line(
                    "API Key:",
                    &state.api_key_input,
                    state.input_field == 2,
                ));

                match state.chosen_preset() {
                    // Preset: one input per template variable, fixed category.
                    Some(preset) => {
                        for (i, tf) in preset.template_fields().iter().enumerate() {
                            let active = state.input_field == 3 + i;
                            let value = state
                                .template_inputs
                                .get(i)
                                .map(String::as_str)
                                .unwrap_or("");
                            lines.push(template_field_line(&tf.label, value, active));
                        }
                        lines.push(Line::from(""));
                        if let Some(url) = preset.api_key_url() {
                            lines.push(Line::from(Span::styled(
                                format!("  Get an API key: {}", url),
                                Style::default().fg(Color::DarkGray),
                            )));
                        }
                        lines.push(Line::from(Span::styled(
                            "  [Tab] next  [Enter] save  [Esc] back",
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                    // Custom: editable category selector on field 3.
                    None => {
                        let cat = CATEGORIES[state.category_index].0;
                        let active = state.input_field == 3;
                        let style = if active {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Gray)
                        };
                        lines.push(Line::from(vec![
                            Span::styled("  Category:  ", style),
                            Span::raw(if active {
                                format!("< {} >", cat)
                            } else {
                                cat.to_string()
                            }),
                        ]));
                        lines.push(Line::from(""));
                        lines.push(Line::from(Span::styled(
                            "  [Tab] next  [←/→] category  [Enter] save  [Esc] back",
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }
                let height = (lines.len() as u16) + 2;
                (" Add Provider ", lines, height)
            }
        },
        DialogMode::EditProvider(_) => (
            " Edit Provider ",
            vec![
                Line::from(""),
                field_line("Name:", &state.name_input, state.input_field == 0),
                field_line("Endpoint:", &state.endpoint_input, state.input_field == 1),
                Line::from(""),
                Line::from(Span::styled(
                    "  [Tab] next  [Enter] save  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            8,
        ),
        DialogMode::AddMcp => (
            " Add MCP Server ",
            vec![
                Line::from(""),
                field_line("Name:", &state.name_input, state.input_field == 0),
                field_line("Command:", &state.command_input, state.input_field == 1),
                field_line("Args:", &state.args_input, state.input_field == 2),
                Line::from(""),
                Line::from(Span::styled(
                    "  Args are space-separated.  [Tab] next  [Enter] save  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            9,
        ),
        DialogMode::AddPrompt => (
            " Add Prompt ",
            vec![
                Line::from(""),
                field_line("Name:", &state.name_input, state.input_field == 0),
                field_line("Content:", &state.content_input, state.input_field == 1),
                Line::from(""),
                Line::from(Span::styled(
                    "  [Tab] next  [Enter] save  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            8,
        ),
        DialogMode::EditPrompt(_) => (
            " Edit Prompt ",
            vec![
                Line::from(""),
                field_line("Content:", &state.content_input, true),
                Line::from(""),
                Line::from(Span::styled(
                    "  [Enter] save  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            7,
        ),
        DialogMode::AddSkill => (
            " Install Skill ",
            vec![
                Line::from(""),
                field_line("Repo:", &state.name_input, true),
                Line::from(""),
                Line::from(Span::styled(
                    "  Format: owner/name   [Enter] install  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            7,
        ),
        DialogMode::AddOpenClawEnv => (
            " Add OpenClaw Env Var ",
            vec![
                Line::from(""),
                field_line("Name:", &state.name_input, state.input_field == 0),
                field_line("Value:", &state.content_input, state.input_field == 1),
                Line::from(""),
                Line::from(Span::styled(
                    "  Value may be JSON (e.g. true, 42, \"text\").  [Tab] next  [Enter] save  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            8,
        ),
        DialogMode::SetOpenClawPrimaryModel => (
            " Set Default Model ",
            vec![
                Line::from(""),
                field_line("Primary:", &state.content_input, true),
                Line::from(""),
                Line::from(Span::styled(
                    "  Format: provider/model   [Enter] save  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            7,
        ),
        DialogMode::AddUniversal => (
            " Add Universal Provider ",
            vec![
                Line::from(""),
                field_line("Name:", &state.name_input, state.input_field == 0),
                field_line("Base URL:", &state.endpoint_input, state.input_field == 1),
                field_line("API Key:", &state.api_key_input, state.input_field == 2),
                Line::from(""),
                Line::from(Span::styled(
                    "  Shared across apps; pick targets with [Enter] after adding.  [Tab] next  [Enter] save  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            9,
        ),
        DialogMode::EditUniversal(_) => (
            " Edit Universal Provider ",
            vec![
                Line::from(""),
                field_line("Name:", &state.name_input, state.input_field == 0),
                field_line("Base URL:", &state.endpoint_input, state.input_field == 1),
                Line::from(""),
                Line::from(Span::styled(
                    "  [Tab] next  [Enter] save  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            8,
        ),
        DialogMode::ConfigureGitSync => (
            " Configure Git Sync ",
            vec![
                Line::from(""),
                field_line("Remote:", &state.endpoint_input, state.input_field == 0),
                field_line("Branch:", &state.name_input, state.input_field == 1),
                Line::from(""),
                Line::from(Span::styled(
                    "  Remote git URL (optional) + branch.  [Tab] next  [Enter] save  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            8,
        ),
        DialogMode::SearchSkills => (
            " Search skills.sh ",
            vec![
                Line::from(""),
                field_line("Query:", &state.name_input, true),
                Line::from(""),
                Line::from(Span::styled(
                    "  At least 2 characters.  [Enter] search  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            7,
        ),
        DialogMode::AddAgent => (
            " Create Subagent ",
            vec![
                Line::from(""),
                field_line("Name:", &state.name_input, state.input_field == 0),
                field_line("Describe:", &state.content_input, state.input_field == 1),
                Line::from(""),
                Line::from(Span::styled(
                    "  Writes ~/.claude/agents/<name>.md; edit it for the full prompt.  [Tab] next  [Enter] save  [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            8,
        ),
        DialogMode::DeleteProvider(_)
        | DialogMode::DeleteMcp(_)
        | DialogMode::DeletePrompt(_)
        | DialogMode::DeleteSkill(_)
        | DialogMode::DeleteSession(_)
        | DialogMode::DeleteOpenClawEnv(_)
        | DialogMode::DeleteOpenClawMemory(_)
        | DialogMode::DeleteUniversal(_)
        | DialogMode::DeleteAgent(_, _) => (
            " Confirm Delete ",
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Are you sure you want to delete this item?",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  [Enter] confirm    [Esc] cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ],
            7,
        ),
        DialogMode::None => return,
    };

    let popup = centered_rect(60, height, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Rgb(30, 32, 44)))
        .title(title);
    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(Color::Rgb(30, 32, 44)).fg(Color::White))
        .block(block)
        .alignment(Alignment::Left);
    f.render_widget(paragraph, popup);
}
