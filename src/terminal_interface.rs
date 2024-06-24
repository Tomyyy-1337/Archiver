use clap::Parser;

/// Folder Archiver and Compression Tool
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Encrypt the folder at the given path
    #[arg(short, long)]
    pub encrypt: Option<String>,

    /// Decrypt the file at the given path 
    #[arg(short, long)]
    pub decrypt: Option<String>,
}