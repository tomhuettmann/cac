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
    pub total_scanned: usize,
    pub page_size: usize,
    pub all_scanned: bool,
}

impl App {
    pub fn new(
        contributors: Vec<Contributor>,
        commit_msg: String,
        commit_id: Oid,
        page_size: usize,
        all_scanned: bool,
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
            total_scanned: page_size,
            page_size,
            all_scanned,
        }
    }

    pub fn load_more(&mut self, new_contributors: Vec<Contributor>, all_scanned: bool) {
        let start_idx = self.contributors.len();
        self.contributors.extend(new_contributors);
        self.all_scanned = all_scanned;
        self.total_scanned += self.page_size;

        // Re-filter to include new contributors
        if self.search.is_empty() {
            self.filtered = (0..self.contributors.len()).collect();
        } else {
            // Add newly matching contributors to filtered list
            use fuzzy_matcher::FuzzyMatcher;
            use fuzzy_matcher::skim::SkimMatcherV2;
            let matcher = SkimMatcherV2::default();

            for i in start_idx..self.contributors.len() {
                let haystack = self.contributors[i].display();
                if matcher.fuzzy_match(&haystack, &self.search).is_some() {
                    self.filtered.push(i);
                }
            }
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
