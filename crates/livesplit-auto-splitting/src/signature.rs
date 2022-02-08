pub enum Signature {
    Simple(Vec<u8>),
    Complex {
        needle: Vec<(u8, bool)>,
        skip_offsets: [usize; 256],
    },
}

impl Signature {
    pub fn new(signature: &str) -> Self {
        let mut bytes_iter = signature.bytes().filter_map(|b| match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 0xA),
            b'A'..=b'F' => Some(b - b'A' + 0xA),
            b'?' => Some(0x10),
            _ => None,
        });

        if memchr::memchr(b'?', signature.as_bytes()).is_some() {
            let mut needle = Vec::new();

            while let (Some(a), Some(b)) = (bytes_iter.next(), bytes_iter.next()) {
                let sig_byte = (a << 4) | b;
                let is_question_marks = a == 0x10 && b == 0x10;
                needle.push((sig_byte, is_question_marks));
            }

            let mut skip_offsets = [0; 256];

            let mut unknown = 0;
            let end = needle.len() - 1;
            for (i, &(byte, mask)) in needle.iter().enumerate().take(end) {
                if !mask {
                    skip_offsets[byte as usize] = end - i;
                } else {
                    unknown = end - i;
                }
            }

            if unknown == 0 {
                unknown = needle.len();
            }

            for offset in &mut skip_offsets[..] {
                if unknown < *offset || *offset == 0 {
                    *offset = unknown;
                }
            }

            Self::Complex {
                needle,
                skip_offsets,
            }
        } else {
            let mut needle = Vec::new();

            while let (Some(a), Some(b)) = (bytes_iter.next(), bytes_iter.next()) {
                let sig_byte = (a << 4) | b;
                needle.push(sig_byte);
            }

            Self::Simple(needle)
        }
    }

    pub fn scan(&self, haystack: &[u8]) -> Option<usize> {
        match self {
            Signature::Simple(needle) => memchr::memmem::find(haystack, needle),
            Signature::Complex {
                needle,
                skip_offsets,
            } => {
                let mut current = 0;
                let end = needle.len() - 1;
                while current <= haystack.len() - needle.len() {
                    let rem = &haystack[current..];
                    if rem
                        .iter()
                        .zip(needle)
                        .all(|(&buf, &(search, mask))| buf == search || mask)
                    {
                        return Some(current);
                    }
                    let offset = skip_offsets[rem[end] as usize];
                    current += offset;
                }
                None
            }
        }
    }
}
