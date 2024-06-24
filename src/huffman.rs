use std::cmp::Reverse;

use indicatif::ParallelProgressIterator;
use priority_queue::PriorityQueue;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Serialize, Deserialize};

use crate::bitbuffer;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ParrallelHuffman {
    chunks: Vec<Huffman>,
}

impl ParrallelHuffman {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    pub fn deserialize(input: &[u8]) -> ParrallelHuffman {
        bincode::deserialize(input).unwrap()
    }

    pub fn encrypt(input: &Vec<u8>, bits: u8) -> ParrallelHuffman {
        let chunk_size = 2usize.pow(bits as u32) - 1;
        let chunks = input.chunks(chunk_size)
            .collect::<Vec<_>>()
            .par_iter()
            .progress()
            .map(|chunk| Huffman::encrypt(&chunk.to_vec()))
            .collect::<Vec<_>>();
        ParrallelHuffman { chunks }
    }

    pub fn decrypt(&self) -> Vec<u8> {
        self.chunks
            .par_iter()
            .progress()
            .flat_map(|chunk| chunk.decrypt())
            .collect()
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Huffman {
    tree: Vec<u8>,
    unused_bits: u8,
    pub data: Vec<u8>,
}

impl Huffman {
    pub fn encrypt(input: &Vec<u8>) -> Huffman {
        let tree = HuffmanTree::build_tree(&input);
        let mut lookup = (0..256).map(|_| Vec::new()).collect::<Vec<_>>();
        tree.build_map(vec![], &mut lookup);

        let mut lookup = (0..256).map(|_| Vec::new()).collect::<Vec<_>>();
        tree.build_map(vec![], &mut lookup);

        let (count, data) = input
            .into_iter()
            .flat_map(|&c| &lookup[c as usize])
            .fold((0usize,Vec::new()), |(indx, mut acc), c|{
                if indx % 8 == 0 {
                    acc.push(if *c {1u8} else {0u8});
                } else if *c {
                    *acc.last_mut().unwrap() |= 1 << (indx % 8);
                }
                (indx + 1, acc)
            });

        Huffman {
            tree: tree.better_serialize(),
            unused_bits: match count % 8 {
                0 => 0,
                n => 8 - n as u8,
            },
            data,
        }
    }

    pub fn decrypt(&self) -> Vec<u8> {
        let tree = HuffmanTree::better_deserialize(&self.tree);
        let data = &self.data;
        let unused = self.unused_bits;
        let mut result = Vec::new();
        let mut input = Vec::new();
        let map = tree.build_reverse_map();
        for i in 0..data.len() * 8 - unused as usize {
            let indx = i / 8;
            let bit = (i % 8) as u8;
            input.push(data[indx] & (1 << bit) != 0);
            if let Some(c) = map[std::iter::once(&true).chain(input.iter()).fold(0usize, |acc, &f| (acc << 1) | if f {1} else {0})] {
                result.push(c);
                input.clear();
            }
        }
        result
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct HuffmanTree {
    pub children: Vec<HuffmanTree>,
    pub character: Option<u8>,
}

impl HuffmanTree {
    pub fn better_serialize(&self) -> Vec<u8> {
        let mut bitbuffer = bitbuffer::BitBuffer::new();
        self.beter_serialize_rec(&mut bitbuffer);
        bitbuffer.serialize()
    }

    fn beter_serialize_rec(&self, bitbuffer: &mut bitbuffer::BitBuffer) {
        match self.character {
            Some(c) => {
                bitbuffer.write_bit(true);
                bitbuffer.write_byte(c);
            }
            None => {
                bitbuffer.write_bit(false);
                self.children[0].beter_serialize_rec(bitbuffer);
                self.children[1].beter_serialize_rec(bitbuffer);
            }
        }
    }

    pub fn better_deserialize(input: &[u8]) -> Self {
        let mut bitbuffer = bitbuffer::BitBuffer::deserialize(input);
        Self::better_deserialize_rec(&mut bitbuffer)
    }

    fn better_deserialize_rec(bitbuffer: &mut bitbuffer::BitBuffer) -> Self {
        if let Some(true) = bitbuffer.read_bit() {
            Self {
                children: vec![],
                character: bitbuffer.read_byte(),
            }
        } else {
            Self {
                children: vec![
                    Self::better_deserialize_rec(bitbuffer),
                    Self::better_deserialize_rec(bitbuffer),
                ],
                character: None,
            }
        }
    }

    pub fn from_counts(counts: [u64;256]) -> HuffmanTree {
        let mut pq: PriorityQueue<Self, _, _> = PriorityQueue::new();
        pq.extend(counts.into_iter().enumerate().map(|(c, count)| (Self {
            children: vec![],
            character: Some(c as u8),
        }, Reverse(count))));

        while pq.len() > 1 {
            let (left, count_left) = pq.pop().unwrap();
            let (right, count_right) = pq.pop().unwrap();
            pq.push(Self {
                children: vec![left, right],
                character: None,
            }, Reverse(count_left.0 + count_right.0));
        }
        pq.pop().unwrap().0
    }

    pub fn build_tree(input: &Vec<u8>) -> HuffmanTree {
        let mut counts = [0u64;256];

        for &e in input {
            counts[e as usize] += 1;
        }

        Self::from_counts(counts)
    }

    fn build_reverse_map(&self) -> Vec<Option<u8>> {
        let mut map = (0..256).map(|_| Vec::new()).collect::<Vec<_>>();
        self.build_map(Vec::new(), &mut map);

        let max_len = map.iter().map(|v| v.len()).max().unwrap();
        let mut result = vec![None; 2usize.pow(max_len as u32 + 1)];
        for (c, path) in map.into_iter().enumerate() {
            let indx = std::iter::once(true).chain(path.into_iter()).fold(0usize, |acc, b| (acc << 1) | if b {1} else {0});
            result[indx] = Some(c as u8);
        }

        result
    }

    fn build_map(&self, current_path: Vec<bool>, map: &mut Vec<Vec<bool>>) {
        match self.character {
            Some(c) => {
                map[c as usize] = current_path;
            }
            None => {
                self.children[0].build_map({
                    let mut path = current_path.clone();
                    path.push(false);
                    path
                }, map);
                self.children[1].build_map({
                    let mut path = current_path.clone();
                    path.push(true);
                    path
                }, map);
            }
        }
    }
}