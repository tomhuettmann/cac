use crate::git::Contributor;
use git2::Oid;

pub struct App {
    pub contributors: Vec<Contributor>,
    pub filtered: Vec<usize>,
    pub selected: Vec<usize>,
    pub cursor: usize,
    pub search: String,
    pub commit_msg: String,
    pub commit_id: Oid,
    pub should_quit: bool,
    pub confirmed: bool,
}

impl App {
    pub fn new(
        contributors: Vec<Contributor>,
        commit_msg: String,
        commit_id: Oid,
    ) -> Self {
        let filtered: Vec<usize> = (0..contributors.len()).collect();
        let mut app = Self {
            contributors,
            filtered,
            selected: Vec::new(),
            cursor: 0,
            search: String::new(),
            commit_msg,
            commit_id,
            should_quit: false,
            confirmed: false,
        };
        app.sort_filtered();
        app
    }

    /// Reorders `filtered` to pin selected contributors at the top,
    /// in the order they were selected, followed by unselected contributors.
    fn sort_filtered(&mut self) {
        let mut selected_indices = Vec::new();
        let mut unselected_indices = Vec::new();

        for &idx in &self.filtered {
            if self.is_selected(idx) {
                selected_indices.push(idx);
            } else {
                unselected_indices.push(idx);
            }
        }

        // Sort selected_indices to match their order in self.selected
        selected_indices.sort_by_key(|&idx| {
            self.selected.iter().position(|&s| s == idx).unwrap_or(usize::MAX)
        });

        // Pin selected to top, followed by unselected
        self.filtered = [selected_indices, unselected_indices].concat();
    }

    pub fn filter(&mut self) {
        use fuzzy_matcher::FuzzyMatcher;
        use fuzzy_matcher::skim::SkimMatcherV2;

        let matcher = SkimMatcherV2::default();

        if self.search.is_empty() {
            self.filtered = (0..self.contributors.len()).collect();
        } else {
            let mut scored: Vec<(usize, i64)> = self
                .contributors
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    let haystack = c.display();
                    matcher.fuzzy_match(&haystack, &self.search).map(|score| (i, score))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered = scored.into_iter().map(|(i, _)| i).collect();
        }

        if self.filtered.is_empty() {
            self.cursor = 0;
        } else if self.cursor >= self.filtered.len() {
            self.cursor = self.filtered.len() - 1;
        }

        self.sort_filtered();
    }

    pub fn toggle_selected(&mut self) {
        if let Some(&idx) = self.filtered.get(self.cursor) {
            if let Some(pos) = self.selected.iter().position(|&s| s == idx) {
                self.selected.remove(pos);
            } else {
                self.selected.push(idx);
            }

            // Re-sort filtered to pin selected at top
            self.sort_filtered();

            // Make cursor follow the toggled item to its new position
            if let Some(new_pos) = self.filtered.iter().position(|&i| i == idx) {
                self.cursor = new_pos;
            }
        }
    }

    pub fn is_selected(&self, contributor_idx: usize) -> bool {
        self.selected.contains(&contributor_idx)
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.filtered.len() {
            self.cursor += 1;
        }
    }

    pub fn get_selected_contributors(&self) -> Vec<Contributor> {
        self.selected
            .iter()
            .map(|&i| self.contributors[i].clone())
            .collect()
    }
}
