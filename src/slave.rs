use helpers;
use processors::executor;
use slog::Logger;
use std::io::{self, BufReader, BufWriter};

pub fn run(log: &Logger, base_dir: &str, ignore_strings: &mut Vec<String>) {
    let ignores = helpers::process_ignores(log, ignore_strings);

    let (remote_reader, remote_writer) = (BufReader::new(io::stdin()), BufWriter::new(io::stdout()));
    executor::start(log, base_dir, ignores, remote_reader, remote_writer);
}
