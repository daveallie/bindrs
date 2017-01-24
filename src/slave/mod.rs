use std::io::{self, BufReader, BufWriter};
use super::shared::{executor, helpers};

pub fn run(base_dir: &str, ignore_strings: &mut Vec<String>) {
    let base_dir = helpers::dir::resolve_path(base_dir);
    let ignores = helpers::process_ignores(ignore_strings);

    let base_dir = base_dir.unwrap_or_else(|| panic!("failed to find base directory"));

    let (remote_reader, remote_writer) = (BufReader::new(io::stdin()),
                                          BufWriter::new(io::stdout()));
    executor::start(base_dir, ignores, remote_reader, remote_writer);
}
