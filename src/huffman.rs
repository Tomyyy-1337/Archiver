use std::cmp::Reverse;

use priority_queue::PriorityQueue;
use serde::{Serialize, Deserialize};

use crate::bitbuffer;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Huffman {
    tree: Vec<u8>,
    unused_bits: u8,
    pub data: Vec<u8>,
}

impl Huffman {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }

    pub fn deserialize(input: &[u8]) -> Huffman {
        bincode::deserialize(input).unwrap()
    }

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
        for i in 0..data.len() * 8 - unused as usize {
            let indx = i / 8;
            let bit = (i % 8) as u8;
            input.push(data[indx] & (1 << bit) != 0);
            if let Some(char) = tree.decrypt_char(&input) {
                result.push(char);
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

    fn decrypt_char(&self, code: &[bool]) -> Option<u8> {
        match code.split_first() {
            Some((first, rest)) => {
                if *first {
                    self.children.get(1).and_then(|child| child.decrypt_char(rest))
                } else {
                    self.children.get(0).and_then(|child| child.decrypt_char(rest))
                }
            }
            None => self.character,
        }
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