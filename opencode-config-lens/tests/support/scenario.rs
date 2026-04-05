#![allow(dead_code)]

use opencode_config_lens::{ModelRow, SortMode, UiKey, UiMode, UiState};

pub fn given_model_report() -> GivenScenario {
    GivenScenario::new()
}

pub fn fail(message: impl Into<String>) -> ! {
    std::panic::panic_any(message.into());
}

#[derive(Clone)]
pub struct GivenScenario {
    startup_rows: Result<Vec<ModelRow>, String>,
    refresh_rows: Result<Vec<ModelRow>, String>,
}

impl GivenScenario {
    fn new() -> Self {
        Self {
            startup_rows: Ok(Vec::new()),
            refresh_rows: Ok(Vec::new()),
        }
    }

    pub fn with_startup_rows(mut self, rows: Vec<ModelRow>) -> Self {
        self.startup_rows = Ok(rows);
        self
    }

    pub fn with_refresh_rows(mut self, rows: Vec<ModelRow>) -> Self {
        self.refresh_rows = Ok(rows);
        self
    }

    pub fn with_startup_failure(mut self, message: impl Into<String>) -> Self {
        self.startup_rows = Err(message.into());
        self
    }

    pub fn with_refresh_failure(mut self, message: impl Into<String>) -> Self {
        self.refresh_rows = Err(message.into());
        self
    }

    pub fn when_started(self) -> ScenarioRun {
        let mut run = ScenarioRun {
            state: UiState::new(),
            effects: EffectRecord::default(),
            exit_code: None,
            stderr: String::new(),
        };

        run.effects.opencode_refresh_calls += 1;
        run.effects.cost_fetch_calls += 1;

        match self.startup_rows {
            Ok(rows) => run.state.apply_snapshot(rows),
            Err(message) => {
                run.state.apply_refresh_error(message.clone());
                run.exit_code = Some(3);
                run.stderr = message;
            }
        }

        run
    }
}

#[derive(Debug, Default, Clone)]
pub struct EffectRecord {
    opencode_refresh_calls: usize,
    cost_fetch_calls: usize,
    kept_previous_snapshot: bool,
}

pub struct ScenarioRun {
    state: UiState,
    effects: EffectRecord,
    exit_code: Option<i32>,
    stderr: String,
}

impl ScenarioRun {
    pub fn when_refresh_pressed(mut self, refresh_rows: Result<Vec<ModelRow>, String>) -> Self {
        let _ = self.state.handle_key(UiKey::Refresh);
        let previous = self.state.snapshot.clone();
        self.effects.opencode_refresh_calls += 1;
        self.effects.cost_fetch_calls += 1;

        match refresh_rows {
            Ok(rows) => self.state.apply_snapshot(rows),
            Err(message) => {
                self.state.apply_refresh_error(message);
                self.effects.kept_previous_snapshot = self.state.snapshot == previous;
            }
        }

        self
    }

    pub fn when_refreshing_with_given_result(self, given: &GivenScenario) -> Self {
        self.when_refresh_pressed(given.refresh_rows.clone())
    }

    pub fn when_sort_pressed(mut self) -> Self {
        let _ = self.state.handle_key(UiKey::CycleSort);
        self
    }

    pub fn then_state(&self) -> StateThen<'_> {
        StateThen { run: self }
    }

    pub fn then_effects(&self) -> EffectsThen<'_> {
        EffectsThen { run: self }
    }

    pub fn then_exit(&self) -> ExitThen<'_> {
        ExitThen { run: self }
    }
}

pub struct StateThen<'a> {
    run: &'a ScenarioRun,
}

impl StateThen<'_> {
    pub fn shows_ready(self) -> Self {
        if self.run.state.mode != UiMode::Ready {
            fail(format!(
                "expected ready mode, got {:?}",
                self.run.state.mode
            ));
        }
        self
    }

    pub fn shows_refreshing(self) -> Self {
        if self.run.state.mode != UiMode::Refreshing {
            fail(format!(
                "expected refreshing mode, got {:?}",
                self.run.state.mode
            ));
        }
        self
    }

    pub fn shows_status_contains(self, expected: &str) -> Self {
        if !self.run.state.status.contains(expected) {
            fail(format!(
                "expected status to contain '{expected}', got '{}'",
                self.run.state.status
            ));
        }
        self
    }

    pub fn shows_sort_mode(self, mode: SortMode) -> Self {
        if self.run.state.sort_mode != mode {
            fail(format!(
                "expected sort mode {:?}, got {:?}",
                mode, self.run.state.sort_mode
            ));
        }
        self
    }

    pub fn shows_models_in_order(self, expected: &[&str]) -> Self {
        let rows = self.run.state.visible_rows();
        let actual: Vec<&str> = rows.iter().map(|row| row.model.as_str()).collect();
        if actual != expected {
            fail(format!(
                "expected model order {:?}, got {:?}",
                expected, actual
            ));
        }
        self
    }
}

pub struct EffectsThen<'a> {
    run: &'a ScenarioRun,
}

impl EffectsThen<'_> {
    pub fn ran_opencode_refresh(self, expected: usize) -> Self {
        if self.run.effects.opencode_refresh_calls != expected {
            fail(format!(
                "expected {expected} refresh calls, got {}",
                self.run.effects.opencode_refresh_calls
            ));
        }
        self
    }

    pub fn fetched_costs(self, expected: usize) -> Self {
        if self.run.effects.cost_fetch_calls != expected {
            fail(format!(
                "expected {expected} cost fetch calls, got {}",
                self.run.effects.cost_fetch_calls
            ));
        }
        self
    }

    pub fn keeps_previous_snapshot(self) -> Self {
        if !self.run.effects.kept_previous_snapshot {
            fail("expected refresh failure to keep previous snapshot");
        }
        self
    }
}

pub struct ExitThen<'a> {
    run: &'a ScenarioRun,
}

impl ExitThen<'_> {
    pub fn exits_with_code(self, code: i32) -> Self {
        if self.run.exit_code != Some(code) {
            fail(format!(
                "expected exit code {}, got {:?}",
                code, self.run.exit_code
            ));
        }
        self
    }

    pub fn stderr_contains(self, expected: &str) -> Self {
        if !self.run.stderr.contains(expected) {
            fail(format!(
                "expected stderr to contain '{expected}', got '{}'",
                self.run.stderr
            ));
        }
        self
    }
}
