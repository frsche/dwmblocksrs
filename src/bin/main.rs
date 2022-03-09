use clap::{Command, Arg};
use std::path::PathBuf;
use dwmblocksrs::run_with_config;
use log::{Level, error, info};

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
            let mut config_path = dirs::config_dir().expect("config directory does not exist");
            config_path.push(PathBuf::from("dwmblocksrs/dwmblocksrs.yaml"));
            config_path
        });

    info!("loading config file '{}'", config_path.to_str().unwrap());

    if let Err(e) = run_with_config(config_path).await {
        error!("{e}");
    }
}
