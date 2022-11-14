#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Signature {
    Simple(Box<[u8]>),
    Complex {
        needle: Box<[(u8, u8)]>,
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
                let sig_byte = (a << 4) | (b & 0x0F);
                let mask = ((a != 0x10) as u8 * 0xF0) | ((b != 0x10) as u8 * 0x0F);
                needle.push((sig_byte & mask, mask));
            }

            let mut skip_offsets = [0; 256];

            let mut unknown = 0;
            let end = needle.len() - 1;
            for (i, &(byte, mask)) in needle.iter().enumerate().take(end) {
                if mask == 0xFF {
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
                needle: needle.into_boxed_slice(),
                skip_offsets,
            }
        } else {
            let mut needle = Vec::new();

            while let (Some(a), Some(b)) = (bytes_iter.next(), bytes_iter.next()) {
                let sig_byte = (a << 4) | b;
                needle.push(sig_byte);
            }

            Self::Simple(needle.into_boxed_slice())
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
                        .zip(needle.iter())
                        .all(|(&buf, &(search, mask))| buf & mask == search)
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

#[test]
fn foo() {
    Signature::new("C?");
    println!();
    let sig = Signature::new("48 83 3C ?? 00 75 ?? 8B C? E8");
    dbg!(&sig);
    sig.scan(&[0x48, 0x83, 0x3C, 0x03, 0x00, 0x75, 0x1B, 0x8B, 0xCF, 0xE8]);
}
