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
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Convert { input, output, format } => {
            println!("Converting '{}' to '{}' as format '{}'", input, output, format);
            // Add conversion logic here
        }
        Commands::Run { config, debug } => {
            println!("Running with config: {:?}", config);
            println!("Debug mode: {}", debug);
            // Add run logic here
        }
    }
}
