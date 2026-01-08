mod xml;
use xml::parse_xml;
use xml::ZwClasses;
use std::time::Duration;
use std::io::{self, Write, Read};
use std::net::TcpListener;

use clap::{Parser, Subcommand, ValueEnum};
use serialport::SerialPort;

use crate::types::Frame;
use crate::zniffer_parser::ParserResult;

use std::sync::mpsc;
use std::thread;

mod types;
mod zniffer_parser;

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
    parser: zniffer_parser::Parser,
}

impl Zniffer {
    fn new(port: Box<dyn SerialPort>, region: Region) -> Self {
        Zniffer {
            port,
            region,
            parser: zniffer_parser::Parser::new()
        }
    }

    fn get_version(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let msg: Vec<u8> = vec![
            0x23, // SOF
            0x01, // Command: 0x01 = Version
            0x00, // Length
        ];

        self.port.write_all(&msg)?;

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
                    match self.parser.parse(buffer.clone()) {
                        ParserResult::ValidCommand { id } => {
                            println!("Got command ID {:?}", id);
                        },
                        ParserResult::ValidFrame { frame: _ } => {
                            // This should not happen since we're expecting a response to Get Version.
                        },
                        ParserResult::IncompleteFrame => {
                            // Continue parsing.
                        },
                    }
                },
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                    self.parser.timeout();
                    break;
                }
                Err(e) => {
                    eprintln!("Error reading from serial port: {:?}", e);
                    break;
                }
            }
        }
        Ok(buffer[0..response_length].to_vec())
    }

    fn set_region(&mut self) -> Result<(), std::io::Error>  {
        let msg: Vec<u8> = vec![
            0x23, // SOF
            0x02, // Set region
            0x01, // Length
            self.region as u8,
        ];
        self.port.write_all(&msg)?;
        Ok(())
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
                    //println!("Received {:?} bytes", bytes_read);
                    //for byte in &buffer[..bytes_read] {
                    //    print!("0x{:02X} ", byte);
                    //}
                    //println!();
                    response_length = bytes_read;
                    print_hex(&buffer[0..response_length].to_vec());
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

    fn get_frames(&mut self) -> Result<Frame, bool> {
        let mut buffer: Vec<u8> = vec![0; 128];
        loop {
            // TODO: Do we need to read data from the serial port into a ring buffer to avoid
            // dropping frame 2 of 2 that might have been read from the serial port at once?
            match self.port.read(buffer.as_mut_slice()) {
                Ok(bytes_read) => {
                    match self.parser.parse(buffer[..bytes_read].to_vec()) {
                        ParserResult::ValidCommand { id: _ } => {
                            // This should not happen as we do not expect
                            // unsolicited commands from the zniffer device.
                        },
                        ParserResult::ValidFrame { frame } => {
                            return Ok(frame);
                        },
                        ParserResult::IncompleteFrame => {
                            // Continue parsing.
                        },
                    }
                },
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                    self.parser.timeout();
                }
                Err(e) => {
                    eprintln!("Error reading from serial port: {:?}", e);
                    return Err(false);
                }
            }
        }
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

    match zniffer.get_version() {
        Ok(version) => {
            println!("Got version:");
            print_hex(&version);
        },
        Err(e) => {
            eprintln!("Failed to get the version: {:?}", e);
        }
    }

    match zniffer.set_region() {
        Ok(()) => {
            // Don't do anything.
        },
        Err(e) => {
            eprintln!("Failed to set the region: {:?}", e);
        }
    }

    let _ = zniffer.start();

    // PC Zniffer PTI default port is 4905
    let listener = TcpListener::bind("0.0.0.0:4905").unwrap();
    println!("Server listening on port 4905");

    // We might want to use "let (tx, _) = broadcast::channel(100);" to support multiple receivers.
    let (tx, rx) = mpsc::channel::<Frame>();

    let parser_thread_handle = thread::spawn(move || {
        loop {
            match zniffer.get_frames() {
                Ok(frame) => {
                    tx.send(frame).unwrap();
                },
                Err(true) => {
                    panic!("Should never happen!");
                },
                Err(false) => {
                    println!("Failed to get frame!");
                    return;
                }
            }
        }
    });


    if let Ok((mut stream, addr)) = listener.accept() {
        println!("Connection received from {}", addr);
        let process_thread_handle: thread::JoinHandle<()> = thread::spawn(move ||
        {
            for mut frame in rx {
                println!("rx:{:?}", frame);

                match frame.to_pti_vector()
                {
                    Ok(pty_frame) =>  {

                        let hex_string: String = pty_frame.iter()
                                .map(|byte| format!("{:02X}", byte))
                                .collect::<Vec<String>>()
                                .join(" ");

                        println!("tx pti:{:?}", hex_string);
                        stream.write_all(&pty_frame).unwrap();
                    }
                    Err(_e) => {
                        println!("Failed to form PTI packet");
                    }
                }
            }
        });
        process_thread_handle.join().unwrap();
    }

    parser_thread_handle.join().unwrap();
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
            // Add run logic here
            let zw: ZwClasses = parse_xml();
            for class in zw.cmd_class {
                println!("{:?}", class.key);
            }
            println!("Region: {:?}", region);
            run(port.to_string(), region);
        }
    }
}
