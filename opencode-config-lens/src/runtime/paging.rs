use std::ops::Range;

#[derive(Debug, Clone)]
pub struct PageState {
    page_index: usize,
    viewport_height: usize,
}

impl PageState {
    pub fn new() -> Self {
        Self {
            page_index: 0,
            viewport_height: 0,
        }
    }

    #[cfg(test)]
    pub fn with_viewport_height(viewport_height: usize) -> Self {
        Self {
            page_index: 0,
            viewport_height,
        }
    }

    pub fn set_viewport_height(&mut self, viewport_height: usize) {
        self.viewport_height = viewport_height;
    }

    pub fn current_page(&self) -> usize {
        self.page_index + 1
    }

    pub fn next_page(&mut self) {
        self.page_index = self.page_index.saturating_add(1);
    }

    pub fn previous_page(&mut self) {
        self.page_index = self.page_index.saturating_sub(1);
    }

    pub fn reset(&mut self) {
        self.page_index = 0;
    }

    pub fn clamp_to_rows(&mut self, row_heights: &[usize]) {
        let total_pages = self.page_count(row_heights);
        self.page_index = self.page_index.min(total_pages.saturating_sub(1));
    }

    pub fn page_count(&self, row_heights: &[usize]) -> usize {
        self.page_ranges(row_heights).len()
    }

    pub fn page_label(&self, row_heights: &[usize]) -> String {
        let total_pages = self.page_count(row_heights).max(1);
        let current_page = self.current_page().min(total_pages);
        format!("page {}/{}", current_page, total_pages)
    }

    pub fn page_range(&self, row_heights: &[usize]) -> Range<usize> {
        let ranges = self.page_ranges(row_heights);
        let page_index = self.page_index.min(ranges.len().saturating_sub(1));
        ranges[page_index].clone()
    }

    pub fn page_ranges(&self, row_heights: &[usize]) -> Vec<Range<usize>> {
        if row_heights.is_empty() {
            return vec![0..0];
        }

        let content_height = self.viewport_height.saturating_sub(1).max(1);
        let mut ranges = Vec::new();
        let mut start = 0usize;
        let mut used = 0usize;

        for (idx, &height) in row_heights.iter().enumerate() {
            let height = height.max(1);

            if used > 0 && used + height > content_height {
                ranges.push(start..idx);
                start = idx;
                used = 0;
            }

            used += height;
        }

        ranges.push(start..row_heights.len());
        ranges
    }
}

impl Default for PageState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::PageState;

    #[test]
    fn page_state_should_move_forward_and_back() {
        let mut state = PageState::new();

        assert_eq!(state.current_page(), 1);

        state.next_page();
        assert_eq!(state.current_page(), 2);

        state.previous_page();
        assert_eq!(state.current_page(), 1);

        state.previous_page();
        assert_eq!(state.current_page(), 1);
    }

    #[test]
    fn page_state_should_count_pages_row_at_a_time() {
        let state = PageState::with_viewport_height(5);

        assert_eq!(state.page_count(&[]), 1);
        assert_eq!(state.page_count(&[4]), 1);
        assert_eq!(state.page_count(&[5]), 1);
        assert_eq!(state.page_count(&[3, 2]), 2);
        assert_eq!(state.page_count(&[3, 1]), 1);
    }

    #[test]
    fn page_state_should_keep_wrapped_rows_together() {
        let state = PageState::with_viewport_height(5);

        assert_eq!(state.page_ranges(&[3, 2]), vec![0..1, 1..2]);
        assert_eq!(state.page_ranges(&[4, 1]), vec![0..1, 1..2]);
    }

    #[test]
    fn page_state_should_keep_oversized_rows_intact() {
        let state = PageState::with_viewport_height(3);

        assert_eq!(state.page_ranges(&[3]), vec![0..1]);
        assert_eq!(state.page_ranges(&[5, 1]), vec![0..1, 1..2]);
    }

    #[test]
    fn page_state_should_clamp_after_resize() {
        let mut state = PageState::with_viewport_height(5);

        state.next_page();
        state.next_page();
        assert_eq!(state.current_page(), 3);

        state.set_viewport_height(100);
        state.clamp_to_rows(&[1, 1]);

        assert_eq!(state.current_page(), 1);
        assert_eq!(state.page_label(&[1, 1]), "page 1/1");
    }

    #[test]
    fn empty_report_should_show_page_one_of_one() {
        let state = PageState::new();

        assert_eq!(state.page_count(&[]), 1);
        assert_eq!(state.page_label(&[]), "page 1/1");
    }
}
