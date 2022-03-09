use std::{collections::HashMap, fs::read_to_string, path::PathBuf, time::Duration};

use format_serde_error::SerdeError;
use serde::Deserialize;

use crate::{
    color::{Color, SegmentColoring},
    segments::{self, Segment, SegmentKind},
};

#[derive(Deserialize, Debug)]
struct ConfigFile {
    segments: Vec<SegmentConfig>,

    left_separator: Option<String>,
    right_separator: Option<String>,

    update_all_signal: Option<u32>,
    script_dir: Option<String>,

    #[serde(default)]
    colors: HashMap<String, Color>,
    #[serde(flatten)]
    coloring: SegmentColorConfig,
}

#[derive(Deserialize, Debug)]
struct SegmentConfig {
    #[serde(flatten)]
    kind: SegmentKindConfig,
    update_interval: Option<u64>,
    #[serde(default)]
    signals: Vec<u32>,

    left_separator: Option<String>,
    right_separator: Option<String>,
    icon: Option<String>,
    #[serde(default)]
    hide_if_empty: bool,

    #[serde(flatten)]
    coloring: SegmentColorConfig,
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
struct SegmentColorConfig {
    text: Option<String>,
    left_separator: Option<String>,
    right_separator: Option<String>,
    icon: Option<String>,
}

#[derive(Debug, Default)]
pub struct Configuration {
    pub script_dir: PathBuf,
    pub update_all_signal: Option<u32>,

    // defaults
    pub coloring: SegmentColoring,
    pub left_separator: Option<String>,
    pub right_separator: Option<String>,
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
        colors,
        coloring,
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

    let coloring = SegmentColoring::from(coloring, &colors)?;

    let configuration = Configuration {
        left_separator,
        right_separator,

        script_dir,
        update_all_signal,
        coloring,
    };

    let segments = segments
        .into_iter()
        .map(|segment_config| parse_segment(segment_config, &configuration, &colors))
        .collect::<Result<Vec<Segment>, String>>()?;

    Ok(segments)
}

fn parse_segment(
    segment_config: SegmentConfig,
    config: &Configuration,
    colors: &HashMap<String, Color>,
) -> Result<Segment, String> {
    let SegmentConfig {
        kind,
        update_interval,
        mut signals,
        left_separator,
        right_separator,
        icon,
        hide_if_empty,

        coloring,
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

    let coloring = SegmentColoring::from(coloring, &colors)?;

    if let Some(offset) = config.update_all_signal {
        signals.push(offset);
    }

    let update_interval = update_interval.map(Duration::from_secs);

    Ok(Segment::new_from_config(
        kind,
        update_interval,
        signals,
        left_separator,
        right_separator,
        icon,
        hide_if_empty,
        coloring,
        config,
    ))
}

fn expand_path<T: AsRef<str>>(path_str: T) -> Result<PathBuf, String> {
    let str = shellexpand::full(&path_str).map_err(|x| x.to_string())?;
    Ok(PathBuf::from(str.as_ref()))
}

impl SegmentColoring {
    fn from(
        c: SegmentColorConfig,
        mapping: &HashMap<String, Color>,
    ) -> Result<SegmentColoring, String> {
        let SegmentColorConfig {
            text,
            left_separator,
            right_separator,
            icon,
        } = c;

        let text = Self::color_lookup(text, mapping)?;
        let left_separator = Self::color_lookup(left_separator, mapping)?;
        let right_separator = Self::color_lookup(right_separator, mapping)?;
        let icon = Self::color_lookup(icon, mapping)?;

        Ok(Self {
            text,
            left_separator,
            right_separator,
            icon,
        })
    }

    fn color_lookup(
        c: Option<String>,
        mapping: &HashMap<String, Color>,
    ) -> Result<Option<Color>, String> {
        match c {
            Some(c) => match mapping.get(&c) {
                Some(&c) => Ok(Some(c)),
                None => Err(format!("undefined color: {c}")),
            },
            None => Ok(None),
        }
    }
}
