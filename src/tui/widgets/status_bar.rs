use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::{AppState, Mode, StatusLevel};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    // Mode indicator
    let (mode_str, mode_color) = match state.mode {
        Mode::Normal => ("NORMAL", Color::Green),
        Mode::Edit => ("INSERT", Color::Yellow),
        Mode::Search => ("SEARCH", Color::Cyan),
        Mode::VectorSearch => ("VSEARCH", Color::Magenta),
        Mode::Chat => ("CHAT", Color::Blue),
        Mode::Kanban => ("KANBAN", Color::LightGreen),
        Mode::Git => ("GIT", Color::LightRed),
        Mode::Help => ("HELP", Color::White),
        Mode::NewNote => ("NEW NOTE", Color::Yellow),
        Mode::GitCommitInput => ("COMMIT", Color::Yellow),
        Mode::ConfirmDelete => ("DELETE?", Color::Red),
        Mode::MeetilyImport => ("MEETILY", Color::Cyan),
        Mode::Settings => ("SETTINGS", Color::Cyan),
        Mode::Summarize => ("SUMMARIZE", Color::LightMagenta),
    };

    let left = Line::from(vec![
        Span::styled(
            format!(" {mode_str} "),
            Style::default()
                .fg(Color::Black)
                .bg(mode_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            state
                .current_note
                .as_ref()
                .map(|n| n.relative_path.clone())
                .unwrap_or_else(|| state.notes_dir.to_string_lossy().to_string()),
            Style::default().fg(Color::White),
        ),
    ]);

    let right_text = if let Some((msg, level)) = &state.status_message {
        let color = match level {
            StatusLevel::Info => Color::White,
            StatusLevel::Success => Color::Green,
            StatusLevel::Warning => Color::Yellow,
            StatusLevel::Error => Color::Red,
        };
        Line::from(Span::styled(format!("{msg} "), Style::default().fg(color)))
    } else {
        let branch = state.git_branch();
        Line::from(Span::styled(
            format!(" git:{branch} "),
            Style::default().fg(Color::DarkGray),
        ))
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(30)])
        .split(area);

    f.render_widget(Paragraph::new(left).style(Style::default().bg(Color::Black)), chunks[0]);
    f.render_widget(
        Paragraph::new(right_text).style(Style::default().bg(Color::Black)),
        chunks[1],
    );
}
