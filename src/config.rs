use std::{fs::read_to_string, path::PathBuf, time::Duration};

use format_serde_error::SerdeError;
use serde::{Deserialize, Serialize};

use crate::segments::{self, Segment, SegmentId};

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
pub(crate) struct Configuration {
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

    let script_dir = script_dir.map(PathBuf::from).unwrap_or_default();

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
        .enumerate()
        .map(|(segment_id, segment_config)| {
            parse_segment(segment_config, segment_id, &configuration)
        })
        .collect::<Result<Vec<Segment>, String>>()?;

    Ok(segments)
}
fn parse_segment(
    segment_config: SegmentConfig,
    segment_id: SegmentId,
    config: &Configuration,
) -> Result<Segment, String> {
    let SegmentConfig {
        update_interval,
        signals: mut signal_offsets,
        kind,
        left_separator,
        right_separator,
        icon,
        hide_if_empty,
    } = segment_config;

    let kind = match kind {
        SegmentKindConfig::Program { program, args } => {
            segments::program_output::ProgramOutput::new(program.into(), args).into()
        }
        SegmentKindConfig::ShellScript { script, mut args } => {
            let mut script_path = config.script_dir.clone();
            script_path.push(PathBuf::from(script));
            args.insert(0, script_path.to_str().unwrap().into());

            segments::program_output::ProgramOutput::new("/bin/sh".into(), args).into()
        }
        SegmentKindConfig::Constant { constant } => {
            segments::constant::Constant::new(constant.clone()).into()
        }
    };

    let sigrtmin = libc::SIGRTMIN();
    let sigrtmax = libc::SIGRTMAX();

    if let Some(offset) = config.update_all_signal {
        signal_offsets.push(offset);
    }
    let signals = signal_offsets
                .into_iter()
                .map(|offset| {
                    let signal = offset as i32 + sigrtmin;
                    if signal >= sigrtmin && signal <= sigrtmax {Ok(signal)} else {
                        Err(format!("signal offset {offset} results in signal {signal}, which is not in the valid range"))}
                })
                .collect::<Result<Vec<_>,_>>()?;

    let update_interval = update_interval.map(Duration::from_secs);

    Ok(Segment::new(
        kind,
        update_interval,
        signals,
        segment_id,
        left_separator,
        right_separator,
        icon,
        hide_if_empty,
        &config,
    ))
}
