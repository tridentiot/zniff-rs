mod frame_definition;
mod xml;
use core::error;
use std::time::Duration;
use std::io::{self, Write, Read};

use clap::{
    Parser,
    Subcommand,
    ValueEnum,
    value_parser,
};
use serialport::SerialPort;

mod proxy;
use crate::proxy::{
    ProxyProtocol,
    Proxy
};

use crate::types::Frame;
mod zw_parser;
use zw_parser::ZwParser;
mod zlf;

use crate::zniffer_parser::ParserResult;

mod types;
mod zniffer_parser;

mod generator;
use crate::generator::FrameGenerator;

use tokio::{
    io::{
        AsyncReadExt,
    },
    net::{
        TcpListener,
        TcpStream,
    },
    sync::broadcast
};

mod client_handler;
use crate::client_handler::handle_client;

mod zn_client;
use crate::zn_client::ZnClient;

#[derive(Parser)]
#[command(name = "zniff-rs")]
#[command(about = "zniff-rs is a tool for sniffing, parsing and converting Z-Wave data.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate frames
    Generator {
        /// From file
        #[arg(long)]
        from_file: String,
        /// Delay in milliseconds between frames
        #[arg(long)]
        delay: u16,
    },

    /// Connect to a device, generator, server or proxy.
    Client {
        /// Address to connect to
        #[arg(long)]
        address: Vec<String>,

        /// Serial port to connect to
        #[arg(short = 's', long = "serial")]
        serial: Vec<String>,
    },

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

    /// Runs a PTI server that listens for Zniffer frames and serves them over TCP.
    Run {
        /// Configuration file
        #[arg(short, long)]
        config: Option<String>,

        /// Enable debug mode
        #[arg(long)]
        debug: bool,

        /// Serial port
        #[arg(short = 's', long = "serial")]
        serial: Vec<String>,

        /// Z-Wave region
        #[arg(long, value_enum, required = true)]
        region: Region,
    },

    /// Parses a Z-Wave frame from a string input.
    Parse {
        /// String representing the Z-Wave frame
        #[arg(long)]
        input: String,
    },

    /// Runs a Zniffer TCP to PTI or Zniffer TCP proxy.
    Proxy {
        /// Protocol to use by the client connecting to the proxy (PTI or zniff-rs)
        /// The PTI protocol can be used by the PC Zniffer application.
        /// The zniff-rs protocol can be used by other zniff-rs instances.
        #[arg(long, value_parser = value_parser!(ProxyProtocol))]
        protocol: ProxyProtocol,

        /// Address to bind to
        #[arg(long)]
        address: String,
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
        buffer[0..response_length].to_vec()
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

async fn run(serial_ports: &Vec<String>, region: &Region) {
    let baud_rate = 230_400;

    if serial_ports.is_empty() {
        eprintln!("No serial ports provided. Use --serial to specify at least one port.");
        return;
    }

    if serial_ports.len() > 1 {
        println!("Multiple serial ports provided:");
        for (i, port) in serial_ports.iter().enumerate() {
            println!("Serial[{}]: {}", i, port);
        }
    }

    println!("Connecting to the first one: {}", serial_ports[0]);
    let port: Box<dyn SerialPort> = serialport::new(&serial_ports[0], baud_rate)
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

    let (tx, _rx) = broadcast::channel(16);

    let tx_clone = tx.clone();
    // Spawn a blocking task to read frames since the serial port read is blocking (not async).
    tokio::task::spawn_blocking(move || {
        loop {
            match zniffer.get_frames() {
                Ok(frame) => {
                    println!("Received frame: {:?}", frame);
                    match tx_clone.send(frame) {
                        Ok(_) => {
                            // Successfully sent
                        },
                        Err(e) => {
                            println!("Failed to send frame to channel: {:?}", e);
                            return;
                        }
                    }
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

    // PC Zniffer PTI default port is 4905
    let listener = TcpListener::bind("0.0.0.0:4905").await.unwrap();
    println!("Server listening on port 4905");

    loop {
        let (stream, addr) = listener.accept().await.unwrap();
        println!("Client connected: {addr}");

        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            handle_client(stream, &mut rx).await;
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Generator { from_file, delay } => {
            let generator: FrameGenerator = FrameGenerator::new(from_file.to_string(), *delay)?;

            generator.run()?;

            Ok(())
        },
        Commands::Client { address, serial } => {
            if address.is_empty() && serial.is_empty() {
                eprintln!("No address or serial ports provided. Use --address or --serial to specify at least one.");

                // Return an error so the exit code becomes non-zero
                return Err("No address or serial ports provided".into());
            }

            let client = match ZnClient::try_new(&serial, &address) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to create ZnClient: {:?}", e);
                    return Err("Failed to create ZnClient".into());
                }
            };

            client.run().await?;
/*
            let addr = format!("{}:9000", address[0]);
            let mut stream = TcpStream::connect(&addr).await.unwrap();
            println!("Connected to {addr}");

            let mut buf = [0u8; 8192];
            let mut total = 0usize;

            loop {
                let n: usize = stream.read(&mut buf).await.unwrap();
                if n == 0 {
                    println!("Server closed connection. Total bytes: {total}");
                    break;
                }
                total += n;
                // Print as hex (first 32 bytes for brevity)
                let preview = &buf[..n];
                    print!("recv {n} bytes: ");
                for b in preview {
                    print!("{:02X} ", b);
                }
                println!();
            }
 */
            Ok(())
        },
        Commands::Convert { input, output, format } => {
            println!("Converting '{}' to '{}' as format '{}'", input, output, format);
            // Add conversion logic here
            Ok(())
        }
        Commands::Run { config, debug , serial, region} => {
            println!("Running with config: {:?}", config);
            println!("Debug mode: {}", debug);
            // Add run logic here
            println!("Region: {:?}", region);
            run(serial, region).await;
            Ok(())
        },
        Commands::Parse { input } => {
            let fd = frame_definition::parse_xml();
            let zwc = xml::parse_xml();
            let zw_parser: ZwParser = ZwParser::new(&fd, &zwc);
            zw_parser.parse_str(&input);
            Ok(())
        },
        Commands::Proxy { protocol, address } => {
            let proxy = Proxy::new(address.to_string(), protocol.clone());
            proxy.run().await?;
            Ok(())
        }
    }
}
