use core::str;
use env_logger::Env;
use log::*;
use std::io::BufRead;
use std::io::BufReader;
use std::time::Duration;
use std::time::SystemTime;

use clap::Parser;
use tether_agent::{PlugOptionsBuilder, TetherAgentOptionsBuilder};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long = "loglevel",default_value_t=String::from("info"))]
    pub log_level: String,

    #[arg(default_value_t=String::from("/dev/cu.usbmodem1201"))]
    port: String,

    #[arg(long = "baudRate", default_value_t = 9600)]
    baud_rate: u32,

    /// Role to use in Tether topics
    #[arg(long = "tether.role", default_value_t = String::from("serial"))]
    tether_role: String,

    /// ID to use in Tether topics
    #[arg(long = "tether.id", default_value_t = String::from("any"))]
    tether_id: String,

    /// Plug Name to use in Tether topics
    #[arg(long = "tether.plugName", default_value_t = String::from("values"))]
    tether_plug_name: String,

    /// How long to wait (ms) for first message to be relayed, or next message.
    /// Program will panic if this timeout is reached. Leave blank to disable the check.
    #[arg(long = "timeout")]
    timeout_ms: Option<usize>,
}

fn main() {
    let cli = Cli::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or(&cli.log_level))
        .filter_module("tether_agent", log::LevelFilter::Warn)
        .filter_module("symphonia_core", log::LevelFilter::Warn)
        .filter_module("symphonia_bundle_mp3", log::LevelFilter::Warn)
        .init();

    let mut tether_agent = TetherAgentOptionsBuilder::new(&cli.tether_role)
        .id(Some(&cli.tether_id))
        .build()
        .expect("failed to init Tether Agent");

    let output_plug = PlugOptionsBuilder::create_output(&cli.tether_plug_name)
        .build(&mut tether_agent)
        .unwrap();

    let port = serialport::new(&cli.port, cli.baud_rate)
        .timeout(Duration::from_millis(100))
        .open();

    let mut last_value_parsed = SystemTime::now();

    match port {
        Ok(mut serial_port) => {
            // let mut serial_buf: Vec<u8> = vec![0; 32];
            info!(
                "Receiving data on {} at {} baud...",
                &cli.port, cli.baud_rate
            );
            loop {
                if let Some(timeout) = cli.timeout_ms {
                    let elapsed = last_value_parsed.elapsed().unwrap();
                    if elapsed > Duration::from_millis(timeout as u64) {
                        error!("Elapsed {}ms > timeout {}ms", elapsed.as_millis(), timeout);
                        panic!("Reached timeout since start/previous message");
                    }
                }

                let mut reader = BufReader::new(&mut serial_port);
                let mut value = String::new();
                if reader.read_line(&mut value).is_ok() {
                    last_value_parsed = SystemTime::now();
                    let s = value.trim();
                    debug!("String: {}", s);
                    if let Ok(int_value) = s.parse::<u32>() {
                        debug!("uint32: {}", int_value);
                        tether_agent
                            .encode_and_publish(&output_plug, int_value)
                            .expect("failed to publish");
                    }
                    // if let Ok(float_value) = s.parse::<f32>() {
                    //     debug!("f32: {}", float_value);
                    // }
                }
            }
        }
        Err(e) => {
            error!("Failed to open \"{}\". Error: {}", &cli.port, e);
            panic!("Failed to open port");
        }
    }
}
