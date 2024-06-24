use std::fs;

use archive::Archive;
use clap::Parser;
use huffman::Huffman;

mod archive;
mod lz77;
mod bitbuffer;
mod huffman;
mod terminal_interface;

fn main() {
    let args = terminal_interface::Args::parse();

    let lz_buffer_size = 25;

    if let Some(encrypt) = args.encrypt {
        let root = Archive::read_from_disk(&encrypt);
        let serialized = root.serialize();
        let lz_encoded = lz77::LZ77::encode(&serialized, lz_buffer_size);
        let huffman = Huffman::encrypt(&lz_encoded.serialize());
        let full_path = fs::canonicalize(encrypt).unwrap();
        let dir_name = full_path.file_name().unwrap().to_str().unwrap();
        fs::write(format!("{}.tmy",dir_name), huffman.serialize()).unwrap();
    }
    if let Some(decrypt) = args.decrypt {
        let huffman_serialized = fs::read(decrypt).unwrap();
        let huffman = Huffman::deserialize(&huffman_serialized);
        let mut lz_encoded = lz77::LZ77::deserialize(&huffman.decrypt());
        let root = Archive::deserialize(&lz_encoded.decode(lz_buffer_size));
        root.write_to_disk(".");
    }

}
