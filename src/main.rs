use std::time::Duration;
use std::io::{self, Write, Read};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "toolbox")]
#[command(about = "A CLI tool with multiple subcommands", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Converts a file from one format to another
    Convert {
        /// Input file
        #[arg(short, long)]
        input: String,

        /// Output file
        #[arg(short, long)]
        output: String,

        /// Format to convert to
        #[arg(short, long)]
        format: String,
    },

    /// Runs the main application logic
    Run {
        /// Configuration file
        #[arg(short, long)]
        config: Option<String>,

        /// Enable debug mode
        #[arg(long)]
        debug: bool,

        /// Serial port
        #[arg(long)]
        port: String,
    },
}

fn run(port_name: String) {
    let baud_rate = 115200;

    println!("Connecting to {}", port_name);
    let mut port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(1000))
        .open()
        .expect("Failed to open port");

    let msg: Vec<u8> = vec![
        b'#', // SOF
        0x01, // Command: 0x01 = Version
        0x00, // Length
    ];
    let send_result = port.write_all(&msg);

    match send_result {
        Ok(()) => println!("Write successful"),
        Err(e) => eprintln!("Write failed: {}", e),
    }

    let mut buffer: Vec<u8> = vec![0; 32];
    match port.read(buffer.as_mut_slice()) {
        Ok(bytes_read) => {
            println!(
                "Received ({} bytes): {:?}",
                bytes_read,
                &buffer[..bytes_read]
            );
        },
        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
            println!("Timed out waiting for response");
        }
        Err(e) => {
            eprintln!("Error reading from serial port: {:?}", e);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Convert { input, output, format } => {
            println!("Converting '{}' to '{}' as format '{}'", input, output, format);
            // Add conversion logic here
        }
        Commands::Run { config, debug , port} => {
            println!("Running with config: {:?}", config);
            println!("Debug mode: {}", debug);
            run(port.to_string());
        }
    }
}
