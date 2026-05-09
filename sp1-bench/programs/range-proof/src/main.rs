#![no_main]
sp1_zkvm::entrypoint!(main);

pub fn main() {
    let x = sp1_zkvm::io::read::<u64>();
    let lower = sp1_zkvm::io::read::<u64>();
    let upper = sp1_zkvm::io::read::<u64>();

    let gt_lower = (x > lower) as u64;
    let lt_upper = (x < upper) as u64;
    let in_range = gt_lower + lt_upper;

    sp1_zkvm::io::commit_slice(&in_range.to_le_bytes());
}
