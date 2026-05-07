use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{app::AppState, notes::markdown};

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .title(note_title(state))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(note) = &state.current_note {
        let lines = markdown::render(&note.body);
        let text = Text::from(lines);
        let para = Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .scroll((state.viewer_scroll as u16, 0));
        f.render_widget(para, inner);
    } else {
        let help = Paragraph::new(
            "  Navigate with j/k\n  Enter to open a note\n  e to edit\n  / to search\n  ? for help",
        )
        .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, inner);
    }
}

fn note_title(state: &AppState) -> String {
    if let Some(note) = &state.current_note {
        let modified = if state.is_modified { " ●" } else { "" };
        format!(" {} {}", note.title(), modified)
    } else {
        " noterm ".to_string()
    }
}
