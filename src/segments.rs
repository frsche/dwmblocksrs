pub mod constant;
pub mod program_output;

use std::time::Duration;

use async_std::{channel::Sender, task};
use std::fmt::Debug;

use crate::config::Configuration;
use crate::SegmentId;

#[derive(Debug)]
pub struct Segment {
    kind: Box<dyn SegmentKind>,
    update_interval: Option<Duration>,
    pub signals: Vec<i32>,
    segment_id: SegmentId,

    left_separator: String,
    right_separator: String,
    icon: String,
    hide_if_empty: bool,
}

pub trait SegmentKind: Debug {
    fn compute_value(&self) -> String;
}

impl Segment {
    pub(crate) fn new(
        kind: Box<dyn SegmentKind>,
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
            .or_else(|| config.left_separator.clone())
            .unwrap_or_else(|| "".into());
        let right_separator = right_separator
            .or_else(|| config.right_separator.clone())
            .unwrap_or_else(|| "".into());
        let icon = icon.unwrap_or_else(|| "".into());

        Segment {
            kind,
            update_interval,
            signals,
            segment_id,

            left_separator,
            right_separator,
            icon,
            hide_if_empty,
        }
    }

    pub(crate) async fn run_update_loop(&self, channel: Sender<SegmentId>) {
        if let Some(interval) = self.update_interval {
            let segment_id = self.segment_id;
            task::spawn(async move {
                channel.send(segment_id).await.unwrap();
                task::sleep(interval).await;
            });
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segments::constant::Constant;
    use std::path::PathBuf;

    #[macro_export]
    macro_rules! test_segment_kinds {
        ( $( $name:ident: $segment:expr => $expect:expr, )+ ) => {
            mod segment_kinds {
                use super::*;
                $(
                    #[test]
                    fn $name() {
                    assert_eq!($segment.compute_value(), $expect);
                    }
                )+
            }
        }
    }

    impl Default for Segment {
        fn default() -> Self {
            Self {
                kind: Box::new(Constant::new("test".into())),
                update_interval: Default::default(),
                signals: Default::default(),
                segment_id: Default::default(),
                left_separator: Default::default(),
                right_separator: Default::default(),
                icon: Default::default(),
                hide_if_empty: Default::default(),
            }
        }
    }

    mod segment {
        use super::*;

        #[test]
        fn consant() {
            let s: Segment = Default::default();
            assert_eq!(&s.compute_value(), "test");
        }

        #[test]
        fn left_separator() {
            let s = Segment {
                left_separator: ">".into(),
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), ">test");
        }

        #[test]
        fn right_separator() {
            let s = Segment {
                right_separator: "<".into(),
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), "test<");
        }

        #[test]
        fn icon() {
            let s = Segment {
                icon: "$".into(),
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), "$test");
        }

        #[test]
        fn all() {
            let s = Segment {
                left_separator: ">".into(),
                right_separator: "<".into(),
                icon: "$".into(),
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), ">$test<");
        }

        #[test]
        fn hide_if_empty_false() {
            let s = Segment {
                kind: Box::new(Constant::new("".into())),
                left_separator: ">".into(),
                right_separator: "<".into(),
                icon: "$".into(),
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), ">$<");
        }

        #[test]
        fn hide_if_empty_true() {
            let s = Segment {
                kind: Box::new(Constant::new("".into())),
                left_separator: ">".into(),
                right_separator: "<".into(),
                icon: "$".into(),
                hide_if_empty: true,
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), "");
        }

        #[test]
        fn config_left_separator() {
            let kind = Box::new(Constant::new("test".into()));
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
            let kind = Box::new(Constant::new("test".into()));
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
