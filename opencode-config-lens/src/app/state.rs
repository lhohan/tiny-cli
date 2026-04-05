//! Application state module
//!
//! This module manages the UI state and its transitions.

use crate::report::model::{ModelRow, SortMode};

/// UI key inputs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiKey {
    Quit,
    Refresh,
    CycleSort,
}

/// UI actions resulting from key handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAction {
    None,
    Quit,
    Refresh,
}

/// UI operational modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Loading,
    Ready,
    Refreshing,
}

/// Complete UI state
#[derive(Debug, Clone)]
pub struct UiState {
    pub sort_mode: SortMode,
    pub mode: UiMode,
    pub status: String,
    pub snapshot: Vec<ModelRow>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            sort_mode: SortMode::ActiveFirst,
            mode: UiMode::Loading,
            status: "Loading model data...".to_string(),
            snapshot: Vec::new(),
        }
    }

    pub fn handle_key(&mut self, key: UiKey) -> UiAction {
        match key {
            UiKey::Quit => UiAction::Quit,
            UiKey::Refresh => {
                self.set_refreshing();
                UiAction::Refresh
            }
            UiKey::CycleSort => {
                self.sort_mode = match self.sort_mode {
                    SortMode::ActiveFirst => SortMode::CostAsc,
                    SortMode::CostAsc => SortMode::CostDesc,
                    SortMode::CostDesc => SortMode::ModelName,
                    SortMode::ModelName => SortMode::ActiveFirst,
                };
                UiAction::None
            }
        }
    }

    pub fn apply_snapshot(&mut self, rows: Vec<ModelRow>) {
        self.snapshot = rows;
        self.mode = UiMode::Ready;
        self.status = "Loaded model data".to_string();
    }

    pub fn apply_refresh_error(&mut self, message: String) {
        self.mode = UiMode::Ready;
        self.status = message;
    }

    pub fn set_refreshing(&mut self) {
        self.mode = UiMode::Refreshing;
        self.status = "Refreshing model data...".to_string();
    }

    pub fn visible_rows(&self) -> Vec<ModelRow> {
        let mut rows = self.snapshot.clone();
        rows.sort_by(|a, b| crate::report::sort::compare_rows(a, b, self.sort_mode));
        rows
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_state_should_start_in_loading_mode() {
        let state = UiState::new();
        assert!(matches!(state.mode, UiMode::Loading));
        assert_eq!(state.status, "Loading model data...");
    }

    #[test]
    fn ui_state_should_quit_on_q_key() {
        let mut state = UiState::new();
        let action = state.handle_key(UiKey::Quit);
        assert!(matches!(action, UiAction::Quit));
    }

    #[test]
    fn ui_state_should_refresh_on_r_key() {
        let mut state = UiState::new();
        let action = state.handle_key(UiKey::Refresh);
        assert!(matches!(action, UiAction::Refresh));
        assert!(matches!(state.mode, UiMode::Refreshing));
    }

    #[test]
    fn ui_state_should_cycle_sort_modes() {
        let mut state = UiState::new();
        assert!(matches!(state.sort_mode, SortMode::ActiveFirst));

        state.handle_key(UiKey::CycleSort);
        assert!(matches!(state.sort_mode, SortMode::CostAsc));

        state.handle_key(UiKey::CycleSort);
        assert!(matches!(state.sort_mode, SortMode::CostDesc));

        state.handle_key(UiKey::CycleSort);
        assert!(matches!(state.sort_mode, SortMode::ModelName));

        state.handle_key(UiKey::CycleSort);
        assert!(matches!(state.sort_mode, SortMode::ActiveFirst));
    }

    #[test]
    fn ui_state_should_apply_snapshot() {
        let mut state = UiState::new();
        state.apply_snapshot(vec![ModelRow {
            model: "test".to_string(),
            provider: "test".to_string(),
            model_name: "test".to_string(),
            active: true,
            input_cost: None,
            output_cost: None,
            usage: vec![],
        }]);

        assert!(matches!(state.mode, UiMode::Ready));
        assert_eq!(state.snapshot.len(), 1);
    }

    #[test]
    fn ui_state_should_preserve_snapshot_on_refresh_error() {
        let mut state = UiState::new();
        state.apply_snapshot(vec![ModelRow {
            model: "test".to_string(),
            provider: "test".to_string(),
            model_name: "test".to_string(),
            active: true,
            input_cost: None,
            output_cost: None,
            usage: vec![],
        }]);

        state.set_refreshing();
        state.apply_refresh_error("network error".to_string());

        assert!(matches!(state.mode, UiMode::Ready));
        assert_eq!(state.snapshot.len(), 1);
        assert!(state.status.contains("error"));
    }
}
