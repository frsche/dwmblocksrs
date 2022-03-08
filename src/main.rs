mod config;
pub mod segments;
mod signals;

use async_std::prelude::*;
use std::{ffi::CString, path::PathBuf, ptr};

use async_std::channel::{self, Receiver};
use clap::{Arg, Command};
use config::parse_config;
use log::{error, info, Level};
use segments::Segment;
use signals::spawn_signal_handler;
use x11::xlib::{XCloseDisplay, XDefaultScreen, XOpenDisplay, XRootWindow, XStoreName};

fn set_statusbar(status_text: &str) {
    unsafe {
        // https://github.com/hugglesfox/statusd/blob/main/src/xsetroot.rs
        // https://github.com/KJ002/simple_status/blob/main/src/status.rs
        let display = XOpenDisplay(ptr::null());
        let screen = XDefaultScreen(display);
        let window = XRootWindow(display, screen);

        let c_str = CString::new(status_text).unwrap();

        XStoreName(display, window, c_str.as_ptr() as *const i8);

        XCloseDisplay(display);
    }
}

async fn update_loop(segments: Vec<Segment>, mut updates: Receiver<usize>) {
    // a vector with the current text for each segment
    let mut current_status = segments
        .iter()
        .map(|s| s.compute_value())
        .collect::<Vec<_>>();

    // set the initial status text
    let mut last_status_text = current_status.join("");
    set_statusbar(&last_status_text);

    // wait for a new update
    while let Some(segment_ref) = updates.next().await {
        // compute the new value of the requested segment
        current_status[segment_ref] = segments[segment_ref].compute_value();
        // and compute the new status text
        let status_text = current_status.join("");
        // only set the status text if it is different than before
        if last_status_text != status_text {
            set_statusbar(&status_text);
            last_status_text = status_text;
        }
    }
}

async fn run(config: PathBuf) -> Result<(), String> {
    let segments = parse_config(config)?;

    let (tx, rx) = channel::unbounded();

    for segment in &segments {
        segment.run_update_loop(tx.clone()).await;
    }

    spawn_signal_handler(&segments, tx).await;
    update_loop(segments, rx).await;

    Ok(())
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

    if let Err(e) = run(config_path).await {
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
