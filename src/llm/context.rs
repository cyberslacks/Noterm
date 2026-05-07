use crate::{config::LlmConfig, notes::Note};

pub fn build_system_prompt(config: &LlmConfig, context_notes: &[Note]) -> String {
    let mut prompt = config.system_prompt.clone();

    if !context_notes.is_empty() {
        prompt.push_str("\n\n## Relevant Notes\n");
        for note in context_notes.iter().take(config.max_context_notes) {
            let excerpt = if note.body.len() > 2000 {
                &note.body[..2000]
            } else {
                &note.body
            };
            prompt.push_str(&format!(
                "\n### {}\n```markdown\n{}\n```\n",
                note.relative_path, excerpt
            ));
        }
    }

    prompt
}
