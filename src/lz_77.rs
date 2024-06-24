use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use crate::bitbuffer::{self, BitBuffer};
use suffix_array::SuffixArray;
use indicatif::ParallelProgressIterator;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct LZ77 {
    pub bitbuffers: Vec<bitbuffer::BitBuffer>,
}

impl LZ77 {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&self).unwrap()
    }
    
    pub fn deserialize(input: &[u8]) -> Self {
        bincode::deserialize(input).unwrap()
    }

    #[inline]
    fn lpc(input: &[u8], i: usize, j: usize) -> usize {
        if i == 0 || j == 0 {
            return 0;
        }
        let mut k = 0;
        while j + k < input.len() && i + k < input.len() && input[i + k] == input[j + k] {
            k += 1;
        }
        k
    }

    pub fn fast_encode(input: &[u8]) -> BitBuffer {
        let n = input.len();
        
        let suffix_array = SuffixArray::new(input);        

        let (_,suffix_array) = suffix_array.into_parts();
        let suffix_array = suffix_array.into_iter().map(|i| i as usize).collect::<Vec<_>>();

        let mut inverse_suffix_array = vec![0; n+1];
        for (i, suffix_indx) in suffix_array.iter().enumerate() {
            inverse_suffix_array[*suffix_indx] = i;
        }

        let mut nsv = vec![0u32; n+1];
        let mut psv = vec![u32::MAX; n+1];
        for i in 1..n as u32 {
            let mut j = i - 1;
            while psv[j as usize] != u32::MAX && suffix_array[i as usize] < suffix_array[j as usize] {
                nsv[j as usize] = i;
                j = psv[j as usize];
            }
            psv[i as usize] = j;
        }
        psv = psv.into_iter().map(|i| if i == u32::MAX {0} else {i}).collect::<Vec<_>>();
        nsv = nsv.into_iter().map(|i| i).collect::<Vec<_>>();

        let mut factors = Vec::new();
        let mut k = 0;
        while k < n {
            let psv = suffix_array[psv[inverse_suffix_array[k] as usize] as usize];
            let nsv = suffix_array[nsv[inverse_suffix_array[k] as usize] as usize];
            let (p,l,c,indx) = LZ77::lz_factor(k, psv, nsv, input);
            k = indx;
            factors.push((p,l,c));
        } 

        let mut current_char_index = 0usize;
        let mut lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
        let mut max_lenght = 2usize.pow(lenght_size as u32) - 1;

        
        factors.into_iter().fold(BitBuffer::new(), | mut acc ,(mut p,mut l,c)| {
            if l == 0 {
                acc.write_bits(0, lenght_size);
                acc.write_byte(c);
                current_char_index += 1;
                lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                max_lenght = 2usize.pow(lenght_size as u32) - 1;
            } else if l < max_lenght as usize {
                let current_bits = 32 - (current_char_index as u32).leading_zeros() as u8;
                acc.write_bits(l as u32, lenght_size);
                acc.write_bits(p as u32, current_bits);
                current_char_index += l;
                lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                max_lenght = 2usize.pow(lenght_size as u32) - 1;
            } else {
                while l >= max_lenght as usize{
                    let current_bits = 32 - (current_char_index as u32).leading_zeros() as u8;
                    acc.write_bits(u32::MAX, lenght_size);
                    acc.write_bits(p as u32, current_bits);
                    p += max_lenght;
                    l -= max_lenght;
                    current_char_index += max_lenght;
                    lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                    max_lenght = 2usize.pow(lenght_size as u32) - 1;
                }
                if l != 0 {
                    let current_bits = 32 - (current_char_index as u32).leading_zeros() as u8;
                    acc.write_bits(l as u32, lenght_size);
                    acc.write_bits(p as u32, current_bits);
                    current_char_index += l;
                    lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                    max_lenght = 2usize.pow(lenght_size as u32) - 1;
                }
            }
            acc
        })
    }

    #[inline]
    fn lenght_size(bits: u8) -> u8 {
        (bits / 3).min(8).max(1)
    }

    fn decode_chunk(factors: Vec<(u32, u32, u8)>) -> Vec<u8> {
        factors.into_iter().fold(Vec::new(), |mut acc, (p,l,c)| {
            match l {
                0 => acc.push(c),
                _ => for i in 0..l {
                    acc.push(acc[p as usize + i as usize]);
                },
            }
            acc
        })
    }

    #[inline]
    fn lz_factor(i:usize, psv: usize, nsv: usize, x: &[u8]) -> (usize, usize, u8, usize) {
        let v1 = LZ77::lpc(x, i, psv);
        let v2 = LZ77::lpc(x, i, nsv);
        let (p,l) = if v1 > v2 {
            (psv, v1)
        } else {
            (nsv, v2)
        };
        if let Some(e) = x.get(i + l) {
            return (p, l, *e, i + l.max(1));
        }
        (p, l, 0, i + l)
    }

    pub fn encode(input: &[u8], bits: u8) -> LZ77 {
        let n = input.len();
        let chunk_size = 2usize.pow(bits as u32) - 1;
        let num_chunks = n / chunk_size + if n % chunk_size == 0 {0} else {1};

        let data = (0..num_chunks).into_par_iter() 
            .progress()
            .map(|i| {
                let start = i * chunk_size;
                let end = usize::min((i + 1) * chunk_size, n);
                let chunk = &input[start..end];
                let factors = LZ77::fast_encode(chunk);
                factors
            })
            .collect::<Vec<_>>();

        LZ77 {
            bitbuffers: data,
        }
    }

    pub fn decode(self) -> Vec<u8> {
        self.bitbuffers.into_par_iter().progress().flat_map(|mut chunk| {
            let mut current_char_index = 0usize;
            let mut factors = Vec::new();
            let mut current_bits;
            let mut lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
            while let Some(l) = chunk.read_bits(lenght_size) {
                match l {
                    0 => {
                        factors.push((0, 0, chunk.read_byte().unwrap()));
                        current_char_index += 1;
                        lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                    },
                    _ => {
                        current_bits =  32 - (current_char_index as u32).leading_zeros() as u8;
                        factors.push((chunk.read_bits(current_bits).unwrap(), l, 0));         
                        current_char_index += l as usize;     
                        lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                    },
                }
            }
            LZ77::decode_chunk(factors)
        }).collect::<Vec<_>>()
    }

}