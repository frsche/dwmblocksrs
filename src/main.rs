mod config;
pub mod segments;
mod signals;

use std::{
    ffi::CString,
    path::PathBuf,
    ptr,
    sync::mpsc::{channel, Receiver},
    thread,
};

use clap::{Arg, Command};
use config::parse_config;
use log::{error, info};
use segments::{Segment, SegmentReference};
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

fn get_status_text(segments: &[Segment]) -> String {
    segments
        .iter()
        .map(|s| s.current_value())
        .collect::<String>()
}

fn update_loop(segments: Vec<Segment>, signals: Receiver<usize>) {
    let mut last_status_text = "".into();

    for segment_ref in signals.iter() {
        segments[segment_ref].update();
        let status_text = get_status_text(&segments);

        // only set the text if it has changed
        if last_status_text != status_text {
            set_statusbar(&status_text);
            last_status_text = status_text;
        }
    }
}

fn run(config: PathBuf) -> Result<(), String> {
    let segments = parse_config(config)?;

    let (tx, rx) = channel::<SegmentReference>();

    for (segment_ref, interval) in segments
        .iter()
        .filter_map(|s| {
            if let Some(interval) = s.update_interval {
                Some(interval)
            } else {
                None
            }
        })
        .enumerate()
    {
        let channel = tx.clone();
        thread::spawn(move || loop {
            thread::sleep(interval);
            channel.send(segment_ref).unwrap();
        });
    }

    spawn_signal_handler(&segments, tx);

    update_loop(segments, rx);

    Ok(())
}

fn main() {
    simple_logger::init().unwrap();

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

    if let Err(e) = run(config_path) {
        error!("{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_config() {
        let segments = parse_config("test_config.yaml".into()).expect("config should parse");
        let status_text = get_status_text(&segments);
        println!("{}", status_text);
        assert_eq!(
            "Segment1 | Segment2 | hello world |  | %%% | $>>><<<",
            status_text
        );
    }
}
