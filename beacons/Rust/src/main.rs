use clap::Parser;
use tracing::info;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(version, about, long_about = format!("\t\t\t<3 Rusty Beacon version {} Ɛ>\r\nA reasonably simple rust implementation of a beacon for the BeaconatorC2 project.", VERSION))]
struct Args {
    #[arg(short = 'i', long = "ip_address")]
    ip_address: String,

    #[arg(short = 'p', long = "port")]
    port: u16,
}

fn main() {
    let args = Args::parse();

    tracing_subscriber::fmt::init();
    info!("[+] rusty_beacon version {} is alive", VERSION);
    info!(" |- Server IP: {}", args.ip_address);
    info!(" |- Server Port: {}", args.port);
}
