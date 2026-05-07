use sp1_build::build_program_with_args;

fn main() {
    build_program_with_args("../programs/fib", Default::default());
    build_program_with_args("../programs/rsa-enc", Default::default());
    build_program_with_args("../programs/rsa-dec", Default::default());
}
