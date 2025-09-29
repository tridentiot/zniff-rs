use std::time::Duration;
use std::io::{self, Write, Read};

use clap::{Parser, Subcommand, ValueEnum};

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

        /// Z-Wave region
        #[arg(long, value_enum)]
        region: Region,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Region {
    EU = 0,
    US = 1,
    ANZ = 2,
    HK = 3,
    IN = 5,
    IL = 6,
    RU = 7,
    CN = 8,
    USLR = 9,
    EULR = 11,
    JP = 32,
    KR = 33,
}

fn run(port_name: String, _region: &Region) {
    let baud_rate = 230_400;

    println!("Connecting to {}", port_name);
    let mut port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(1000))
        .open()
        .expect("Failed to open port");

    /*
    let get_version: Vec<u8> = vec![
        0x23, // Command SOF
        0x03, // Command: 0x01 = Version
        0x00, // Length
    ];
    */

    let msg: Vec<u8> = vec![
        0x23, // SOF
        0x01, // Command: 0x01 = Version
        0x00, // Length
    ];
    let send_result = port.write_all(&msg);

    match send_result {
        Ok(()) => println!("Write successful"),
        Err(e) => eprintln!("Write failed: {}", e),
    }

    let mut buffer: Vec<u8> = vec![0; 128];
    loop {
        match port.read(buffer.as_mut_slice()) {
            Ok(bytes_read) => {
                println!("Received {:?} bytes", bytes_read);
                for byte in &buffer[..bytes_read] {
                    print!("0x{:02X} ", byte);
                }
                println!();

                /*
                println!(
                    "Received ({} bytes): {:x}",
                    bytes_read,
                    &buffer[..bytes_read]
                );
                */
            },
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                println!("Timed out waiting for response");
            }
            Err(e) => {
                eprintln!("Error reading from serial port: {:?}", e);
                break;
            }
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
        Commands::Run { config, debug , port, region} => {
            println!("Running with config: {:?}", config);
            println!("Debug mode: {}", debug);
            println!("Region: {:?}", region);
            run(port.to_string(), region);
        }
    }
}
