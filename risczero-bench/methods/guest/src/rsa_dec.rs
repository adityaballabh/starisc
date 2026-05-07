use risc0_zkvm::guest::env;

fn main() {
    let encrypted: u64 = env::read();
    let n: u64 = env::read();
    let d: u64 = env::read();

    let decrypted = mod_pow(encrypted, d, n);

    env::commit(&decrypted);
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
