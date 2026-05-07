use risc0_zkvm::guest::env;

fn main() {
    let n: u32 = env::read();
    let a0: u64 = env::read();
    let b0: u64 = env::read();

    let mut a = a0;
    let mut b = b0;
    for _ in 0..(n / 2) {
        a = a + b;
        b = a + b;
    }

    env::commit(&a);
}
