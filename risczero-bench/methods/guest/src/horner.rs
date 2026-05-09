use risc0_zkvm::guest::env;

fn main() {
    let n: u32 = env::read();
    let x: u64 = env::read();

    let mut acc: u64 = 0;
    for i in 0..n {
        let coeff = u64::from(i + 1);
        acc = acc.wrapping_mul(x).wrapping_add(coeff);
    }

    env::commit(&acc);
}
