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
    fn lpc(input: &[u8], i: u32, j: u32) -> u32 {
        input[i as usize..]
            .iter()
            .zip(input[j as usize..].iter())
            .position(|(a,b)| a != b)
            .unwrap_or(0) as u32
    }

    pub fn fast_encode(input: &[u8]) -> BitBuffer {
        let n = input.len();

        let (_,suffix_array) = SuffixArray::new(input).into_parts();

        let mut nsv = Vec::new();
        let mut psv = Vec::new();
        let mut inverse_suffix_array = Vec::new();

        std::thread::scope(|scope| {
            scope.spawn(|| {
                inverse_suffix_array = vec![0; n+1];
                for (i, suffix_indx) in suffix_array.iter().enumerate() {
                    inverse_suffix_array[*suffix_indx as usize] = i;
                }
            });

            scope.spawn(|| {
                nsv = vec![0u32; n+1];
                psv = vec![u32::MAX; n+1];
                
                for i in 1..n as u32 {
                    let mut j = i - 1;
                    while psv[j as usize] != u32::MAX && suffix_array[i as usize] < suffix_array[j as usize] {
                        nsv[j as usize] = i;
                        j = psv[j as usize];
                    }
                    psv[i as usize] = j;
                }
                psv = psv.iter().map(|&i| if i == u32::MAX {0} else {i}).collect::<Vec<_>>();
            });
        }); 
        
        let mut factors = Vec::new();
        let mut k = 0u32;
        while k < n as u32{
            let psv = suffix_array[psv[inverse_suffix_array[k as usize] as usize] as usize];
            let nsv = suffix_array[nsv[inverse_suffix_array[k as usize] as usize] as usize];
            let (p,l,c,indx) = LZ77::lz_factor(k, psv, nsv, input);
            k = indx;
            factors.push((p,l,c));
        } 
        
        let factors = factors.into_iter()
            .scan(0, |count, (p,l,c)| {
                *count += l.max(1);
                if l == 1 && *count >= 128 {
                    return Some(vec![(0,0,input[p as usize])]);
                }
                if l == 2 && *count >= 32768 {
                    return Some(vec![(0,0,input[p as usize]), (0,0,input[p as usize + 1])]);
                }
                if l == 3 && *count >= 8388608 {
                    return Some(vec![(0,0,input[p as usize]), (0,0,input[p as usize + 1]), (0,0,input[p as usize + 2])]);
                }
                Some(vec![(p,l,c)])
            })
            .flatten()
            .collect::<Vec<_>>();
            
        let mut current_char_index = 0usize;
        let mut lenght_size = 1;
        let mut max_lenght = 2u32.pow(lenght_size as u32) - 1;
        let mut buffer = BitBuffer::new();

        let char_count = factors.iter().filter(|(_,l,_)| *l == 0).count();
        let char_prob = char_count as f32 / factors.len() as f32;
        
        let flag_mode = if char_prob > 0.25 {
            buffer.write_bit(true);
            true
        } else {
            buffer.write_bit(false);
            false
        };        
        
        factors.into_iter().fold(buffer, | mut acc ,(mut p,mut l,c)| {
            if l == 0 {
                if flag_mode {
                    acc.write_bit(false);
                } else {
                    acc.write_bits(0, lenght_size);
                }
                acc.write_byte(c);
                current_char_index += 1;
                lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                max_lenght = 2u32.pow(lenght_size as u32) - 1;
                return acc;
            } 
            while l > max_lenght{
                let current_bits = 32 - (current_char_index as u32).leading_zeros() as u8;
                if flag_mode {
                    acc.write_bit(true);
                } 
                acc.write_bits(u32::MAX, lenght_size);
                acc.write_bits(p as u32, current_bits);
                p += max_lenght;
                l -= max_lenght;
                current_char_index += max_lenght as usize;
                lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                max_lenght = 2u32.pow(lenght_size as u32) - 1;
            }
            let current_bits = 32 - (current_char_index as u32).leading_zeros() as u8;
            if flag_mode {
                acc.write_bit(true);
            }
            acc.write_bits(l as u32, lenght_size);
            acc.write_bits(p as u32, current_bits);
            current_char_index += l as usize;
            lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
            max_lenght = 2u32.pow(lenght_size as u32) - 1;

            acc
        })
    }

    #[inline]
    fn lenght_size(bits: u8) -> u8 {
        (bits / 2).min(8).max(1)
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
    fn lz_factor(i:u32, psv: u32, nsv: u32, x: &[u8]) -> (u32, u32, u8, u32) {
        let v1 = LZ77::lpc(x, i, psv);
        let v2 = LZ77::lpc(x, i, nsv);
        let (p,l) = if v1 > v2 {
            (psv, v1)
        } else {
            (nsv, v2)
        };
        if let Some(e) = x.get((i + l) as usize) {
            return (p, l, *e, i + l.max(1));
        }
        (p, l, 0, i + l)
    }

    pub fn encode(input: &[u8], bits: u8) -> LZ77 {
        let n = input.len();
        let chunk_size = 2usize.pow(bits as u32) - 2;
        let num_chunks = n / chunk_size + if n % chunk_size == 0 {0} else {1};

        let progress = indicatif::ProgressBar::new(num_chunks as u64);
        progress.set_position(0);
        let data = (0..num_chunks).into_par_iter() 
            .map(|i| {
                let start = i * chunk_size;
                let end = usize::min((i + 1) * chunk_size, n);
                let chunk = &input[start..end];
                let factors = LZ77::fast_encode(chunk);
                progress.inc(1);
                factors
            })
            .collect::<Vec<_>>();

        progress.finish_and_clear();
        
        LZ77 {
            bitbuffers: data,
        }
    }

    pub fn decode(self) -> Vec<u8> {
        self.bitbuffers.into_par_iter().progress().flat_map(|mut chunk| {
            let mut current_char_index = 0usize;
            let mut factors = Vec::new();
            let mut current_bits;
            let mut lenght_size = 1;
            let flag_mode = chunk.read_bit().unwrap();
            if flag_mode {
                while let Some(char_flag) = chunk.read_bit() {
                    match char_flag {
                        false => {
                            factors.push((0, 0, chunk.read_byte().unwrap()));
                            current_char_index += 1;
                            lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                        },
                        true => {
                            let l = chunk.read_bits(lenght_size).unwrap();
                            current_bits =  32 - (current_char_index as u32).leading_zeros() as u8;
                            factors.push((chunk.read_bits(current_bits).unwrap(), l, 0));         
                            current_char_index += l as usize;     
                            lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                        },
                    }
                }
            } else {
                while let Some(l) = chunk.read_bits(lenght_size) {
                    if l == 0 {
                        factors.push((0, 0, chunk.read_byte().unwrap()));
                        current_char_index += 1;
                        lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                    } else {
                        current_bits =  32 - (current_char_index as u32).leading_zeros() as u8;
                        factors.push((chunk.read_bits(current_bits).unwrap(), l, 0));         
                        current_char_index += l as usize;     
                        lenght_size = Self::lenght_size(31 - (current_char_index as u32).leading_zeros() as u8);
                    }
                }
            }
            LZ77::decode_chunk(factors)
        }).collect::<Vec<_>>()
    }

}