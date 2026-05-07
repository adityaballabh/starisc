#![no_main]
sp1_zkvm::entrypoint!(main);

pub fn main() {
    let message = sp1_zkvm::io::read::<u64>();
    let n = sp1_zkvm::io::read::<u64>();
    let e: u64 = 65_537;

    let encrypted = mod_pow(message, e, n);

    sp1_zkvm::io::commit_slice(&encrypted.to_le_bytes());
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
