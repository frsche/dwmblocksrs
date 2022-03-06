pub mod constant;
pub mod program_output;

use std::{cell::RefCell, sync::mpsc::Sender, thread, time::Duration};

use crate::config::Configuration;

use self::{constant::Constant, program_output::ProgramOutput};

pub(crate) type SegmentId = usize;

#[derive(Debug)]
pub(crate) struct Segment {
    kind: SegmentKind,
    update_interval: Option<Duration>,
    pub signals: Vec<i32>,
    segment_id: SegmentId,

    left_separator: String,
    right_separator: String,
    icon: String,
    hide_if_empty: bool,

    current_value: RefCell<String>,
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
        segment_id: SegmentId,
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

        let s = Segment {
            kind,
            update_interval,
            signals,
            segment_id,

            left_separator,
            right_separator,
            icon,
            hide_if_empty,

            current_value: RefCell::new("".into()),
        };

        s.update();
        s
    }

    pub(crate) fn run_update_loop(&self, channel: Sender<SegmentId>) {
        if let Some(interval) = self.update_interval {
            let segment_id = self.segment_id;
            thread::spawn(move || loop {
                thread::sleep(interval);
                channel.send(segment_id).unwrap();
            });
        }
    }

    pub(crate) fn update(&self) {
        *self.current_value.borrow_mut() = self.compute_value();
    }

    pub(crate) fn current_value(&self) -> String {
        self.current_value.borrow().clone()
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    macro_rules! test_segment_kinds {
        ( $( $name:ident: $segment:expr => $expect:expr, )+ ) => {
            mod segment_kinds {
                use super::*;
                $(
                    #[test]
                    fn $name() {
                    let kind : SegmentKind = $segment.into();
                    assert_eq!(&kind.compute_value(), $expect);
                    }
                )+
            }
        }
    }

    test_segment_kinds!(
        constant: Constant::new("constant".into()) => "constant",
        program: ProgramOutput::new("echo".into(),vec!["hello".into()]) => "hello",
    );

    fn default_segment() -> Segment {
        Segment::new(
            Constant::new("test".into()).into(),
            None,
            vec![],
            0,
            None,
            None,
            None,
            false,
            &Configuration {
                left_separator: None,
                right_separator: None,
                script_dir: PathBuf::default(),
                update_all_signal: None,
            },
        )
    }

    mod segment {
        use super::*;

        #[test]
        fn consant() {
            let s = default_segment();
            assert_eq!(&s.compute_value(), "test");
        }

        #[test]
        fn left_separator() {
            let mut s = default_segment();
            s.left_separator = ">".into();
            assert_eq!(&s.compute_value(), ">test");
        }

        #[test]
        fn right_separator() {
            let mut s = default_segment();
            s.right_separator = "<".into();
            assert_eq!(&s.compute_value(), "test<");
        }

        #[test]
        fn icon() {
            let mut s = default_segment();
            s.icon = "$".into();
            assert_eq!(&s.compute_value(), "$test");
        }

        #[test]
        fn all() {
            let mut s = default_segment();
            s.left_separator = ">".into();
            s.right_separator = "<".into();
            s.icon = "$".into();
            assert_eq!(&s.compute_value(), ">$test<");
        }

        #[test]
        fn hide_if_empty_false() {
            let mut s = default_segment();
            s.kind = Constant::new("".into()).into();
            s.left_separator = ">".into();
            s.right_separator = "<".into();
            s.icon = "$".into();
            assert_eq!(&s.compute_value(), ">$<");
        }

        #[test]
        fn hide_if_empty_true() {
            let mut s = default_segment();
            s.kind = Constant::new("".into()).into();
            s.left_separator = ">".into();
            s.right_separator = "<".into();
            s.icon = "$".into();
            s.hide_if_empty = true;
            assert_eq!(&s.compute_value(), "");
        }

        #[test]
        fn config_left_separator() {
            let kind = Constant::new("test".into()).into();
            let segment = Segment::new(
                kind,
                None,
                vec![],
                0,
                None,
                None,
                None,
                false,
                &Configuration {
                    left_separator: Some(">".into()),
                    right_separator: None,
                    script_dir: PathBuf::default(),
                    update_all_signal: None,
                },
            );
            assert_eq!(&segment.compute_value(), ">test")
        }

        #[test]
        fn config_left_separator_overwrite() {
            let kind = Constant::new("test".into()).into();
            let segment = Segment::new(
                kind,
                None,
                vec![],
                0,
                Some("!".into()),
                None,
                None,
                false,
                &Configuration {
                    left_separator: Some(">".into()),
                    right_separator: None,
                    script_dir: PathBuf::default(),
                    update_all_signal: None,
                },
            );
            assert_eq!(&segment.compute_value(), "!test")
        }
    }
}
