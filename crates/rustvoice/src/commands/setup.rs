use std::{collections::HashMap, io};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use crate::cli::SetupAction;

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn run(action: Option<SetupAction>) -> Result<(), Error> {
    match action {
        Some(SetupAction::Db) => {
            let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./db.sqlite".into());
            return prompt_db_init(&url).await;
        }
        None => {}
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_tui(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result?;

    if app.saved {
        println!("Configuration saved to .env");
        let url = app
            .fields
            .iter()
            .find(|f| f.key == "DATABASE_URL")
            .map(|f| f.value.clone())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "sqlite:./db.sqlite".to_owned());
        prompt_db_init(&url).await?;
    }
    Ok(())
}

// ─── App ───────────────────────────────────────────────────────────────────

struct Field {
    key: &'static str,
    label: &'static str,
    value: String,
    snapshot: String, // restored on Esc in edit mode
    masked: bool,
    /// If true and value is empty, the key is omitted from .env entirely.
    optional: bool,
}

enum Mode {
    Browse,
    Edit { cursor: usize },
}

struct App {
    fields: Vec<Field>,
    selected: usize,
    mode: Mode,
    saved: bool,
}

impl App {
    fn new() -> Self {
        let env = env_read(".env");
        let mk = |key: &'static str,
                  label: &'static str,
                  default: &str,
                  masked: bool,
                  optional: bool| {
            let value = env.get(key).cloned().unwrap_or_else(|| default.to_owned());
            Field {
                key,
                label,
                snapshot: value.clone(),
                value,
                masked,
                optional,
            }
        };
        Self {
            fields: vec![
                mk("DISCORD_TOKEN", "Discord Token", "", true, false),
                mk("DISCORD_SERVER_ID", "Discord Server ID", "", false, true),
                mk(
                    "DATABASE_URL",
                    "Database URL",
                    "sqlite:./db.sqlite",
                    false,
                    false,
                ),
                mk("IPC_SOCKET_PATH", "IPC Socket Path", "", false, true),
            ],
            selected: 0,
            mode: Mode::Browse,
            saved: false,
        }
    }

    fn cursor(&self) -> Option<usize> {
        match &self.mode {
            Mode::Edit { cursor } => Some(*cursor),
            Mode::Browse => None,
        }
    }

    fn enter_edit(&mut self) {
        let cur = self.fields[self.selected].value.len();
        self.fields[self.selected].snapshot = self.fields[self.selected].value.clone();
        self.mode = Mode::Edit { cursor: cur };
    }

    fn confirm_edit(&mut self) {
        self.fields[self.selected].snapshot = self.fields[self.selected].value.clone();
        self.mode = Mode::Browse;
    }

    fn cancel_edit(&mut self) {
        let snap = self.fields[self.selected].snapshot.clone();
        self.fields[self.selected].value = snap;
        self.mode = Mode::Browse;
    }

    fn save(&mut self) {
        env_write(".env", &self.fields);
        self.saved = true;
    }
}

// ─── TUI loop ──────────────────────────────────────────────────────────────

fn run_tui<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };

        // Snapshot before any mutable borrow of `app`.
        let cur = app.cursor();

        if let Some(cur) = cur {
            // ── Edit mode ──────────────────────────────────────────────
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => app.cancel_edit(),

                (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                    app.confirm_edit();
                    app.save();
                    return Ok(());
                }

                // Confirm field and move to the next one (or leave edit mode)
                (KeyCode::Enter, _) | (KeyCode::Tab, _) => {
                    app.confirm_edit();
                    if app.selected + 1 < app.fields.len() {
                        app.selected += 1;
                        app.enter_edit();
                    }
                }

                // Cursor movement
                (KeyCode::Left, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                    if cur > 0 {
                        let prev = prev_char_boundary(&app.fields[app.selected].value, cur);
                        app.mode = Mode::Edit { cursor: prev };
                    }
                }
                (KeyCode::Right, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                    let len = app.fields[app.selected].value.len();
                    if cur < len {
                        let next = next_char_boundary(&app.fields[app.selected].value, cur);
                        app.mode = Mode::Edit { cursor: next };
                    }
                }
                (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                    app.mode = Mode::Edit { cursor: 0 };
                }
                (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                    let len = app.fields[app.selected].value.len();
                    app.mode = Mode::Edit { cursor: len };
                }

                // Deletion
                (KeyCode::Backspace, _) => {
                    if cur > 0 {
                        let prev = prev_char_boundary(&app.fields[app.selected].value, cur);
                        app.fields[app.selected].value.remove(prev);
                        app.mode = Mode::Edit { cursor: prev };
                    }
                }
                (KeyCode::Delete, _) | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                    let len = app.fields[app.selected].value.len();
                    if cur < len {
                        app.fields[app.selected].value.remove(cur);
                    }
                }
                (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                    app.fields[app.selected].value.truncate(cur);
                }
                (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                    let tail = app.fields[app.selected].value[cur..].to_owned();
                    app.fields[app.selected].value = tail;
                    app.mode = Mode::Edit { cursor: 0 };
                }
                (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                    let head = &app.fields[app.selected].value[..cur];
                    let word_start = head.rfind(' ').map(|i| i + 1).unwrap_or(0);
                    let tail = app.fields[app.selected].value[cur..].to_owned();
                    app.fields[app.selected].value =
                        format!("{}{}", &app.fields[app.selected].value[..word_start], tail);
                    app.mode = Mode::Edit { cursor: word_start };
                }

                // Printable character (no Ctrl/Alt)
                (KeyCode::Char(c), m)
                    if !m.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    app.fields[app.selected].value.insert(cur, c);
                    app.mode = Mode::Edit {
                        cursor: cur + c.len_utf8(),
                    };
                }

                _ => {}
            }
        } else {
            // ── Browse mode ─────────────────────────────────────────────
            match (key.code, key.modifiers) {
                (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                    if app.selected > 0 {
                        app.selected -= 1;
                    }
                }
                (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                    if app.selected + 1 < app.fields.len() {
                        app.selected += 1;
                    }
                }
                (KeyCode::Enter, _) => app.enter_edit(),
                (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                    app.save();
                    return Ok(());
                }
                (KeyCode::Esc, _) | (KeyCode::Char('q'), KeyModifiers::NONE) => {
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}

// ─── Drawing ───────────────────────────────────────────────────────────────

// Prefix width (2) + label width (20) = 42 total label column
const LABEL_COL: u16 = 22;

fn draw(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let n = app.fields.len() as u16;
    // panel height: borders(2) + fields(n) + blank separator(1) + help(1) + margins(2)
    let panel_h = n + 6;
    let panel_w = 72u16.min(area.width.saturating_sub(4));

    // Centre the panel vertically and horizontally
    let [_, row, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(panel_h),
        Constraint::Fill(1),
    ])
    .areas(area);

    let [_, panel, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(panel_w),
        Constraint::Fill(1),
    ])
    .areas(row);

    f.render_widget(
        Block::default()
            .title(" RustVoice Setup ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
        panel,
    );

    // Inside the border, with 1-char horizontal padding
    let inner = panel.inner(Margin {
        vertical: 1,
        horizontal: 2,
    });
    let value_w = inner.width.saturating_sub(LABEL_COL);

    let editing_cursor = app.cursor();

    for (i, field) in app.fields.iter().enumerate() {
        let is_sel = i == app.selected;
        let is_edit = is_sel && editing_cursor.is_some();

        let y = inner.y + i as u16;
        let row_rect = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };

        // Raw string shown in the value column
        let raw: String = if field.masked && !field.value.is_empty() && !is_edit {
            "*".repeat(field.value.len().min(value_w as usize))
        } else {
            field.value.clone()
        };

        // Scroll the view so the cursor stays visible when editing
        let (visible, cursor_col): (String, Option<u16>) = if is_edit {
            let cur = editing_cursor.unwrap();
            let w = value_w as usize;
            // Keep cursor at most at the last visible column
            let scroll = cur.saturating_sub(w.saturating_sub(1));
            let visible: String = raw.chars().skip(scroll).take(w).collect();
            (
                format!("{:<w$}", visible, w = w),
                Some((cur - scroll) as u16),
            )
        } else {
            let visible: String = raw.chars().take(value_w as usize).collect();
            (format!("{:<w$}", visible, w = value_w as usize), None)
        };

        let prefix_style = if is_sel {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let value_style = if is_edit {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else if is_sel {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        let prefix = if is_sel { "❯ " } else { "  " };
        let line = Line::from(vec![
            Span::styled(format!("{prefix}{:<20}", field.label), prefix_style),
            Span::styled(visible, value_style),
        ]);

        f.render_widget(Paragraph::new(line), row_rect);

        if let Some(col) = cursor_col {
            // col is the cursor offset within the value column
            let cx = row_rect.x + LABEL_COL + col;
            if cx < row_rect.x + row_rect.width {
                f.set_cursor_position((cx, y));
            }
        }
    }

    // Help line at the bottom of the inner area
    let help_y = inner.y + inner.height.saturating_sub(1);
    let help_rect = Rect {
        x: inner.x,
        y: help_y,
        width: inner.width,
        height: 1,
    };
    let help = if editing_cursor.is_some() {
        "Esc cancel  Enter/Tab next field  Ctrl+S save"
    } else {
        "↑↓ / Ctrl+P/N navigate  Enter edit  Ctrl+S save  q quit"
    };
    f.render_widget(
        Paragraph::new(help)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        help_rect,
    );
}

// ─── Char-boundary helpers ─────────────────────────────────────────────────

fn prev_char_boundary(s: &str, pos: usize) -> usize {
    s[..pos].char_indices().last().map(|(i, _)| i).unwrap_or(0)
}

fn next_char_boundary(s: &str, pos: usize) -> usize {
    s[pos..]
        .char_indices()
        .nth(1)
        .map(|(i, _)| pos + i)
        .unwrap_or(s.len())
}

// ─── .env I/O ──────────────────────────────────────────────────────────────

fn env_read(path: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        return map;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim();
            let v = if let Some(rest) = v.strip_prefix('"') {
                rest.split('"').next().unwrap_or("")
            } else if let Some(rest) = v.strip_prefix('\'') {
                rest.split('\'').next().unwrap_or("")
            } else {
                v.split('#').next().map(str::trim).unwrap_or(v)
            };
            map.insert(k.trim().to_owned(), v.to_owned());
        }
    }
    map
}

fn env_write(path: &str, fields: &[Field]) {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let managed: std::collections::HashSet<&str> = fields.iter().map(|f| f.key).collect();

    // Preserve lines that don't belong to managed keys (comments, blank lines, other vars)
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|line| {
            if let Some((k, _)) = line.trim().split_once('=') {
                !managed.contains(k.trim())
            } else {
                true
            }
        })
        .map(str::to_owned)
        .collect();

    for f in fields {
        if f.optional && f.value.is_empty() {
            continue;
        }
        lines.push(format!("{}=\"{}\"", f.key, f.value));
    }

    let _ = std::fs::write(path, lines.join("\n") + "\n");
}

// ─── Database init ─────────────────────────────────────────────────────────

async fn prompt_db_init(url: &str) -> Result<(), Error> {
    match db::management::needs_migration(url).await {
        Ok(None) => {
            println!("Database is up to date.");
        }
        Ok(Some(n)) => {
            let question = format!("{n} pending migration(s). Apply now?");
            if prompt_yes_no(&question, true)? {
                db::connection::connect(url).await?;
                println!("Database ready.");
            }
        }
        Err(e) => {
            eprintln!("Warning: could not check database: {e}");
            if prompt_yes_no("Initialize database now?", true)? {
                db::connection::connect(url).await?;
                println!("Database ready.");
            }
        }
    }
    Ok(())
}

fn prompt_yes_no(question: &str, default_yes: bool) -> io::Result<bool> {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("{question} {hint} ");
    use io::Write as _;
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(match input.trim().to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default_yes,
    })
}
