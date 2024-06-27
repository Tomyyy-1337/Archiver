use std::fs;
use clap::Parser;

mod archive;
mod lz_77;
mod huffman;
mod bitbuffer;
mod terminal_interface;

use huffman::ParrallelHuffman;
use archive::Archive;
use lz_77::LZ77;

fn main() {
    let args = terminal_interface::Args::parse();

    let lz_buffer_size = args.lz_buffer as u8;
    let huffman_bits = args.huffman_buffer as u8;

    if let Some(path) = args.encrypt {
        compress(&path, lz_buffer_size, huffman_bits);
    } else if let Some(path) = args.decrypt {
        decompress(&path);
    } else if let Some(path) = args.benchmark{
        benchmark(&path, lz_buffer_size, huffman_bits);
    }
}

fn compress(path: &str, lz_buffer_size: u8, huffman_bits: u8) {
    let root = Archive::read_from_disk(&path);
    let serialized = root.serialize();
    if serialized.len() >= 2usize.pow(20) {
        println!("Read archive of size {}MB", serialized.len() / 2usize.pow(20));
    } else {
        println!("Read archive of size {}KB", serialized.len() / 2usize.pow(10));
    }
    let mut lz_encoded = LZ77::encode(&serialized, lz_buffer_size).serialize();
    let mut huffman = ParrallelHuffman::encrypt(&lz_encoded, huffman_bits).serialize();
    let full_path = fs::canonicalize(path).unwrap();
    let dir_name = full_path.file_name().unwrap().to_str().unwrap();

    let compressed = if lz_encoded.len() <= huffman.len() {
        lz_encoded.insert(0, 0);
        lz_encoded
    } else {
        huffman.insert(0, 1);
        huffman
    };
    if compressed.len() >= 2usize.pow(20) {
        println!("Compressed archive to {}MB.", compressed.len() / 2usize.pow(20));
    } else {
        println!("Compressed archive to {}KB.", compressed.len() / 2usize.pow(10));
    }
    fs::write(format!("{}.tmy",dir_name), compressed).unwrap();
}

fn decompress(path: &str) {
    let contents = fs::read(path).unwrap();
    if contents.len() < 2usize.pow(20) {
        println!("Read archive of size {}KB", contents.len() / 2usize.pow(10));
    } else {
        println!("Read archive of size {}MB", contents.len() / 2usize.pow(20));
    }
    let root = if contents[0] == 0 {
        let lz_encoded = &contents[1..];
        let lz_encoded = LZ77::deserialize(&lz_encoded);
        Archive::deserialize(&lz_encoded.decode())
    } else {
        let huffman_serialized = &contents[1..];
        let huffman = ParrallelHuffman::deserialize(&huffman_serialized);
        let lz_encoded = LZ77::deserialize(&huffman.decrypt());
        Archive::deserialize(&lz_encoded.decode())
    };
    root.write_to_disk(".");
    println!("Decompressed archive successfully!");
}

fn benchmark(path: &str, lz_buffer_size: u8, huffman_bits: u8) {
    println!("Starting benchmark with LZ77 chunk size {:2}MB and huffman chunk size {}KB", 2f32.powi(lz_buffer_size as i32 - 20), 2u32.pow(huffman_bits as u32 - 10));
    let root = Archive::read_from_disk(&path);
    let serialized = root.serialize();
    if serialized.len() >= 2usize.pow(20) {
        println!("Read archive of size {}MB", serialized.len() / 2usize.pow(20));
    } else {
        println!("Read archive of size {}KB", serialized.len() / 2usize.pow(10));
    }

    println!("Testing Compression...");
    
    let start = std::time::Instant::now();

    let lz_encoded = LZ77::encode(&serialized, lz_buffer_size).serialize();
    let lz_time = std::time::Instant::now();

    let huffman = ParrallelHuffman::encrypt(&lz_encoded, huffman_bits).serialize();
    let lz_huffman_time = std::time::Instant::now();

    let compressed = if lz_encoded.len() <= huffman.len() {
        println!("Compression mode: LZ77 only.");
        &lz_encoded
    } else {
        println!("Compression mode: LZ77 + Huffman.");
        &huffman
    };

    if compressed.len() >= 2usize.pow(20) {
        println!("Compressed archive to {}MB.", compressed.len() / 2usize.pow(20));
    } else {
        println!("Compressed archive to {}KB.", compressed.len() / 2usize.pow(10));
    }

    println!("Testing Decompression...");
    let start_decompress = std::time::Instant::now();
    
    let lz = ParrallelHuffman::decrypt(&ParrallelHuffman::deserialize(&huffman));
    let huffman_time_decode = std::time::Instant::now();
    let decoded = LZ77::deserialize(&lz).decode();
    let lz_time_decode = std::time::Instant::now();

    assert_eq!(lz, lz_encoded, "Decoded LZ77 does not match original LZ77");
    assert_eq!(root, Archive::deserialize(&decoded), "Decoded archive does not match original"); 

    println!("Benchmark finished successfully!");
    println!("LZ77    Compression      : {:?}", lz_time.duration_since(start));
    println!("Huffman Compression      : {:?}", lz_huffman_time.duration_since(lz_time));
    println!("Huffman Decompression    : {:?}", huffman_time_decode.duration_since(start_decompress));
    println!("LZ77    Decompression    : {:?}", lz_time_decode.duration_since(huffman_time_decode));
    println!("Compression Ratio : {:.2}%", 100.0 * (compressed.len() as f32 / serialized.len() as f32));
}