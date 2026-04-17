#![no_main]
sp1_zkvm::entrypoint!(main);

pub fn main() {
    let n = sp1_zkvm::io::read::<u32>();

    let mut a: u64 = 0;
    let mut b: u64 = 1;
    for _ in 0..n {
        let c = a + b;
        a = b;
        b = c;
    }

    sp1_zkvm::io::commit_slice(&a.to_le_bytes());
}
