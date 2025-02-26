use core::str;
use env_logger::Env;
use log::*;
use std::io::BufRead;
use std::io::BufReader;
use std::time::Duration;

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

    #[arg(long = "tether.role", default_value_t = String::from("serial"))]
    tether_role: String,
}

fn main() {
    let cli = Cli::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or(&cli.log_level))
        .filter_module("tether_agent", log::LevelFilter::Warn)
        .filter_module("symphonia_core", log::LevelFilter::Warn)
        .filter_module("symphonia_bundle_mp3", log::LevelFilter::Warn)
        .init();

    let mut tether_agent = TetherAgentOptionsBuilder::new(&cli.tether_role)
        .build()
        .expect("failed to init Tether Agent");

    let output_plug = PlugOptionsBuilder::create_output("values")
        .build(&mut tether_agent)
        .unwrap();

    let port = serialport::new(&cli.port, cli.baud_rate)
        .timeout(Duration::from_millis(100))
        .open();

    match port {
        Ok(mut serial_port) => {
            // let mut serial_buf: Vec<u8> = vec![0; 32];
            info!(
                "Receiving data on {} at {} baud...",
                &cli.port, cli.baud_rate
            );
            loop {
                let mut reader = BufReader::new(&mut serial_port);
                let mut value = String::new();
                if reader.read_line(&mut value).is_ok() {
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
            eprintln!("Failed to open \"{}\". Error: {}", &cli.port, e);
            panic!("Failed to open port");
        }
    }
}
