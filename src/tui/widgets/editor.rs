use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders},
    Frame,
};

use crate::app::AppState;

pub fn render(f: &mut Frame, area: Rect, state: &mut AppState) {
    let modified = if state.is_modified { " ●" } else { "" };
    let title = state
        .current_note
        .as_ref()
        .map(|n| format!(" {} {} [EDIT]", n.title(), modified))
        .unwrap_or_else(|| " [EDIT] ".to_string());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));

    state.editor.set_block(block);
    state
        .editor
        .set_cursor_line_style(Style::default().bg(Color::DarkGray));

    f.render_widget(state.editor.widget(), area);
}
