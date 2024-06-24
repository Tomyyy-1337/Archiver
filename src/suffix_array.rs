use std::cmp::Ordering;

pub struct SuffixArray {
    pub n: usize,
    pub s: Vec<u8>,
    pub array: Vec<usize>,
}

fn compare_node(i: usize, j: usize, k: usize, rank: &[i32]) -> Ordering {
    if rank[i] != rank[j] {
        rank[i].cmp(&rank[j])
    } else {
        let ri = if i + k < rank.len() { rank[i + k] } else { -1 };
        let rj = if j + k < rank.len() { rank[j + k] } else { -1 };
        ri.cmp(&rj)
    }
}

impl SuffixArray {
    pub fn new(s: &[u8]) -> SuffixArray {
        let n = s.len();
        let mut rank = vec![0; n + 1];
        let mut array = vec![0; n + 1];

        for i in 0..=n {
            array[i] = i;
            rank[i] = if i < n { s[i] as i32 } else { -1 };
        }

        let mut tmp = vec![0; n + 1];
        let mut k = 1;
        while k <= n {
            array.sort_by(|a, b| compare_node(*a, *b, k, &rank));

            tmp[array[0]] = 0;
            for i in 1..=n {
                let d = if compare_node(array[i - 1], array[i], k, &rank) == Ordering::Less {
                    1
                } else {
                    0
                };
                tmp[array[i]] = tmp[array[i - 1]] + d;
            }
            std::mem::swap(&mut rank, &mut tmp);
            k *= 2;
        }

        SuffixArray {
            n,
            array,
            s: Vec::from(s),
        }
    }

    pub fn contains(&self, t: &[u8]) -> bool {
        let b = self.lower_bound(t);
        if b >= self.array.len() {
            false
        } else {
            let start = self.array[b];
            let end = (t.len() + start).min(self.s.len());
            let sub = &self.s[start..end];
            sub == t
        }
    }

    fn binary_search<F>(&self, string: &[u8], f: F) -> usize
    where
        F: Fn(&[u8], &[u8]) -> bool,
    {
        let (mut ng, mut ok) = (-1, self.n as i32 + 1);
        while ok - ng > 1 {
            let pos = (ng + ok) / 2;
            let start = self.array[pos as usize];
            let end = (start + string.len()).min(self.s.len());
            let substring = &self.s[start..end];
            if f(substring, string) {
                ng = pos;
            } else {
                ok = pos;
            }
        }
        ok as usize
    }

    pub fn lower_bound(&self, t: &[u8]) -> usize {
        let check_function = |sub: &[u8], s: &[u8]| sub.cmp(s) == Ordering::Less;
        self.binary_search(t, check_function)
    }

    pub fn upper_bound(&self, t: &[u8]) -> usize {
        let check_function = |sub: &[u8], s: &[u8]| sub.cmp(s) != Ordering::Greater;
        self.binary_search(t, check_function)
    }
}

pub fn construct_lcp<T: Ord>(string: &[T], suffix_array: &[usize]) -> Vec<usize> {
    assert_eq!(string.len() + 1, suffix_array.len());
    let n = string.len();
    let mut lcp = vec![0; n];
    let mut rank = vec![0; n + 1];
    for i in 0..=n {
        rank[suffix_array[i]] = i;
    }

    let mut height = 0;
    lcp[0] = 0;
    for i in 0..n {
        let j = suffix_array[rank[i] - 1];

        if height > 0 {
            height -= 1;
        }
        while j + height < n && i + height < n {
            if string[j + height] != string[i + height] {
                break;
            }
            height += 1;
        }

        lcp[rank[i] - 1] = height;
    }

    lcp
}