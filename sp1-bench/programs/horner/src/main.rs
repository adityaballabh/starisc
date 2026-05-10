#![no_main]
sp1_zkvm::entrypoint!(main);

pub fn main() {
    let n = sp1_zkvm::io::read::<u32>();
    let x = sp1_zkvm::io::read::<u64>();

    let mut acc: u64 = 0;
    for i in 0..n {
        let coeff = u64::from(i + 1);
        acc = acc.wrapping_mul(x).wrapping_add(coeff);
    }

    sp1_zkvm::io::commit_slice(&acc.to_le_bytes());
}
