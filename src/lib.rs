pub(crate) mod color;
mod config;
pub mod segments;
mod status_bar;

use std::path::PathBuf;

use async_std::channel;
use async_std::prelude::*;
use async_std::task;
use config::parse_config;
use segments::Segment;
use status_bar::StatusBar;

pub(crate) type SegmentId = usize;

/// Run the statusbar with the given segments
pub async fn run(segments: Vec<Segment>) -> Result<(), String> {
    // when a segment should get updated, it's id is send through this channel
    let (tx, mut rx) = channel::unbounded::<(SegmentId, String)>();

    let num_segments = segments.len();

    // for each segment we spawn a task that requests updates according to the update period
    for (id, segment) in segments.into_iter().enumerate() {
        let sender = tx.clone();
        task::spawn(async move { segment.run_update_loop(id, sender).await });
    }

    let mut status_bar = StatusBar::new(num_segments);

    // wait for a new update to arrive
    while let Some((id, text)) = rx.next().await {
        // and update that segment in the status bar
        status_bar.update_segment(id, text);
    }

    Ok(())
}

/// Run the statusbar with the given configuration file
pub async fn run_with_config(config_path: PathBuf) -> Result<(), String> {
    let segments = parse_config(config_path)?;
    run(segments).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_config() {
        let mut segments = parse_config("test_config.yaml".into()).expect("config should parse");
        let status_text = segments
            .iter_mut()
            .map(|s| s.compute_value())
            .collect::<String>();
        assert_eq!(
            "Segment1 | Segment2 | hello world |  | %%% | $>>><<<",
            status_text
        );
    }

    #[test]
    fn test_sample_config_color() {
        let mut segments = parse_config("test_config_color.yaml".into()).unwrap();
        let status_text = segments
            .iter_mut()
            .map(|s| s.compute_value())
            .collect::<String>();
        assert_eq!(
            "\x02>\x01\x02$\x01\x02test\x01\x03<\x01\x03>\x01\x02segment\x01<",
            status_text
        );
    }
}
