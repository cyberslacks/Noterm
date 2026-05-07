use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::AppState;
use super::search_overlay::centered_rect;

pub fn render(f: &mut Frame, area: ratatui::layout::Rect, _state: &AppState) {
    let popup = centered_rect(60, 70, area);
    f.render_widget(ratatui::widgets::Clear, popup);

    let block = Block::default()
        .title(" Help (? or Esc to close) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::White));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let bindings: &[(&str, &str)] = &[
        ("Navigation", ""),
        ("j / ↓", "Move down in file tree"),
        ("k / ↑", "Move up in file tree"),
        ("Enter", "Open file / expand directory"),
        ("", ""),
        ("Modes", ""),
        ("e", "Edit current note"),
        ("n", "New note"),
        ("d", "Delete selected note (with confirmation)"),
        ("/", "Full-text search"),
        ("v", "Vector / semantic search"),
        ("c", "Toggle LLM chat panel"),
        ("K", "Kanban board view"),
        ("G", "Git panel"),
        ("?", "This help screen"),
        ("Esc", "Return to normal / close overlay"),
        ("", ""),
        ("Editor", ""),
        ("Ctrl+s", "Save note"),
        ("Esc", "Save and exit editor"),
        ("", ""),
        ("Chat", ""),
        ("Enter", "Send message"),
        ("Ctrl+l", "Clear chat history"),
        ("", ""),
        ("Kanban", ""),
        ("h / l", "Move between columns"),
        ("m", "Move card to next column"),
        ("j / k", "Navigate cards"),
        ("", ""),
        ("Git", ""),
        ("s", "Stage all changes"),
        ("c", "Commit (enter message)"),
        ("p", "Push to remote"),
        ("P", "Pull from remote"),
        ("Tab", "Switch Status / Log tabs"),
        ("", ""),
        ("I", "Meetily import panel"),
        ("S", "Settings (LLM provider, models, API keys)"),
        ("q / Ctrl+q", "Quit noterm"),
    ];

    let items: Vec<ListItem> = bindings
        .iter()
        .map(|(key, desc)| {
            if desc.is_empty() && !key.is_empty() {
                // Section header
                ListItem::new(Line::from(Span::styled(
                    key.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )))
            } else if key.is_empty() {
                ListItem::new("")
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:<12}", key),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(desc.to_string()),
                ]))
            }
        })
        .collect();

    f.render_widget(List::new(items), inner);
}
