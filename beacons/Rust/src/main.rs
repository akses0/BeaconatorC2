use clap::{Parser, ValueEnum};
use tracing::{info, debug};
use sysinfo::System;
use mac_address::get_mac_address;
use std::{env, fmt::Display};
use md5::{Md5, Digest};
use hex::encode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(version, about, long_about = format!("\t\t\t<3 Rusty Beacon version {} Ɛ>\r\nA reasonably simple rust implementation of a beacon for the BeaconatorC2 project.", VERSION))]
struct Args {
    #[arg(short = 'i', long = "ip_address", default_value = "127.0.0.1")]
    ip_address: String,

    #[arg(short = 'p', long = "port", default_value = "5074")]
    port: u16,

    #[arg(short = 'e', long = "encoding", value_enum, default_value = "plain-text")]
    encoding: EncodingStrategy,

    #[arg(short = 's', long = "schema_file", default_value = "rusty_beacon.yaml")]
    schema_file: String,

    #[arg(short = 'n', long = "interval", default_value = "15")]
    interval: u8,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum EncodingStrategy {
    PlainText,
    Base64,
    Xor,
    Rot13,
}

impl Display for EncodingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodingStrategy::PlainText => write!(f, "plain-text"),
            EncodingStrategy::Base64 => write!(f, "base64"),
            EncodingStrategy::Xor => write!(f, "xor"),
            EncodingStrategy::Rot13 => write!(f, "rot13"),
        }
    }
}

struct Config {
    beacon_id: String,
    ip_address: String,
    port: u16,
    encoding: EncodingStrategy,
    schema_file: String,
    interval: u8,
    jitter: u8,
}

fn main() {
    let args = Args::parse();
    let config = Config {
        beacon_id: generate_beacon_id(),
        ip_address: args.ip_address,
        port: args.port,
        encoding: EncodingStrategy::PlainText,
        schema_file: args.schema_file,
        interval: args.interval,
        jitter: 0,
    };

    tracing_subscriber::fmt::init();
    info!("[+] rusty_beacon version {} is alive", VERSION);
    info!(" |- Beacon Id:\t\t{}", config.beacon_id);
    info!(" |- Server IP:\t\t{}", config.ip_address);
    info!(" |- Server Port:\t{}", config.port);
    info!(" |- Encoding:\t\t{}", config.encoding);
    info!(" |- Schema File:\t{}", config.schema_file);
    info!(" |- Interval:\t\t{}", config.interval);
    info!(" |- Jitter:\t\t{}", config.jitter);
}

fn generate_beacon_id() -> String {
    let mut system_info = format!("{}{}", System::host_name().unwrap_or_else(|| "unknown_host".to_string()), std::env::consts::OS);
    let username = std::env::var("USER").unwrap_or_default();
    system_info.push_str(&format!("{}", username));

    match get_mac_address() {
        Ok(mac) => system_info.push_str(&format!("{}", mac.unwrap_or_default())),
        Err(_) => system_info.push_str(&format!("{}", "unknown_mac")),
    }

    if let Ok(exe) = env::current_exe() {
        system_info.push_str(&exe.to_string_lossy());
    }

    debug!("system_info: {}", system_info);
    let mut hasher = Md5::new();
    hasher.update(system_info.as_bytes());
    let result = hasher.finalize();
    let hash = encode(result);

    /* return */ hash
}
