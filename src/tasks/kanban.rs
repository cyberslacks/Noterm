use super::{Task, TaskStatus};

#[derive(Debug, Clone, Default)]
pub struct KanbanState {
    pub columns: Vec<KanbanColumn>,
    pub focused_col: usize,
    pub focused_card: usize,
}

#[derive(Debug, Clone)]
pub struct KanbanColumn {
    pub status: TaskStatus,
    pub cards: Vec<Task>,
    pub scroll: usize,
}

impl KanbanState {
    pub fn from_tasks(tasks: Vec<Task>) -> Self {
        let mut columns: Vec<KanbanColumn> = TaskStatus::all()
            .iter()
            .map(|s| KanbanColumn {
                status: s.clone(),
                cards: Vec::new(),
                scroll: 0,
            })
            .collect();

        for task in tasks {
            if let Some(col) = columns.iter_mut().find(|c| c.status == task.status) {
                col.cards.push(task);
            }
        }

        KanbanState {
            columns,
            focused_col: 0,
            focused_card: 0,
        }
    }

    pub fn focused_column(&self) -> Option<&KanbanColumn> {
        self.columns.get(self.focused_col)
    }

    pub fn focused_task(&self) -> Option<&Task> {
        self.columns
            .get(self.focused_col)
            .and_then(|c| c.cards.get(self.focused_card))
    }

    pub fn move_focused_right(&mut self) {
        if self.focused_col + 1 < self.columns.len() {
            self.move_card_to_column(self.focused_col, self.focused_col + 1);
        }
    }

    pub fn move_focused_left(&mut self) {
        if self.focused_col > 0 {
            let dest = self.focused_col - 1;
            self.move_card_to_column(self.focused_col, dest);
        }
    }

    fn move_card_to_column(&mut self, from: usize, to: usize) {
        let card_idx = self.focused_card;
        if from >= self.columns.len() || to >= self.columns.len() {
            return;
        }
        if card_idx >= self.columns[from].cards.len() {
            return;
        }
        let mut card = self.columns[from].cards.remove(card_idx);
        card.status = self.columns[to].status.clone();
        self.columns[to].cards.push(card);
        self.focused_col = to;
        self.focused_card = self.columns[to].cards.len() - 1;
    }

    pub fn nav_down(&mut self) {
        if let Some(col) = self.columns.get(self.focused_col) {
            if self.focused_card + 1 < col.cards.len() {
                self.focused_card += 1;
            }
        }
    }

    pub fn nav_up(&mut self) {
        if self.focused_card > 0 {
            self.focused_card -= 1;
        }
    }

    pub fn nav_col_right(&mut self) {
        if self.focused_col + 1 < self.columns.len() {
            self.focused_col += 1;
            self.focused_card = 0;
        }
    }

    pub fn nav_col_left(&mut self) {
        if self.focused_col > 0 {
            self.focused_col -= 1;
            self.focused_card = 0;
        }
    }
}
