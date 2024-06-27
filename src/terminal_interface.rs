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

    /// Benchmark the folder at the given path
    #[arg(short, long)]
    pub benchmark: Option<String>,

    /// The size of the LZ77 buffer (8-31) 
    #[arg(short, long, default_value = "28")]
    pub lz_buffer: u32,

    /// The size of the Huffman buffer (8-31)
    #[arg(long, default_value = "20")]
    pub huffman_buffer: u32,
}