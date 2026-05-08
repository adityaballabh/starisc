use risc0_zkvm::guest::env;

fn main() {
    let n: u32 = env::read();
    let a0: u64 = env::read();
    let b0: u64 = env::read();

    let mut a = a0;
    let mut b = b0;
    for _ in 0..n {
        let c = a.wrapping_add(b);
        a = b;
        b = c;
    }

    env::commit(&a);
}
