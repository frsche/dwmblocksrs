mod config;
mod segments;
mod signals;
mod status_bar;

use std::path::PathBuf;

use async_std::channel;
use async_std::prelude::*;
use clap::{Arg, Command};
use config::parse_config;
use log::{error, info, Level};
use segments::Segment;
use segments::SegmentId;
use signals::spawn_signal_handler;
use status_bar::StatusBar;

async fn run(segments: Vec<Segment>) -> Result<(), String> {
    // when a segment should get updated, it's id is send through this channel
    let (tx, mut rx) = channel::unbounded::<SegmentId>();

    // for each segment we spawn a task that requests updates according to the update period
    for segment in &segments {
        segment.run_update_loop(tx.clone()).await;
    }

    spawn_signal_handler(&segments, tx).await;

    let mut status_bar = StatusBar::new(segments);

    // wait for a new update to arrive
    while let Some(segment) = rx.next().await {
        // and update that segment in the status bar
        status_bar.update_segment(segment)
    }

    Ok(())
}

pub async fn run_with_config(config_path: PathBuf) -> Result<(), String> {
    let segments = parse_config(config_path)?;
    run(segments).await
}

#[async_std::main]
async fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();

    let matches = Command::new("dwmblocksrs")
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .value_name("config_path")
                .help("the path to the configuration file"),
        )
        .get_matches();

    // use the path provided as the argument for the configuration
    let config_path = matches
        .value_of("config")
        .map(PathBuf::from)
        // or else look for the configuration file in the config directory
        .unwrap_or_else(|| {
            let mut config_path = dirs::config_dir().expect("config dir should exist");
            config_path.push(PathBuf::from("dwmblocksrs/dwmblocksrs.yaml"));
            config_path
        });

    info!("loading config file '{}'", config_path.to_str().unwrap());

    if let Err(e) = run_with_config(config_path).await {
        error!("{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_config() {
        let segments = parse_config("test_config.yaml".into()).expect("config should parse");
        let status_text = segments
            .iter()
            .map(|s| s.compute_value())
            .collect::<String>();
        assert_eq!(
            "Segment1 | Segment2 | hello world |  | %%% | $>>><<<",
            status_text
        );
    }
}
