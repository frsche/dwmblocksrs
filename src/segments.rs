pub mod constant;
pub mod program_output;

use std::time::{Duration, Instant};

use async_std::channel::Sender;
use async_std::future::timeout;
use async_std::stream::StreamExt;
use lazy_static::lazy_static;
use log::warn;
use signal_hook_async_std::Signals;
use std::fmt::Debug;

use crate::color::{Colorable, SegmentColoring};
use crate::config::Configuration;
use crate::SegmentId;

lazy_static! {
    static ref SIGRTMIN: i32 = libc::SIGRTMIN();
    static ref SIGRTMAX: i32 = libc::SIGRTMAX();
}

#[derive(Debug)]
pub struct Segment {
    kind: Box<dyn SegmentKind>,
    update_interval: Option<Duration>,
    signals: Vec<i32>,

    pub left_separator: String,
    pub right_separator: String,
    pub icon: String,
    pub hide_if_empty: bool,

    pub coloring: SegmentColoring,
}

pub trait SegmentKind: Debug + Send + Sync {
    fn compute_value(&mut self) -> String;
}

impl Segment {
    pub fn new(
        kind: Box<dyn SegmentKind>,
        update_interval: Option<Duration>,
        signal_offsets: Vec<u32>,
    ) -> Result<Self, String> {
        Ok(Self {
            kind,
            update_interval,
            signals: Self::convert_signal_offsets(signal_offsets)?,
            left_separator: Default::default(),
            right_separator: Default::default(),
            icon: Default::default(),
            hide_if_empty: Default::default(),
            coloring: Default::default(),
        })
    }

    pub(crate) fn new_from_config(
        kind: Box<dyn SegmentKind>,
        update_interval: Option<Duration>,
        signal_offsets: Vec<u32>,
        left_separator: Option<String>,
        right_separator: Option<String>,
        icon: Option<String>,
        hide_if_empty: bool,
        coloring: SegmentColoring,
        config: &Configuration,
    ) -> Result<Self, String> {
        let left_separator = left_separator
            .or_else(|| config.left_separator.clone())
            .unwrap_or_else(|| "".into());
        let right_separator = right_separator
            .or_else(|| config.right_separator.clone())
            .unwrap_or_else(|| "".into());
        let icon = icon.unwrap_or_else(|| "".into());
        let coloring = coloring.or_default(&config.coloring);

        Ok(Segment {
            kind,
            update_interval,
            signals: Self::convert_signal_offsets(signal_offsets)?,

            left_separator,
            right_separator,
            icon,
            hide_if_empty,

            coloring,
        })
    }

    fn convert_signal_offsets(signal_offsets: Vec<u32>) -> Result<Vec<i32>, String> {
        let signals = signal_offsets
            .into_iter()
            .map(move |signal| signal as i32 + *SIGRTMIN)
            .collect::<Vec<_>>();

        if signals.iter().any(|signal| *signal > *SIGRTMAX) {
            return Err("A used signal is greater than SIGRTMAX.".into());
        }

        Ok(signals)
    }

    pub(crate) async fn run_update_loop(
        mut self,
        id: SegmentId,
        channel: Sender<(SegmentId, String)>,
    ) {
        // register_signal handler
        let mut signals = Signals::new(&self.signals).unwrap();

        loop {
            let last_update = Instant::now();
            // compute initial value for that segment and send it through the channel
            channel.send((id, self.compute_value())).await.unwrap();

            // if we have an update interval for that segment
            if let Some(update_interval) = self.update_interval {
                // calculate time since the last update
                let duration = Instant::elapsed(&last_update);

                // if we still have some time to wait until the next update
                match update_interval.checked_sub(duration) {
                    Some(duration) => {
                        // wait for signals or timeout at that duration
                        let _ = timeout(duration, signals.next()).await;
                    }
                    // otherwise, update directly
                    None => warn!("execution of segment {id} took longer than update interval"),
                };
            } else {
                // if we have no periodic updates, simply wait for signals
                signals.next().await;
            };
        }
    }

    pub(crate) fn compute_value(&mut self) -> String {
        let new_value = self.kind.compute_value();

        if self.hide_if_empty && new_value.is_empty() {
            return "".into();
        }

        format!(
            "{}{}{}{}",
            self.left_separator.color(self.coloring.left_separator),
            self.icon.color(self.coloring.icon),
            new_value.color(self.coloring.text),
            self.right_separator.color(self.coloring.right_separator)
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
                left_separator: Default::default(),
                right_separator: Default::default(),
                icon: Default::default(),
                hide_if_empty: Default::default(),
                coloring: Default::default(),
            }
        }
    }

    mod segment {
        use crate::color::Color;

        use super::*;

        #[test]
        fn consant() {
            let mut s: Segment = Default::default();
            assert_eq!(&s.compute_value(), "test");
        }

        #[test]
        fn left_separator() {
            let mut s = Segment {
                left_separator: ">".into(),
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), ">test");
        }

        #[test]
        fn right_separator() {
            let mut s = Segment {
                right_separator: "<".into(),
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), "test<");
        }

        #[test]
        fn icon() {
            let mut s = Segment {
                icon: "$".into(),
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), "$test");
        }

        #[test]
        fn all() {
            let mut s = Segment {
                left_separator: ">".into(),
                right_separator: "<".into(),
                icon: "$".into(),
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), ">$test<");
        }

        #[test]
        fn hide_if_empty_false() {
            let mut s = Segment {
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
            let mut s = Segment {
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
        fn color_text() {
            let mut s = Segment {
                coloring: SegmentColoring {
                    text: Color::Colored(2),
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), "\x02test\x01");
        }

        #[test]
        fn color_left_separator() {
            let mut s = Segment {
                left_separator: ">".into(),
                coloring: SegmentColoring {
                    left_separator: Color::Colored(2),
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), "\x02>\x01test");
        }

        #[test]
        fn color_right_separator() {
            let mut s = Segment {
                right_separator: "<".into(),
                coloring: SegmentColoring {
                    right_separator: Color::Colored(2),
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), "test\x02<\x01");
        }

        #[test]
        fn color_icon() {
            let mut s = Segment {
                icon: "$".into(),
                coloring: SegmentColoring {
                    icon: Color::Colored(2),
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(&s.compute_value(), "\x02$\x01test");
        }

        #[test]
        fn all_colors() {
            let mut s = Segment {
                left_separator: ">".into(),
                right_separator: "<".into(),
                icon: "$".into(),
                coloring: SegmentColoring {
                    left_separator: Color::Colored(2),
                    icon: Color::Colored(3),
                    text: Color::Colored(4),
                    right_separator: Color::Colored(5),
                },
                ..Default::default()
            };
            assert_eq!(
                &s.compute_value(),
                "\x02>\x01\x03$\x01\x04test\x01\x05<\x01"
            );
        }

        #[test]
        fn config_left_separator() {
            let kind = Box::new(Constant::new("test".into()));
            let mut segment = Segment::new_from_config(
                kind,
                None,
                vec![],
                None,
                None,
                None,
                false,
                Default::default(),
                &Configuration {
                    left_separator: Some(">".into()),
                    right_separator: None,
                    script_dir: PathBuf::default(),
                    update_all_signal: None,
                    coloring: Default::default(),
                },
            )
            .unwrap();
            assert_eq!(&segment.compute_value(), ">test")
        }

        #[test]
        fn config_left_separator_overwrite() {
            let kind = Box::new(Constant::new("test".into()));
            let mut segment = Segment::new_from_config(
                kind,
                None,
                vec![],
                Some("!".into()),
                None,
                None,
                false,
                Default::default(),
                &Configuration {
                    left_separator: Some(">".into()),
                    right_separator: None,
                    script_dir: PathBuf::default(),
                    update_all_signal: None,
                    coloring: Default::default(),
                },
            )
            .unwrap();
            assert_eq!(&segment.compute_value(), "!test")
        }

        #[test]
        fn config_right_separator_overwrite() {
            let kind = Box::new(Constant::new("test".into()));
            let mut segment = Segment::new_from_config(
                kind,
                None,
                vec![],
                None,
                Some("!".into()),
                None,
                false,
                Default::default(),
                &Configuration {
                    left_separator: None,
                    right_separator: Some(">".into()),
                    script_dir: PathBuf::default(),
                    update_all_signal: None,
                    coloring: Default::default(),
                },
            )
            .unwrap();
            assert_eq!(&segment.compute_value(), "test!")
        }

        #[test]
        fn config_color() {
            let kind = Box::new(Constant::new("test".into()));
            let mut segment = Segment::new_from_config(
                kind,
                None,
                vec![],
                Some(">".into()),
                Some("<".into()),
                Some("$".into()),
                false,
                Default::default(),
                &Configuration {
                    coloring: SegmentColoring {
                        left_separator: Color::Colored(2),
                        icon: Color::Colored(3),
                        text: Color::Colored(4),
                        right_separator: Color::Colored(5),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(
                &segment.compute_value(),
                "\x02>\x01\x03$\x01\x04test\x01\x05<\x01"
            )
        }

        #[test]
        fn config_color_overwrite() {
            let kind = Box::new(Constant::new("test".into()));
            let mut segment = Segment::new_from_config(
                kind,
                None,
                vec![],
                Some(">".into()),
                Some("<".into()),
                Some("$".into()),
                false,
                SegmentColoring {
                    left_separator: Color::Colored(6),
                    icon: Color::Colored(7),
                    text: Color::Colored(8),
                    right_separator: Color::Colored(9),
                },
                &Configuration {
                    coloring: SegmentColoring {
                        left_separator: Color::Colored(2),
                        icon: Color::Colored(3),
                        text: Color::Colored(4),
                        right_separator: Color::Colored(5),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(
                &segment.compute_value(),
                "\x06>\x01\x07$\x01\x08test\x01\x09<\x01"
            )
        }
    }
}
