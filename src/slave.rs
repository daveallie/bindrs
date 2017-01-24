use super::shared::{executor, helpers};
use std::io::{self, BufReader, BufWriter};

pub fn run(base_dir: &str, ignore_strings: &mut Vec<String>) {
    let ignores = helpers::process_ignores(ignore_strings);

    let (remote_reader, remote_writer) = (BufReader::new(io::stdin()),
                                          BufWriter::new(io::stdout()));
    executor::start(base_dir.to_owned(), ignores, remote_reader, remote_writer);
}
