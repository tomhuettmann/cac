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
        Self {
            contributors,
            filtered,
            selected: Vec::new(),
            cursor: 0,
            search: String::new(),
            commit_msg,
            commit_id,
            should_quit: false,
            confirmed: false,
        }
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
    }

    pub fn toggle_selected(&mut self) {
        if let Some(&idx) = self.filtered.get(self.cursor) {
            if let Some(pos) = self.selected.iter().position(|&s| s == idx) {
                self.selected.remove(pos);
            } else {
                self.selected.push(idx);
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
