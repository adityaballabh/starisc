use std::path::Path;

pub fn sample_op_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples/sample.op")
        .to_str()
        .unwrap()
        .to_string()
}
