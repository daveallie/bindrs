#![deny(missing_debug_implementations, missing_copy_implementations,
    trivial_casts, trivial_numeric_casts,
    unsafe_code, unstable_features,
    unused_import_braces, unused_qualifications)]

#[macro_use]
extern crate slog;
extern crate slog_bunyan;
extern crate slog_stream;
extern crate slog_term;
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
use slog::{Level, LevelFilter, Logger, Duplicate, DrainExt};
use std::fs::{self, File};
use std::path::Path;

mod master;
mod slave;
mod shared;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let yaml = load_yaml!("cli.yml");
    let m = App::from_yaml(yaml)
        .version(VERSION)
        .get_matches();

    if let Some(ref m) = m.subcommand_matches("master") {
        run_master(m);
    } else if let Some(ref m) = m.subcommand_matches("slave") {
        run_slave(m);
    }
}

fn run_master(m: &ArgMatches) {
    let base_dir = get_base_dir(m.value_of("base_dir").unwrap());
    let remote_dir = m.value_of("remote_dir").unwrap();
    let remote_port = m.value_of("port");
    let verbose_mode = m.is_present("verbose");
    let mut ignore_strings = get_ignore_strings(m);

    let log = setup_log(&base_dir, verbose_mode, true);
    info!(log, "Starting BindRS");

    master::run(&log,
                &base_dir,
                remote_dir,
                remote_port,
                &mut ignore_strings)
}

fn run_slave(m: &ArgMatches) {
    let base_dir = get_base_dir(m.value_of("base_dir").unwrap());
    let mut ignore_strings = get_ignore_strings(m);
    let verbose_mode = m.is_present("verbose");

    let log = setup_log(&base_dir, verbose_mode, false);
    info!(log, "Starting BindRS");

    slave::run(&log, &base_dir, &mut ignore_strings)
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
            helpers::print_error_and_exit("failed to find base directory");
            "".to_owned()
        }
    }
}

fn setup_log(base_dir: &str, verbose_mode: bool, master_mode: bool) -> Logger {
    let mut path_buf = Path::new(base_dir).to_path_buf();
    path_buf.push(".bindrs");
    fs::create_dir_all(path_buf.as_path()).unwrap();

    path_buf.push("bindrs");
    path_buf.set_extension("log");

    let file = File::create(path_buf.as_path()).unwrap();
    let stream = slog_stream::stream(file, slog_bunyan::new().build());

    let level = match verbose_mode {
        true => Level::Debug,
        false => Level::Info,
    };

    if master_mode {
        let termlog = slog_term::streamer().async().full().build();
        Logger::root(Duplicate::new(LevelFilter::new(stream, level),
                                    LevelFilter::new(termlog, level))
                         .fuse(),
                     o!("version" => VERSION, "mode" => "master"))
    } else {
        Logger::root(LevelFilter::new(stream, level).fuse(),
                     o!("version" => VERSION, "mode" => "slave"))
    }
}
