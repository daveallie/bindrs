use std::io::{self, BufReader};
use super::shared::bound_file::BoundFile;
use std::{time, thread};

pub fn run(base_dir: &str, ignore_strings: &mut Vec<String>) {
    let mut writer = io::stdout();
    BoundFile{x: 1.2, y: 3.4}.to_writer(&mut writer);
    BoundFile{x: 5.6, y: 7.8}.to_writer(&mut writer);
    // println!("Running slave in dir: {}", base_dir);

    thread::sleep(time::Duration::new(1, 0));

    let mut br = BufReader::new(io::stdin());
    let bf = BoundFile::from_reader(&mut br);
    info!("{}, {}", bf.x, bf.y);
}
