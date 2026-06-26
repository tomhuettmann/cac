use crate::app::App;
use crate::git::Contributor;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use git2::Repository;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::io;

pub fn run(app: &mut App, _repo: &Repository) -> io::Result<Vec<Contributor>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Esc => {
                    app.should_quit = true;
                    break;
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.should_quit = true;
                    break;
                }
                KeyCode::Enter => {
                    app.confirmed = true;
                    break;
                }
                KeyCode::Tab => {
                    app.toggle_selected();
                    app.search.clear();
                    app.filter();
                }
                KeyCode::Up => {
                    app.move_up();
                }
                KeyCode::Down => {
                    app.move_down();
                }
                KeyCode::Backspace => {
                    app.search.pop();
                    app.filter();
                }
                KeyCode::Char(c) => {
                    app.search.push(c);
                    app.filter();
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    if app.confirmed {
        Ok(app.get_selected_contributors())
    } else {
        Ok(vec![])
    }
}

fn draw(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with commit info
            Constraint::Length(3), // Search input
            Constraint::Min(5),   // Contributor list
            Constraint::Length(2), // Help bar
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_search(f, app, chunks[1]);
    draw_list(f, app, chunks[2]);
    draw_help(f, app, chunks[3]);
}

fn draw_header(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let short_id = &app.commit_id.to_string()[..7];
    let first_line = app.commit_msg.lines().next().unwrap_or("");
    let text = format!(" Amending: {} {}", short_id, first_line);
    let version = format!(" v{} ", env!("CARGO_PKG_VERSION"));
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" cac ")
                .title(Line::from(version).alignment(Alignment::Right)),
        )
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(paragraph, area);
}

fn draw_search(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let input = Paragraph::new(format!(" {}", app.search))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Search contributors "),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(input, area);

    f.set_cursor_position((area.x + 2 + app.search.len() as u16, area.y + 1));
}

fn draw_list(f: &mut ratatui::Frame, app: &App, area: Rect) {
    // Build items with a separator between selected and unselected
    let mut items: Vec<ListItem> = Vec::new();

    let mut in_selected_section = false;
    let mut separator_added = false;

    for (i, &contributor_idx) in app.filtered.iter().enumerate() {
        let is_selected = app.is_selected(contributor_idx);

        // Add separator when transitioning from selected to unselected
        if in_selected_section && !is_selected && !separator_added {
            items.push(ListItem::new("─".repeat(50)).style(Style::default().fg(Color::DarkGray)));
            separator_added = true;
        }

        in_selected_section = is_selected;

        let contributor = &app.contributors[contributor_idx];
        let selected = if is_selected {
            "✓ "
        } else {
            "  "
        };
        let style = if i == app.cursor {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };
        items.push(ListItem::new(format!("{}{}", selected, contributor.display())).style(style));
    }

    let title = format!(" Contributors ({}) ", app.filtered.len());
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default());

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(app.cursor));
    f.render_stateful_widget(list, area, &mut list_state);
}

fn draw_help(f: &mut ratatui::Frame, _app: &App, area: Rect) {
    let spans = vec![
        Span::styled(" ↑↓ ", Style::default().fg(Color::Yellow)),
        Span::raw("navigate  "),
        Span::styled(" Tab ", Style::default().fg(Color::Yellow)),
        Span::raw("toggle  "),
        Span::styled(" Enter ", Style::default().fg(Color::Yellow)),
        Span::raw("confirm  "),
        Span::styled(" Esc ", Style::default().fg(Color::Yellow)),
        Span::raw("cancel"),
    ];

    let help = Line::from(spans);
    let paragraph = Paragraph::new(help);
    f.render_widget(paragraph, area);
}
