use risc0_zkvm::guest::env;

fn main() {
    let x: u64 = env::read();
    let lower: u64 = env::read();
    let upper: u64 = env::read();

    let gt_lower = (x > lower) as u64;
    let lt_upper = (x < upper) as u64;
    let in_range = gt_lower + lt_upper;

    env::commit(&in_range);
}
