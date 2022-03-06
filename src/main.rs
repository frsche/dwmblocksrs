mod config;
pub mod segments;
mod signals;

use std::{
    ffi::CString,
    path::PathBuf,
    ptr,
    sync::mpsc::{channel, Receiver},
    time::{Duration, Instant},
};

use clap::{Arg, Command};
use config::{parse_config, Configuration};
use log::{error, info};
use num_integer::gcd;
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

fn update_interval(segments: &[Segment]) -> Option<Duration> {
    // the update interval is the gcd out of all individual update intervals
    let interval = segments
        .iter()
        .filter_map(|segment| segment.update_interval)
        .map(|duration| duration.as_millis() as u64)
        .reduce(gcd)
        .map(Duration::from_millis);

    if let Some(interval) = interval.as_ref() {
        info!("calculated general update interval of {:?}", interval);
    } else {
        info!("segments are updated through signals only")
    }

    interval
}

fn get_status_text(segments: &[Segment], now: &Instant) -> String {
    segments
        .iter()
        .map(|s| s.get_value(&now))
        .collect::<String>()
}

fn update_loop(segments: Vec<Segment>, signals: Receiver<usize>, config: Configuration) {
    // use the general update interval provided in the configuration
    let update_interval = config
        .update_interval
        // or else calculate the update interval based on the individual intervals
        .or_else(|| update_interval(&segments));

    // set the initial status bar text
    let mut last_status_text = get_status_text(&segments, &Instant::now());
    set_statusbar(&last_status_text);

    loop {
        let now = Instant::now();

        // wait for a signal to arrive in the channel
        // if we have a general update interval, wait that long
        // otherwise (only signals are used), wait infinitly
        if let Ok(segment_ref) = match update_interval {
            Some(interval) => signals.recv_timeout(interval),
            None => Ok(signals.recv().unwrap()),
        } {
            // if a signal arrived, manually update the segment
            segments[segment_ref].update(&now);
        };

        // get the current status text
        // here, all the segments calculate a new value
        // either a cached one, or a new one if the corresponding interval exceeded
        let status_text = get_status_text(&segments, &now);

        // only set the text if it has changed
        if last_status_text != status_text {
            set_statusbar(&status_text);
            last_status_text = status_text;
        }
    }
}

fn run(config: PathBuf) -> Result<(), String> {
    let (segments, config) = parse_config(config)?;

    let (tx, rx) = channel::<SegmentReference>();
    spawn_signal_handler(&segments, tx);

    update_loop(segments, rx, config);

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
