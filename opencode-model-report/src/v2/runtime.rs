use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;

use super::{
    ljust, load_report_rows, resolve_config_home, rjust, LoadError, UiAction, UiKey, UiMode,
    UiState,
};

pub struct Cli {
    pub home_dir: Option<PathBuf>,
}

#[derive(Debug)]
pub enum RuntimeError {
    ResolveHome(String),
    Load(LoadError),
    Terminal(String),
    Io(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::ResolveHome(msg) => write!(f, "{}", msg),
            RuntimeError::Load(err) => write!(f, "{}", err),
            RuntimeError::Terminal(msg) => write!(f, "{}", msg),
            RuntimeError::Io(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl RuntimeError {
    pub fn exit_code(&self) -> i32 {
        match self {
            RuntimeError::Load(err) => err.exit_code(),
            _ => 3,
        }
    }
}

enum WorkerMessage {
    Loaded(Result<Vec<super::ModelRow>, LoadError>, bool),
}

pub fn run(cli: Cli) -> Result<(), RuntimeError> {
    let home_dir = resolve_config_home(cli.home_dir.as_deref())
        .map_err(|err| RuntimeError::ResolveHome(err.to_string()))?;

    let (tx, rx) = mpsc::channel();
    spawn_load(tx.clone(), home_dir.clone(), true);

    enable_raw_mode().map_err(|err| RuntimeError::Terminal(err.to_string()))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)
        .map_err(|err| RuntimeError::Terminal(err.to_string()))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|err| RuntimeError::Terminal(err.to_string()))?;
    let _cleanup = TerminalCleanup;

    let mut state = UiState::new();
    let mut fatal: Option<LoadError> = None;

    let result = run_loop(&mut terminal, &mut state, &rx, &tx, &home_dir, &mut fatal);

    result?;
    if let Some(err) = fatal {
        return Err(RuntimeError::Load(err));
    }

    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &mut UiState,
    rx: &Receiver<WorkerMessage>,
    tx: &Sender<WorkerMessage>,
    home_dir: &PathBuf,
    fatal: &mut Option<LoadError>,
) -> Result<(), RuntimeError> {
    loop {
        while let Ok(message) = rx.try_recv() {
            match message {
                WorkerMessage::Loaded(result, initial) => match result {
                    Ok(rows) => {
                        state.apply_snapshot(rows);
                        if initial {
                            *fatal = None;
                        }
                    }
                    Err(err) => {
                        state.apply_refresh_error(err.to_string());
                        if initial {
                            *fatal = Some(err);
                        }
                    }
                },
            }
        }

        terminal
            .draw(|frame| draw(frame, state))
            .map_err(|err| RuntimeError::Terminal(err.to_string()))?;

        if fatal.is_some() {
            return Ok(());
        }

        if event::poll(Duration::from_millis(100))
            .map_err(|err| RuntimeError::Terminal(err.to_string()))?
        {
            if let Event::Key(key) =
                event::read().map_err(|err| RuntimeError::Terminal(err.to_string()))?
            {
                let ui_key = match key.code {
                    KeyCode::Char('q') => Some(UiKey::Quit),
                    KeyCode::Char('r') => Some(UiKey::Refresh),
                    KeyCode::Char('s') => Some(UiKey::CycleSort),
                    _ => None,
                };

                if let Some(ui_key) = ui_key {
                    match state.handle_key(ui_key) {
                        UiAction::Quit => return Ok(()),
                        UiAction::Refresh => spawn_load(tx.clone(), home_dir.clone(), false),
                        UiAction::None => {}
                    }
                }
            }
        }
    }
}

fn spawn_load(tx: Sender<WorkerMessage>, home_dir: PathBuf, initial: bool) {
    thread::spawn(move || {
        let result = load_report_rows(&home_dir);
        let _ = tx.send(WorkerMessage::Loaded(result, initial));
    });
}

fn draw(frame: &mut ratatui::Frame, state: &UiState) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);

    let header = Paragraph::new(header_line(state))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(header_border_style(state)),
        )
        .alignment(ratatui::layout::Alignment::Left);
    frame.render_widget(header, chunks[0]);

    let report = if state.snapshot.is_empty() && matches!(state.mode, UiMode::Loading) {
        loading_view(state)
    } else {
        report_view(&state.visible_rows(), state.sort_mode)
    };

    let panel = Paragraph::new(report)
        .block(
            Block::default()
                .title("model inventory")
                .borders(Borders::ALL)
                .border_style(body_border_style(state)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(panel, chunks[1]);

    let footer = Paragraph::new(Text::from(footer_lines(state)))
        .style(Style::default().fg(Color::Rgb(148, 163, 184)))
        .alignment(ratatui::layout::Alignment::Left);
    frame.render_widget(footer, chunks[2]);
}

fn header_line(state: &UiState) -> Line<'static> {
    let mut spans = vec![Span::styled(
        " opencode-model-report ",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )];

    spans.push(Span::raw("• "));
    spans.push(Span::styled(
        format!("sort: {}", sort_mode_label(state.sort_mode)),
        sort_badge_style(state.sort_mode),
    ));
    spans.push(Span::raw("  "));
    spans.push(Span::styled("q quit", key_hint_style()));
    spans.push(Span::raw("  "));
    spans.push(Span::styled("r refresh", key_hint_style()));
    spans.push(Span::raw("  "));
    spans.push(Span::styled("s sort", key_hint_style()));

    Line::from(spans)
}

fn loading_view(state: &UiState) -> Text<'static> {
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(" ⟳ ", loading_style()),
        Span::styled("Loading model data", loading_style()),
    ]));
    lines.push(Line::from(vec![Span::styled(
        state.status.clone(),
        status_style(state.mode, false),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "Fetching config, inventory, and costs.",
        muted_style(),
    )]));
    Text::from(lines)
}

fn report_view(rows: &[super::ModelRow], sort_mode: super::SortMode) -> Text<'static> {
    let (model_width, active_width, input_width, output_width, prefix_width) = table_widths(rows);
    let mut lines = Vec::new();

    lines.push(table_header_line(
        model_width,
        active_width,
        input_width,
        output_width,
        sort_mode,
    ));

    for (index, row) in rows.iter().enumerate() {
        let row_style = row_style(index, row.active);
        let mut spans = vec![
            Span::styled(ljust(&row.model, model_width), model_style(row.active)),
            Span::raw("  "),
            Span::styled(
                ljust(if row.active { "yes" } else { "no" }, active_width),
                active_badge_style(row.active),
            ),
            Span::raw("  "),
            Span::styled(
                rjust(&super::format_cost(row.input_cost), input_width),
                cost_style(row.input_cost.is_some(), CostKind::Input),
            ),
            Span::raw("  "),
            Span::styled(
                rjust(&super::format_cost(row.output_cost), output_width),
                cost_style(row.output_cost.is_some(), CostKind::Output),
            ),
            Span::raw("  "),
        ];

        let usage_groups = wrap_usage_labels(&row.usage, 50);
        if let Some(first_group) = usage_groups.first() {
            spans.extend(usage_group_spans(first_group));
        }
        lines.push(Line::from(spans).style(row_style));

        for group in usage_groups.iter().skip(1) {
            let mut continuation = vec![Span::raw(" ".repeat(prefix_width))];
            continuation.extend(usage_group_spans(group));
            lines.push(Line::from(continuation).style(row_style));
        }
    }

    Text::from(lines)
}

fn footer_lines(state: &UiState) -> Vec<Line<'static>> {
    let status_line = Line::from(vec![Span::styled(
        state.status.clone(),
        status_style(state.mode, state.status.contains("failed")),
    )]);
    let legend_line = Line::from(vec![
        Span::styled("usage legend: ", muted_style()),
        Span::styled(
            "OpenCode default / small_model",
            usage_style(super::UsageSource::OpenCodeDefault),
        ),
        Span::raw(" / "),
        Span::styled(
            "OpenCode agents",
            usage_style(super::UsageSource::OpenCodeCustom),
        ),
        Span::raw(" / "),
        Span::styled("Weave agents", usage_style(super::UsageSource::Weave)),
        Span::raw(" / "),
        Span::styled(
            "Weave custom_agents",
            usage_style(super::UsageSource::WeaveCustom),
        ),
        Span::raw("  •  "),
        Span::styled("sorted", muted_style()),
    ]);

    vec![status_line, legend_line]
}

fn table_header_line(
    model_width: usize,
    active_width: usize,
    input_width: usize,
    output_width: usize,
    sort_mode: super::SortMode,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(ljust("MODEL", model_width), table_header_style()),
        Span::raw("  "),
        Span::styled(ljust("ACTIVE", active_width), table_header_style()),
        Span::raw("  "),
        Span::styled(rjust("IN", input_width), table_header_style()),
        Span::raw("  "),
        Span::styled(rjust("OUT", output_width), table_header_style()),
        Span::raw("  "),
        Span::styled(
            format!("USAGE  [{}]", sort_mode_label(sort_mode)),
            table_header_style(),
        ),
    ])
    .style(Style::default().bg(Color::Rgb(20, 24, 35)))
}

fn table_widths(rows: &[super::ModelRow]) -> (usize, usize, usize, usize, usize) {
    let model_width = std::iter::once("MODEL".len())
        .chain(rows.iter().map(|row| row.model.len()))
        .max()
        .unwrap_or(0);
    let active_width = "ACTIVE".len();
    let input_width = std::iter::once("IN".len())
        .chain(
            rows.iter()
                .map(|row| super::format_cost(row.input_cost).len()),
        )
        .max()
        .unwrap_or(0);
    let output_width = std::iter::once("OUT".len())
        .chain(
            rows.iter()
                .map(|row| super::format_cost(row.output_cost).len()),
        )
        .max()
        .unwrap_or(0);
    let prefix_width = model_width + 2 + active_width + 2 + input_width + 2 + output_width + 2;

    (
        model_width,
        active_width,
        input_width,
        output_width,
        prefix_width,
    )
}

fn wrap_usage_labels(labels: &[super::UsageLabel], width: usize) -> Vec<Vec<super::UsageLabel>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    let mut current_len = 0usize;

    for label in labels.iter().cloned() {
        let label_len = label.label.len();
        let extra = if current.is_empty() {
            label_len
        } else {
            2 + label_len
        };

        if !current.is_empty() && current_len + extra > width {
            groups.push(current);
            current = Vec::new();
            current_len = 0;
        }

        if !current.is_empty() {
            current_len += 2;
        }
        current_len += label_len;
        current.push(label);
    }

    if !current.is_empty() {
        groups.push(current);
    }

    if groups.is_empty() {
        groups.push(Vec::new());
    }

    groups
}

fn usage_group_spans(group: &[super::UsageLabel]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (idx, label) in group.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(", ", muted_style()));
        }
        spans.push(Span::styled(label.label.clone(), usage_style(label.source)));
    }
    spans
}

fn usage_style(source: super::UsageSource) -> Style {
    match source {
        super::UsageSource::OpenCodeDefault => Style::default().fg(Color::Blue),
        super::UsageSource::OpenCodeCustom => Style::default().fg(Color::Red),
        super::UsageSource::Weave => Style::default().fg(Color::Green),
        super::UsageSource::WeaveCustom => Style::default().fg(Color::Yellow),
    }
}

fn sort_badge_style(sort_mode: super::SortMode) -> Style {
    match sort_mode {
        super::SortMode::ActiveFirst => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        super::SortMode::CostAsc => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        super::SortMode::CostDesc => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        super::SortMode::ModelName => Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    }
}

fn status_style(mode: UiMode, has_error: bool) -> Style {
    if has_error {
        return Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
    }
    match mode {
        UiMode::Loading => Style::default().fg(Color::DarkGray),
        UiMode::Refreshing => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        UiMode::Ready => Style::default().fg(Color::DarkGray),
    }
}

fn loading_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

fn table_header_style() -> Style {
    Style::default()
        .fg(Color::Rgb(215, 223, 255))
        .bg(Color::Rgb(20, 24, 35))
        .add_modifier(Modifier::BOLD)
}

fn row_style(index: usize, active: bool) -> Style {
    let zebra = if index % 2 == 0 {
        Color::Reset
    } else {
        Color::Rgb(18, 22, 31)
    };
    let accent = if active {
        Color::Rgb(21, 33, 31)
    } else {
        Color::Reset
    };

    Style::default().bg(match (zebra, accent) {
        (_, Color::Rgb(r, g, b)) if active => Color::Rgb(r, g, b),
        (Color::Rgb(r, g, b), _) => Color::Rgb(r, g, b),
        _ => Color::Reset,
    })
}

fn model_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(148, 163, 184))
    }
}

fn active_badge_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

enum CostKind {
    Input,
    Output,
}

fn cost_style(known: bool, kind: CostKind) -> Style {
    if !known {
        return Style::default().fg(Color::DarkGray);
    }
    match kind {
        CostKind::Input => Style::default().fg(Color::Cyan),
        CostKind::Output => Style::default().fg(Color::Magenta),
    }
}

fn key_hint_style() -> Style {
    Style::default()
        .fg(Color::Rgb(148, 163, 184))
        .add_modifier(Modifier::DIM)
}

fn muted_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn header_border_style(state: &UiState) -> Style {
    match state.mode {
        UiMode::Loading => Style::default().fg(Color::DarkGray),
        UiMode::Refreshing => Style::default().fg(Color::Cyan),
        UiMode::Ready => Style::default().fg(Color::Rgb(88, 128, 255)),
    }
}

fn body_border_style(state: &UiState) -> Style {
    if state.status.contains("failed") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Rgb(88, 128, 255))
    }
}

fn sort_mode_label(mode: super::SortMode) -> &'static str {
    match mode {
        super::SortMode::ActiveFirst => "active-first",
        super::SortMode::CostAsc => "cost-asc",
        super::SortMode::CostDesc => "cost-desc",
        super::SortMode::ModelName => "model-name",
    }
}

fn restore_terminal() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, Show, LeaveAlternateScreen);
}

struct TerminalCleanup;

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        restore_terminal();
        disable_raw_mode().ok();
    }
}

#[cfg(test)]
mod tests {
    use super::{footer_lines, sort_badge_style, status_style, usage_style};
    use crate::v2::{SortMode, UiMode, UsageSource};
    use ratatui::style::Color;

    #[test]
    fn usage_style_should_colour_sources_differently() {
        assert_eq!(
            usage_style(UsageSource::OpenCodeDefault).fg,
            Some(Color::Blue)
        );
        assert_eq!(
            usage_style(UsageSource::OpenCodeCustom).fg,
            Some(Color::Red)
        );
        assert_eq!(usage_style(UsageSource::Weave).fg, Some(Color::Green));
        assert_eq!(
            usage_style(UsageSource::WeaveCustom).fg,
            Some(Color::Yellow)
        );
    }

    #[test]
    fn sort_badge_style_should_use_distinct_palette() {
        assert_eq!(
            sort_badge_style(SortMode::ActiveFirst).fg,
            Some(Color::Green)
        );
        assert_eq!(sort_badge_style(SortMode::CostAsc).fg, Some(Color::Cyan));
        assert_eq!(sort_badge_style(SortMode::CostDesc).fg, Some(Color::Yellow));
        assert_eq!(sort_badge_style(SortMode::ModelName).fg, Some(Color::Blue));
    }

    #[test]
    fn status_style_should_signal_loading_refresh_and_error() {
        assert_eq!(
            status_style(UiMode::Loading, false).fg,
            Some(Color::DarkGray)
        );
        assert_eq!(
            status_style(UiMode::Refreshing, false).fg,
            Some(Color::Cyan)
        );
        assert_eq!(status_style(UiMode::Ready, true).fg, Some(Color::Red));
    }

    #[test]
    fn footer_lines_should_include_usage_legend_on_separate_line() {
        let lines = footer_lines(&crate::v2::UiState::new());
        assert_eq!(lines.len(), 2);

        let legend_text = lines[1]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(legend_text.contains("usage legend"));
        assert!(legend_text.contains("OpenCode default / small_model"));
        assert!(legend_text.contains("OpenCode agents"));
        assert!(legend_text.contains("Weave agents"));
        assert!(legend_text.contains("Weave custom_agents"));
    }
}
