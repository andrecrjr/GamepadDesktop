#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WindowSwitcherStep {
    Open,
    Advance,
}

#[derive(Default)]
pub(crate) struct WindowSwitcherState {
    open: bool,
}

impl WindowSwitcherState {
    pub(crate) fn next(&mut self) -> WindowSwitcherStep {
        if self.open {
            WindowSwitcherStep::Advance
        } else {
            self.open = true;
            WindowSwitcherStep::Open
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn finish(&mut self) {
        self.open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::{WindowSwitcherState, WindowSwitcherStep};

    #[test]
    fn repeated_r1_advances_without_closing_the_window_switcher() {
        let mut switcher = WindowSwitcherState::default();

        assert_eq!(switcher.next(), WindowSwitcherStep::Open);
        assert!(switcher.is_open());
        assert_eq!(switcher.next(), WindowSwitcherStep::Advance);
        assert!(switcher.is_open());

        switcher.finish();
        assert!(!switcher.is_open());
        assert_eq!(switcher.next(), WindowSwitcherStep::Open);
    }
}
