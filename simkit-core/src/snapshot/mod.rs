use serde::Serialize;

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

/// Serialize value to canonical JSON and compute FNV-1a 64-bit hash.
pub fn hash<T: Serialize>(value: &T) -> u64 {
    let json = serde_json::to_vec(value).expect("serialize");
    fnv1a(&json)
}

pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Foo { a: u32, b: u32 }

    #[test]
    fn hash_stable() {
        let f1 = Foo { a: 1, b: 2 };
        let f2 = Foo { a: 1, b: 2 };
        assert_eq!(hash(&f1), hash(&f2));
    }
}

