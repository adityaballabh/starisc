use risc0_zkvm::guest::env;

fn main() {
    let modulus: u64 = 4_294_967_291;
    let secret: u64 = 1_337;
    let encrypted = mod_pow(secret, 17, modulus);
    let decrypted = mod_pow(encrypted, 1_768_515_943, modulus);

    assert_eq!(decrypted, secret);
    env::commit(&decrypted);
}

fn mod_pow(mut base: u64, mut exp: u32, modulus: u64) -> u64 {
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
