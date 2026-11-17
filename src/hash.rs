//! A tiny, stable, dependency-free hash for cache keys (FNV-1a, 64-bit).
//!
//! Cache keys only need to be well spread and identical across runs. They do
//! not need to resist attackers, so a dozen lines beat a dependency.

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

pub struct Fnv64(u64);

impl Default for Fnv64 {
    fn default() -> Self {
        Self(OFFSET)
    }
}

impl Fnv64 {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mixes in one field. A separator follows every field so that
    /// `("ab", "c")` and `("a", "bc")` hash differently.
    pub fn field(mut self, bytes: &[u8]) -> Self {
        for &byte in bytes.iter().chain(&[0xff]) {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(PRIME);
        }
        self
    }

    pub fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_the_offset_basis() {
        assert_eq!(Fnv64::new().hex(), "cbf29ce484222325");
    }

    #[test]
    fn field_boundaries_matter() {
        let a = Fnv64::new().field(b"ab").field(b"c").hex();
        let b = Fnv64::new().field(b"a").field(b"bc").hex();
        assert_ne!(a, b);
    }

    #[test]
    fn same_input_same_key() {
        let a = Fnv64::new().field(b"fn main() {}").hex();
        let b = Fnv64::new().field(b"fn main() {}").hex();
        assert_eq!(a, b);
    }
}
