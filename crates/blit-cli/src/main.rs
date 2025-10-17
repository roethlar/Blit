use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "blit")]
#[command(about = "A fast, AI-built file transfer tool (v2)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Push files to a remote server
    Push { source: String, destination: String },
    /// Pull files from a remote server
    Pull { source: String, destination: String },
    /// Mirror a directory to a remote server
    Mirror { source: String, destination: String },
    /// List contents of a remote directory
    Ls { path: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Push {
            source,
            destination,
        } => {
            println!("Pushing from {} to {}", source, destination);
            // To be implemented in Phase 2
        }
        Commands::Pull {
            source,
            destination,
        } => {
            println!("Pulling from {} to {}", source, destination);
            // To be implemented in Phase 3
        }
        Commands::Mirror {
            source,
            destination,
        } => {
            println!("Mirroring from {} to {}", source, destination);
            // To be implemented in Phase 2
        }
        Commands::Ls { path } => {
            println!("Listing contents of {}", path);
            // To be implemented in Phase 3
        }
    }

    Ok(())
}
