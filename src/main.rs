use std::time::Duration;
use std::io::{self, Write, Read};

use clap::{Parser, Subcommand, ValueEnum};
use serialport::SerialPort;

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

struct Zniffer {
    port: Box<dyn SerialPort>,
    region: Region,
}

impl Zniffer {
    fn new(port: Box<dyn SerialPort>, region: Region) -> Self {
        Zniffer { port, region }
    }

    fn get_version(&mut self) -> Vec<u8> {
        let msg: Vec<u8> = vec![
            0x23, // SOF
            0x01, // Command: 0x01 = Version
            0x00, // Length
        ];
        let send_result = self.port.write_all(&msg);

        match send_result {
            Ok(()) => println!("Write successful"),
            Err(e) => eprintln!("Write failed: {}", e),
        }

        let mut buffer: Vec<u8> = vec![0; 128];
        let mut response_length: usize = 0;
        loop {
            match self.port.read(buffer.as_mut_slice()) {
                Ok(bytes_read) => {
                    println!("Received {:?} bytes", bytes_read);
                    for byte in &buffer[..bytes_read] {
                        print!("0x{:02X} ", byte);
                    }
                    println!();
                    response_length = bytes_read;
                    // TODO: Add frame parsing so we can exit when a valid frame is received.
                },
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                    // TODO: Remove print when timing out.
                    println!("Timed out waiting for response");
                    break;
                }
                Err(e) => {
                    eprintln!("Error reading from serial port: {:?}", e);
                    break;
                }
            }
        }
        return buffer[0..response_length].to_vec();
    }

    fn set_region(&mut self) {
        let msg: Vec<u8> = vec![
            0x23, // SOF
            0x02, // Set region
            0x01, // Length
            self.region as u8,
        ];
        let send_result = self.port.write_all(&msg);

        match send_result {
            Ok(()) => println!("Write successful"),
            Err(e) => eprintln!("Write failed: {}", e),
        }
    }

    fn start(&mut self) -> Vec<u8> {
        let msg: Vec<u8> = vec![
            0x23, // SOF
            0x04, // Start
            0x00, // Length
        ];
        let send_result = self.port.write_all(&msg);

        match send_result {
            Ok(()) => println!("Write successful"),
            Err(e) => eprintln!("Write failed: {}", e),
        }

        let mut buffer: Vec<u8> = vec![0; 128];
        let mut response_length: usize = 0;
        loop {
            match self.port.read(buffer.as_mut_slice()) {
                Ok(bytes_read) => {
                    println!("Received {:?} bytes", bytes_read);
                    for byte in &buffer[..bytes_read] {
                        print!("0x{:02X} ", byte);
                    }
                    println!();
                    response_length = bytes_read;
                    // TODO: Add frame parsing so we can exit when a valid frame is received.
                },
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                    println!("Timed out waiting for response");
                    break;
                }
                Err(e) => {
                    eprintln!("Error reading from serial port: {:?}", e);
                    break;
                }
            }
        }
        return buffer[0..response_length].to_vec();
    }

    fn get_frames(&mut self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![0; 128];
        let mut response_length: usize = 0;
        loop {
            match self.port.read(buffer.as_mut_slice()) {
                Ok(bytes_read) => {
                    println!("Received {:?} bytes", bytes_read);
                    for byte in &buffer[..bytes_read] {
                        print!("0x{:02X} ", byte);
                    }
                    println!();
                    response_length = bytes_read;
                    // TODO: Add frame parsing so we can exit when a valid frame is received.
                },
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                    println!("Timed out waiting for response");
                    break;
                }
                Err(e) => {
                    eprintln!("Error reading from serial port: {:?}", e);
                    break;
                }
            }
        }
        return buffer[0..response_length].to_vec();
    }
}

fn print_hex(vec: &Vec<u8>) {
    for byte in vec {
        print!("0x{:02X} ", byte);
    }
    println!(); // newline at the end
}


fn run(port_name: String, region: &Region) {
    let baud_rate = 230_400;

    println!("Connecting to {}", port_name);
    let port = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(500))
        .open()
        .expect("Failed to open port");

    let mut zniffer = Zniffer::new(port, *region);

    let version: Vec<u8> = zniffer.get_version();
    print_hex(&version);

    zniffer.set_region();

    let _response = zniffer.start();

    loop {
        let frame: Vec<u8> = zniffer.get_frames();
        print_hex(&frame);
    }

    /*
    let get_version: Vec<u8> = vec![
        0x23, // Command SOF
        0x03, // Command: 0x01 = Version
        0x00, // Length
    ];
    */

    // let msg: Vec<u8> = vec![
    //     0x23, // SOF
    //     0x01, // Command: 0x01 = Version
    //     0x00, // Length
    // ];
    // let send_result = port.write_all(&msg);

    // match send_result {
    //     Ok(()) => println!("Write successful"),
    //     Err(e) => eprintln!("Write failed: {}", e),
    // }

    // let mut buffer: Vec<u8> = vec![0; 128];
    // loop {
    //     match port.read(buffer.as_mut_slice()) {
    //         Ok(bytes_read) => {
    //             println!("Received {:?} bytes", bytes_read);
    //             for byte in &buffer[..bytes_read] {
    //                 print!("0x{:02X} ", byte);
    //             }
    //             println!();

    //             /*
    //             println!(
    //                 "Received ({} bytes): {:x}",
    //                 bytes_read,
    //                 &buffer[..bytes_read]
    //             );
    //             */
    //         },
    //         Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
    //             println!("Timed out waiting for response");
    //         }
    //         Err(e) => {
    //             eprintln!("Error reading from serial port: {:?}", e);
    //             break;
    //         }
    //     }
    // }
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
