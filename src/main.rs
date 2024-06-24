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

    let lz_buffer_size = 28;
    let huffman_bits = 20;

    if let Some(encrypt) = args.encrypt {
        let root = Archive::read_from_disk(&encrypt);
        let serialized = root.serialize();
        let lz_encoded = LZ77::encode(&serialized, lz_buffer_size);
        let huffman = ParrallelHuffman::encrypt(&lz_encoded.serialize(), huffman_bits);
        let full_path = fs::canonicalize(encrypt).unwrap();
        let dir_name = full_path.file_name().unwrap().to_str().unwrap();
        fs::write(format!("{}.tmy",dir_name), huffman.serialize()).unwrap();
    } else if let Some(decrypt) = args.decrypt {
        let huffman_serialized = fs::read(decrypt).unwrap();
        let huffman = ParrallelHuffman::deserialize(&huffman_serialized);
        let lz_encoded = LZ77::deserialize(&huffman.decrypt());
        let root = Archive::deserialize(&lz_encoded.decode());
        root.write_to_disk(".");
    } else if let Some(benchmark) = args.benchmark{
        println!("Starting benchmark with LZ77 chunk size {}MB and huffman chunk size {}KB", 2u32.pow(lz_buffer_size as u32 - 20) - 1, 2u32.pow(huffman_bits as u32 - 10));
        let root = Archive::read_from_disk(&benchmark);
        let serialized = root.serialize();
        if serialized.len() >= 2usize.pow(20) {
            println!("Read archive of size {}MB", serialized.len() / 2usize.pow(20));
        } else {
            println!("Read archive of size {}KB", serialized.len() / 2usize.pow(10));
        }

        let start = std::time::Instant::now();
        let lz_encoded = LZ77::encode(&serialized, lz_buffer_size).serialize();
        let lz_time = start.elapsed();

        let huffman = ParrallelHuffman::encrypt(&lz_encoded, huffman_bits);
        let huffman_time = start.elapsed() - lz_time;

        let compressed = huffman.serialize();
        if compressed.len() >= 2usize.pow(20) {
            println!("Compression finished! Compressed archive to {}MB", compressed.len() / 2usize.pow(20));
        } else {
            println!("Compression finished! Compressed archive to {}KB", compressed.len() / 2usize.pow(10));
        }

        let lz = ParrallelHuffman::decrypt(&huffman);
        let huffman_time_decrypt = start.elapsed() - lz_time - huffman_time;

        let lz_encoded = LZ77::deserialize(&lz);
        let decoded = lz_encoded.decode();
        let lz_time_decrypt = start.elapsed() - lz_time - huffman_time - huffman_time_decrypt;
        println!("Decompression finished!");
        
        assert_eq!(root, Archive::deserialize(&decoded), "Decoded archive does not match original");
        println!("Benchmark finished successfully!");
        println!("LZ Encrypt:      {:?}\nHuffman Encrypt: {:?}\nHuffman Decrypt: {:?}\nLZ Decrypt:      {:?}",lz_time, huffman_time, huffman_time_decrypt, lz_time_decrypt);
        println!("Compression ratio: {}", compressed.len() as f64 / serialized.len() as f64);
    }

}