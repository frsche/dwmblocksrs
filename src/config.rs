use std::{fs::read_to_string, path::PathBuf, time::Duration};

use format_serde_error::SerdeError;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::segments::{self, Segment, SegmentKind};

lazy_static! {
    static ref SIGRTMIN: i32 = libc::SIGRTMIN();
    static ref SIGRTMAX: i32 = libc::SIGRTMAX();
}

#[derive(Deserialize, Serialize, Debug)]
struct ConfigFile {
    segments: Vec<SegmentConfig>,
    left_separator: Option<String>,
    right_separator: Option<String>,
    update_all_signal: Option<u32>,
    script_dir: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct SegmentConfig {
    update_interval: Option<u64>,
    #[serde(default)]
    signals: Vec<u32>,
    #[serde(flatten)]
    kind: SegmentKindConfig,
    left_separator: Option<String>,
    right_separator: Option<String>,
    icon: Option<String>,
    #[serde(default)]
    hide_if_empty: bool,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum SegmentKindConfig {
    Program {
        program: String,
        #[serde(default)]
        args: Vec<String>,
    },
    ShellScript {
        script: String,
        #[serde(default)]
        args: Vec<String>,
    },
    Constant {
        constant: String,
    },
}

#[derive(Debug)]
pub struct Configuration {
    pub left_separator: Option<String>,
    pub right_separator: Option<String>,
    pub script_dir: PathBuf,
    pub update_all_signal: Option<u32>,
}

pub(crate) fn parse_config(config: PathBuf) -> Result<Vec<Segment>, String> {
    let config_str = read_to_string(&config).map_err(|e| {
        format!(
            "Error reading config file '{}': {}",
            config.to_str().unwrap(),
            e
        )
    })?;

    let ConfigFile {
        segments,
        left_separator,
        right_separator,
        update_all_signal,
        script_dir,
    } = serde_yaml::from_str(&config_str)
        .map_err(|e| SerdeError::new(config_str, e).to_string())?;

    let script_dir = script_dir
        .map(expand_path)
        .unwrap_or_else(|| Ok(Default::default()))?;

    if !script_dir.is_dir() {
        return Err(format!(
            "script directory '{}' does not exist.",
            script_dir.to_str().unwrap()
        ));
    }

    let configuration = Configuration {
        left_separator,
        right_separator,
        script_dir,
        update_all_signal,
    };

    let segments = segments
        .into_iter()
        .map(|segment_config| parse_segment(segment_config, &configuration))
        .collect::<Result<Vec<Segment>, String>>()?;

    Ok(segments)
}
fn parse_segment(segment_config: SegmentConfig, config: &Configuration) -> Result<Segment, String> {
    let SegmentConfig {
        update_interval,
        signals: mut signal_offsets,
        kind,
        left_separator,
        right_separator,
        icon,
        hide_if_empty,
    } = segment_config;

    let kind: Box<dyn SegmentKind> = match kind {
        SegmentKindConfig::Program { program, args } => Box::new(
            segments::program_output::ProgramOutput::new(expand_path(program)?, args),
        ),
        SegmentKindConfig::ShellScript { script, mut args } => {
            let mut script_path = config.script_dir.clone();
            script_path.push(expand_path(script)?);
            args.insert(0, script_path.to_str().unwrap().into());

            Box::new(segments::program_output::ProgramOutput::new(
                "/bin/sh".into(),
                args,
            ))
        }
        SegmentKindConfig::Constant { constant } => {
            Box::new(segments::constant::Constant::new(constant))
        }
    };

    if let Some(offset) = config.update_all_signal {
        signal_offsets.push(offset);
    }
    let signals = signal_offsets
                .into_iter()
                .map(|offset| {
                    let signal = offset as i32 + *SIGRTMIN;
                    if signal >= *SIGRTMIN && signal <= *SIGRTMAX {Ok(signal)} else {
                        Err(format!("signal offset {offset} results in signal {signal}, which is not in the valid range"))}
                })
                .collect::<Result<Vec<_>,_>>()?;

    let update_interval = update_interval.map(Duration::from_secs);

    Ok(Segment::new_from_config(
        kind,
        update_interval,
        signals,
        left_separator,
        right_separator,
        icon,
        hide_if_empty,
        config,
    ))
}

fn expand_path<T: AsRef<str>>(path_str: T) -> Result<PathBuf, String> {
    let str = shellexpand::full(&path_str).map_err(|x| x.to_string())?;
    Ok(PathBuf::from(str.as_ref()))
}
