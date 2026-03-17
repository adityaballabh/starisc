use std::path::Path;

pub fn get_op_path(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join(format!("{}.op", name))
        .to_str()
        .unwrap()
        .to_string()
}
