use risc0_zkvm::guest::env;

fn main() {
    let n: u32 = env::read();

    let mut a: u64 = 0;
    let mut b: u64 = 1;
    for _ in 0..n {
        let c = a + b;
        a = b;
        b = c;
    }

    env::commit(&a);
}
