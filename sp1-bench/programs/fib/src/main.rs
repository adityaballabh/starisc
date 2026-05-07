#![no_main]
sp1_zkvm::entrypoint!(main);

pub fn main() {
    let n = sp1_zkvm::io::read::<u32>();
    let a0 = sp1_zkvm::io::read::<u64>();
    let b0 = sp1_zkvm::io::read::<u64>();

    let mut a = a0;
    let mut b = b0;
    for _ in 0..(n / 2) {
        a = a + b;
        b = a + b;
    }

    sp1_zkvm::io::commit_slice(&a.to_le_bytes());
}
