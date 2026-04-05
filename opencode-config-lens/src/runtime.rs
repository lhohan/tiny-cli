use std::io::{self, Stdout};
use std::path::{Path, PathBuf};
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

mod paging;
use paging::PageState;

use crate::{
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
    Loaded(Result<Vec<crate::ModelRow>, LoadError>, bool),
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
    let mut paging = PageState::new();
    let mut fatal: Option<LoadError> = None;

    let result = run_loop(
        &mut terminal,
        &mut state,
        &mut paging,
        &rx,
        &tx,
        &home_dir,
        &mut fatal,
    );

    result?;
    if let Some(err) = fatal {
        return Err(RuntimeError::Load(err));
    }

    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &mut UiState,
    paging: &mut PageState,
    rx: &Receiver<WorkerMessage>,
    tx: &Sender<WorkerMessage>,
    home_dir: &Path,
    fatal: &mut Option<LoadError>,
) -> Result<(), RuntimeError> {
    loop {
        while let Ok(message) = rx.try_recv() {
            match message {
                WorkerMessage::Loaded(result, initial) => match result {
                    Ok(rows) => handle_loaded_rows(state, paging, fatal, rows, initial),
                    Err(err) => handle_load_error(state, fatal, err, initial),
                },
            }
        }

        terminal
            .draw(|frame| draw(frame, state, paging))
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
                if let Some(action) = handle_key(key.code, state, paging) {
                    match action {
                        UiAction::Quit => return Ok(()),
                        UiAction::Refresh => spawn_load(tx.clone(), home_dir.to_path_buf(), false),
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

fn handle_key(code: KeyCode, state: &mut UiState, paging: &mut PageState) -> Option<UiAction> {
    match code {
        KeyCode::Char('q') => Some(UiAction::Quit),
        KeyCode::Char('r') => Some(state.handle_key(UiKey::Refresh)),
        KeyCode::Char('s') => {
            let _ = state.handle_key(UiKey::CycleSort);
            paging.reset();
            None
        }
        KeyCode::Char('j') => {
            paging.next_page();
            None
        }
        KeyCode::Char('k') => {
            paging.previous_page();
            None
        }
        _ => None,
    }
}

fn handle_loaded_rows(
    state: &mut UiState,
    paging: &mut PageState,
    fatal: &mut Option<LoadError>,
    rows: Vec<crate::ModelRow>,
    initial: bool,
) {
    state.apply_snapshot(rows);
    paging.reset();
    if initial {
        *fatal = None;
    }
}

fn handle_load_error(
    state: &mut UiState,
    fatal: &mut Option<LoadError>,
    err: LoadError,
    initial: bool,
) {
    state.apply_refresh_error(err.to_string());
    if initial {
        *fatal = Some(err);
    }
}

fn draw(frame: &mut ratatui::Frame, state: &UiState, paging: &mut PageState) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);

    let report_width = chunks[1].width.saturating_sub(2) as usize;
    let viewport_height = chunks[1].height.saturating_sub(2) as usize;
    let rows = state.visible_rows();
    let layout = ReportLayout::new(&rows, report_width);
    let row_heights = report_row_heights(&rows, &layout);
    paging.set_viewport_height(viewport_height);
    paging.clamp_to_rows(&row_heights);
    let page_label = paging.page_label(&row_heights);

    let header = Paragraph::new(header_line(state))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(header_border_style(state)),
        )
        .alignment(ratatui::layout::Alignment::Left);
    frame.render_widget(header, chunks[0]);

    let report = if rows.is_empty() && matches!(state.mode, UiMode::Loading) {
        loading_view(state)
    } else {
        let page_range = paging.page_range(&row_heights);
        report_view(&rows[page_range], state.sort_mode, &layout)
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

    let footer = Paragraph::new(Text::from(footer_lines(state, &page_label)))
        .style(Style::default().fg(Color::Rgb(148, 163, 184)))
        .alignment(ratatui::layout::Alignment::Left);
    frame.render_widget(footer, chunks[2]);
}

fn header_line(state: &UiState) -> Line<'static> {
    let mut spans = vec![Span::styled(
        " OpenCode Config Lens ",
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
    spans.push(Span::raw("  "));
    spans.push(Span::styled("j next", key_hint_style()));
    spans.push(Span::raw("  "));
    spans.push(Span::styled("k previous", key_hint_style()));

    Line::from(spans)
}

fn loading_view(state: &UiState) -> Text<'static> {
    let lines = vec![
        Line::from(vec![
            Span::styled(" ⟳ ", loading_style()),
            Span::styled("Loading model data", loading_style()),
        ]),
        Line::from(vec![Span::styled(
            state.status.clone(),
            status_style(state.mode, false),
        )]),
        Line::from(vec![Span::styled(
            "Fetching config, inventory, and costs.",
            muted_style(),
        )]),
    ];
    Text::from(lines)
}

#[derive(Clone, Copy)]
struct ReportLayout {
    provider_width: usize,
    model_width: usize,
    input_width: usize,
    output_width: usize,
    prefix_width: usize,
    usage_width: usize,
}

impl ReportLayout {
    fn new(rows: &[crate::ModelRow], available_width: usize) -> Self {
        let (provider_width, model_width, input_width, output_width, prefix_width) =
            table_widths(rows);

        Self {
            provider_width,
            model_width,
            input_width,
            output_width,
            prefix_width,
            usage_width: available_width.saturating_sub(prefix_width).max(1),
        }
    }
}

fn report_view(
    rows: &[crate::ModelRow],
    sort_mode: crate::SortMode,
    layout: &ReportLayout,
) -> Text<'static> {
    let mut lines = Vec::new();

    lines.push(table_header_line(layout, sort_mode));

    for (index, row) in rows.iter().enumerate() {
        let row_style = row_style(index, row.active);
        let mut spans = vec![
            Span::styled(
                ljust(&row.provider, layout.provider_width),
                model_style(row.active),
            ),
            Span::raw("  "),
            Span::styled(
                ljust(&row.model_name, layout.model_width),
                model_style(row.active),
            ),
            Span::raw("  "),
            Span::styled(
                rjust(&crate::format_cost(row.input_cost), layout.input_width),
                cost_style(row.input_cost.is_some(), CostKind::Input),
            ),
            Span::raw("  "),
            Span::styled(
                rjust(&crate::format_cost(row.output_cost), layout.output_width),
                cost_style(row.output_cost.is_some(), CostKind::Output),
            ),
            Span::raw("  "),
        ];

        let usage_groups = wrap_usage_labels(&row.usage, layout.usage_width);
        if let Some(first_group) = usage_groups.first() {
            spans.extend(usage_group_spans(first_group));
        }
        lines.push(Line::from(spans).style(row_style));

        for group in usage_groups.iter().skip(1) {
            let mut continuation = vec![Span::raw(" ".repeat(layout.prefix_width))];
            continuation.extend(usage_group_spans(group));
            lines.push(Line::from(continuation).style(row_style));
        }
    }

    Text::from(lines)
}

fn report_row_heights(rows: &[crate::ModelRow], layout: &ReportLayout) -> Vec<usize> {
    rows.iter()
        .map(|row| {
            wrap_usage_labels(&row.usage, layout.usage_width)
                .len()
                .max(1)
        })
        .collect()
}

fn footer_lines(state: &UiState, page_label: &str) -> Vec<Line<'static>> {
    let status_line = Line::from(vec![
        Span::styled(format!("{} • ", page_label), muted_style()),
        Span::styled(
            state.status.clone(),
            status_style(state.mode, state.status.contains("failed")),
        ),
    ]);
    let legend_line = Line::from(vec![
        Span::styled("usage legend: ", muted_style()),
        Span::styled("OpenCode", usage_style(crate::UsageSource::OpenCodeDefault)),
        Span::raw(" / "),
        Span::styled(
            "OpenCode agents",
            usage_style(crate::UsageSource::OpenCodeCustom),
        ),
        Span::raw(" / "),
        Span::styled("Weave agents", usage_style(crate::UsageSource::Weave)),
        Span::raw(" / "),
        Span::styled(
            "Weave custom_agents",
            usage_style(crate::UsageSource::WeaveCustom),
        ),
        Span::raw("  •  "),
        Span::styled("sorted", muted_style()),
    ]);

    vec![status_line, legend_line]
}

fn table_header_line(layout: &ReportLayout, sort_mode: crate::SortMode) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            ljust("PROVIDER", layout.provider_width),
            table_header_style(),
        ),
        Span::raw("  "),
        Span::styled(ljust("MODEL", layout.model_width), table_header_style()),
        Span::raw("  "),
        Span::styled(rjust("IN", layout.input_width), table_header_style()),
        Span::raw("  "),
        Span::styled(rjust("OUT", layout.output_width), table_header_style()),
        Span::raw("  "),
        Span::styled(
            format!("USAGE  [{}]", sort_mode_label(sort_mode)),
            table_header_style(),
        ),
    ])
    .style(Style::default().bg(Color::Rgb(20, 24, 35)))
}

fn table_widths(rows: &[crate::ModelRow]) -> (usize, usize, usize, usize, usize) {
    let provider_width = std::iter::once("PROVIDER".len())
        .chain(rows.iter().map(|row| row.provider.len()))
        .max()
        .unwrap_or(0);
    let model_width = std::iter::once("MODEL".len())
        .chain(rows.iter().map(|row| row.model_name.len()))
        .max()
        .unwrap_or(0);
    let input_width = std::iter::once("IN".len())
        .chain(
            rows.iter()
                .map(|row| crate::format_cost(row.input_cost).len()),
        )
        .max()
        .unwrap_or(0);
    let output_width = std::iter::once("OUT".len())
        .chain(
            rows.iter()
                .map(|row| crate::format_cost(row.output_cost).len()),
        )
        .max()
        .unwrap_or(0);
    let prefix_width = provider_width + 2 + model_width + 2 + input_width + 2 + output_width + 2;

    (
        provider_width,
        model_width,
        input_width,
        output_width,
        prefix_width,
    )
}

fn wrap_usage_labels(labels: &[crate::UsageLabel], width: usize) -> Vec<Vec<crate::UsageLabel>> {
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

fn usage_group_spans(group: &[crate::UsageLabel]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (idx, label) in group.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(", ", muted_style()));
        }
        spans.push(Span::styled(label.label.clone(), usage_style(label.source)));
    }
    spans
}

fn usage_style(source: crate::UsageSource) -> Style {
    match source {
        crate::UsageSource::OpenCodeDefault => Style::default().fg(Color::Blue),
        crate::UsageSource::OpenCodeCustom => Style::default().fg(Color::LightCyan),
        crate::UsageSource::Weave => Style::default().fg(Color::Rgb(255, 140, 0)),
        crate::UsageSource::WeaveCustom => Style::default().fg(Color::Rgb(255, 179, 71)),
    }
}

fn sort_badge_style(sort_mode: crate::SortMode) -> Style {
    match sort_mode {
        crate::SortMode::ActiveFirst => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        crate::SortMode::CostAsc => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        crate::SortMode::CostDesc => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        crate::SortMode::ModelName => Style::default()
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
    let zebra = if index.is_multiple_of(2) {
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

fn sort_mode_label(mode: crate::SortMode) -> &'static str {
    match mode {
        crate::SortMode::ActiveFirst => "active-first",
        crate::SortMode::CostAsc => "cost-asc",
        crate::SortMode::CostDesc => "cost-desc",
        crate::SortMode::ModelName => "model-name",
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
    use super::{
        footer_lines, handle_key, handle_load_error, handle_loaded_rows, sort_badge_style,
        status_style, usage_style, PageState,
    };
    use crate::{ModelRow, SortMode, UiMode, UiState, UsageLabel, UsageSource};
    use ratatui::{backend::TestBackend, style::Color, Terminal};

    fn render_lines(width: u16) -> Vec<String> {
        let mut state = UiState::new();
        state.apply_snapshot(vec![ModelRow {
            model: "test/model".to_string(),
            provider: "test".to_string(),
            model_name: "model".to_string(),
            active: true,
            input_cost: Some(1.0),
            output_cost: Some(2.0),
            usage: vec![
                UsageLabel {
                    label: "usage01".to_string(),
                    source: UsageSource::OpenCodeDefault,
                },
                UsageLabel {
                    label: "usage02".to_string(),
                    source: UsageSource::OpenCodeCustom,
                },
                UsageLabel {
                    label: "usage03".to_string(),
                    source: UsageSource::Weave,
                },
                UsageLabel {
                    label: "usage04".to_string(),
                    source: UsageSource::WeaveCustom,
                },
                UsageLabel {
                    label: "usage05".to_string(),
                    source: UsageSource::OpenCodeDefault,
                },
                UsageLabel {
                    label: "usage06".to_string(),
                    source: UsageSource::OpenCodeCustom,
                },
                UsageLabel {
                    label: "usage07".to_string(),
                    source: UsageSource::Weave,
                },
                UsageLabel {
                    label: "usage08".to_string(),
                    source: UsageSource::WeaveCustom,
                },
            ],
        }]);

        let mut paging = super::PageState::new();

        let backend = TestBackend::new(width, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| super::draw(frame, &state, &mut paging))
            .expect("draw report");

        let buffer = terminal.backend().buffer();
        let area = buffer.area();
        (0..area.height)
            .map(|y| {
                let mut line = String::new();
                for x in 0..area.width {
                    line.push_str(buffer[(x, y)].symbol());
                }
                line.trim_end().to_string()
            })
            .collect()
    }

    fn model_row(model: &str) -> ModelRow {
        let (provider, model_name) = model
            .split_once('/')
            .map(|(provider, model_name)| (provider.to_string(), model_name.to_string()))
            .unwrap_or_else(|| (String::new(), model.to_string()));

        ModelRow {
            model: model.to_string(),
            provider,
            model_name,
            active: true,
            input_cost: Some(1.0),
            output_cost: Some(2.0),
            usage: vec![UsageLabel {
                label: "default".to_string(),
                source: UsageSource::OpenCodeDefault,
            }],
        }
    }

    fn render_paged_lines(
        width: u16,
        height: u16,
        rows: Vec<ModelRow>,
        page_steps: usize,
    ) -> Vec<String> {
        let mut state = UiState::new();
        state.apply_snapshot(rows);

        let mut paging = PageState::new();
        for _ in 0..page_steps {
            paging.next_page();
        }

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| super::draw(frame, &state, &mut paging))
            .expect("draw report");

        let buffer = terminal.backend().buffer();
        let area = buffer.area();
        (0..area.height)
            .map(|y| {
                let mut line = String::new();
                for x in 0..area.width {
                    line.push_str(buffer[(x, y)].symbol());
                }
                line.trim_end().to_string()
            })
            .collect()
    }

    #[test]
    fn usage_style_should_colour_sources_differently() {
        assert_eq!(
            usage_style(UsageSource::OpenCodeDefault).fg,
            Some(Color::Blue)
        );
        assert_eq!(
            usage_style(UsageSource::OpenCodeCustom).fg,
            Some(Color::LightCyan)
        );
        assert_eq!(
            usage_style(UsageSource::Weave).fg,
            Some(Color::Rgb(255, 140, 0))
        );
        assert_eq!(
            usage_style(UsageSource::WeaveCustom).fg,
            Some(Color::Rgb(255, 179, 71))
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
        let lines = footer_lines(&crate::UiState::new(), "page 1/1");
        assert_eq!(lines.len(), 2);

        let legend_text = lines[1]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(legend_text.contains("usage legend"));
        assert!(legend_text.contains("OpenCode"));
        assert!(legend_text.contains("OpenCode agents"));
        assert!(legend_text.contains("Weave agents"));
        assert!(legend_text.contains("Weave custom_agents"));
    }

    #[test]
    fn footer_lines_should_prefix_page_indicator_before_status() {
        let mut state = UiState::new();
        state.status = "Loaded model data".to_string();

        let status_text = footer_lines(&state, "page 2/3")[0]
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(status_text.starts_with("page 2/3 • "));
        assert!(status_text.contains("Loaded model data"));
    }

    #[test]
    fn header_line_should_include_paging_controls() {
        let text = super::header_line(&UiState::new())
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(text.contains("j next"));
        assert!(text.contains("k previous"));
    }

    #[test]
    fn handle_key_should_page_and_reset_on_sort_change() {
        let mut state = UiState::new();
        let mut paging = PageState::new();

        assert!(handle_key(
            crossterm::event::KeyCode::Char('j'),
            &mut state,
            &mut paging
        )
        .is_none());
        assert_eq!(paging.current_page(), 2);

        assert!(handle_key(
            crossterm::event::KeyCode::Char('k'),
            &mut state,
            &mut paging
        )
        .is_none());
        assert_eq!(paging.current_page(), 1);

        paging.next_page();
        paging.next_page();
        assert!(handle_key(
            crossterm::event::KeyCode::Char('s'),
            &mut state,
            &mut paging
        )
        .is_none());
        assert_eq!(state.sort_mode, SortMode::CostAsc);
        assert_eq!(paging.current_page(), 1);
    }

    #[test]
    fn loaded_rows_should_reset_page_and_failed_refresh_should_keep_it() {
        let mut state = UiState::new();
        let mut paging = PageState::new();
        let mut fatal = None;

        paging.next_page();
        paging.next_page();

        handle_loaded_rows(
            &mut state,
            &mut paging,
            &mut fatal,
            vec![model_row("provider/alpha")],
            false,
        );

        assert_eq!(paging.current_page(), 1);
        assert!(fatal.is_none());

        paging.next_page();
        let before = paging.current_page();
        handle_load_error(
            &mut state,
            &mut fatal,
            crate::LoadError::CurlNotFound,
            false,
        );

        assert_eq!(paging.current_page(), before);
        assert!(fatal.is_none());
    }

    #[test]
    fn draw_should_page_through_rows_and_keep_page_indicator_visible() {
        let rows = vec![
            model_row("provider/alpha"),
            model_row("provider/beta"),
            model_row("provider/gamma"),
            model_row("provider/delta"),
            model_row("provider/epsilon"),
            model_row("provider/zeta"),
        ];

        let first_page = render_paged_lines(80, 11, rows.clone(), 0);
        let second_page = render_paged_lines(80, 11, rows.clone(), 1);

        assert!(first_page.iter().any(|line| line.contains("page 1/2")));
        assert!(second_page.iter().any(|line| line.contains("page 2/2")));
        assert!(first_page.iter().any(|line| line.contains("alpha")));
        assert!(!first_page.iter().any(|line| line.contains("zeta")));
        assert!(second_page.iter().any(|line| line.contains("zeta")));
        assert!(!second_page.iter().any(|line| line.contains("alpha")));
    }

    #[test]
    fn report_view_should_wrap_usage_less_when_terminal_is_wide() {
        let wide_lines = render_lines(130);
        let narrow_lines = render_lines(80);

        let labels = [
            "usage01", "usage02", "usage03", "usage04", "usage05", "usage06", "usage07", "usage08",
        ];

        let wide_usage_lines = wide_lines
            .iter()
            .filter(|line| labels.iter().any(|label| line.contains(label)))
            .count();
        let narrow_usage_lines = narrow_lines
            .iter()
            .filter(|line| labels.iter().any(|label| line.contains(label)))
            .count();

        assert_eq!(
            wide_usage_lines, 1,
            "wide terminal should keep the usage column on one line"
        );
        assert!(
            narrow_usage_lines > 1,
            "narrow terminal should wrap the usage column"
        );
    }
}
