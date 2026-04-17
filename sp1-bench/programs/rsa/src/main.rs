#![no_main]
sp1_zkvm::entrypoint!(main);

pub fn main() {
    let p: u64 = 70_001;
    let q: u64 = 60_013;
    let n = p * q;
    let phi = (p - 1) * (q - 1);

    let e: u64 = 65_537;
    let d: u64 = 2_145_513_473;

    let message: u64 = 1_337;
    let encrypted = mod_pow(message, e, n);
    let decrypted = mod_pow(encrypted, d, n);

    assert_eq!((e * d) % phi, 1);
    assert_eq!(decrypted, message);

    sp1_zkvm::io::commit_slice(&decrypted.to_le_bytes());
}

fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    let mut result = 1_u64;
    base %= modulus;

    while exp > 0 {
        if exp & 1 == 1 {
            result = result * base % modulus;
        }
        base = base * base % modulus;
        exp >>= 1;
    }

    result
}
