pub(crate) mod constant;
pub(crate) mod program_output;

use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

use crate::config::Configuration;

use self::{constant::Constant, program_output::ProgramOutput};

pub(crate) type SegmentReference = usize;

#[derive(Debug)]
pub(crate) struct Segment {
    pub kind: SegmentKind,
    pub update_interval: Option<Duration>,
    pub signals: Vec<i32>,

    left_separator: String,
    right_separator: String,
    icon: String,
    hide_if_empty: bool,

    last_update: RefCell<Instant>,
    last_value: RefCell<String>,
}

#[derive(Debug)]
pub(crate) enum SegmentKind {
    ProgramOutput(ProgramOutput),
    Constant(Constant),
}

impl Segment {
    pub(crate) fn new(
        kind: SegmentKind,
        update_interval: Option<Duration>,
        signals: Vec<i32>,
        left_separator: Option<String>,
        right_separator: Option<String>,
        icon: Option<String>,
        hide_if_empty: bool,
        config: &Configuration,
    ) -> Self {
        let left_separator = left_separator
            .or(config.left_separator.clone())
            .unwrap_or("".into());
        let right_separator = right_separator
            .or(config.right_separator.clone())
            .unwrap_or("".into());
        let icon = icon.unwrap_or("".into());

        let now = Instant::now();

        let segment = Segment {
            kind,
            update_interval,
            signals,

            left_separator,
            right_separator,
            icon,
            hide_if_empty,

            last_update: RefCell::new(now),
            last_value: RefCell::new("".into()),
        };

        segment.update(&now);

        segment
    }

    pub(crate) fn get_value(&self, now: &Instant) -> String {
        if self.update_interval.is_some()
            && self.last_update.borrow().clone() + self.update_interval.unwrap() < *now
        {
            self.update(now)
        } else {
            self.last_value.borrow().clone()
        }
    }

    pub(crate) fn update(&self, now: &Instant) -> String {
        let new_value = self.compute_value();

        *self.last_update.borrow_mut() = now.clone();
        *self.last_value.borrow_mut() = new_value.clone();

        new_value
    }

    pub(crate) fn compute_value(&self) -> String {
        let new_value = self.kind.compute_value();

        if self.hide_if_empty && new_value.is_empty() {
            return "".into();
        }

        format!(
            "{}{}{}{}",
            self.left_separator, self.icon, new_value, self.right_separator
        )
    }
}

impl SegmentKind {
    fn compute_value(&self) -> String {
        match self {
            SegmentKind::ProgramOutput(p) => p.compute_value(),
            SegmentKind::Constant(c) => c.compute_value(),
        }
    }
}
