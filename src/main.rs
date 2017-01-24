#![deny(missing_debug_implementations, missing_copy_implementations,
    trivial_casts, trivial_numeric_casts,
    unsafe_code, unstable_features,
    unused_import_braces, unused_qualifications)]

#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate clap;
extern crate regex;
extern crate notify;
extern crate bincode;
extern crate rustc_serialize;
extern crate byteorder;
extern crate filetime;
extern crate time;

use clap::{App, ArgMatches};
use shared::helpers;

mod master;
mod slave;
mod shared;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    env_logger::init().unwrap();

    let yaml = load_yaml!("cli.yml");
    let m = App::from_yaml(yaml)
        .version(VERSION)
        .get_matches();

    info!("BindRS v{}", VERSION);

    if let Some(ref m) = m.subcommand_matches("master") {
        run_master(m);
    } else if let Some(ref m) = m.subcommand_matches("slave") {
        run_slave(m);
    }
}

fn run_master(m: &ArgMatches) {
    debug!("Running in master mode");
    let base_dir = get_base_dir(m.value_of("base_dir").unwrap());
    let remote_dir = m.value_of("remote_dir").unwrap();
    let remote_port = m.value_of("port");
    let mut ignore_strings = get_ignore_strings(m);

    master::run(&base_dir, remote_dir, remote_port, &mut ignore_strings)
}

fn run_slave(m: &ArgMatches) {
    debug!("Running in slave mode");
    let base_dir = get_base_dir(m.value_of("base_dir").unwrap());
    let mut ignore_strings = get_ignore_strings(m);

    slave::run(&base_dir, &mut ignore_strings)
}

fn get_ignore_strings(m: &ArgMatches) -> Vec<String> {
    match m.values_of("ignore") {
        Some(i) => i.into_iter().map(|str| str.to_owned()).collect(),
        None => vec![],
    }
}

fn get_base_dir(base_dir: &str) -> String {
    match helpers::resolve_path(base_dir) {
        Some(dir) => dir,
        None => {
            helpers::error_and_exit("failed to find base directory");
            "".to_owned()
        }
    }
}
