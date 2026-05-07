use risc0_zkvm::guest::env;

fn main() {
    let message: u64 = env::read();
    let n: u64 = env::read();
    let e: u64 = 65_537;

    let encrypted = mod_pow(message, e, n);

    env::commit(&encrypted);
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
