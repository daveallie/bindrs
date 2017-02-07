#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(feature="clippy", deny(clippy_pedantic))]
#![cfg_attr(feature="clippy", allow(missing_docs_in_private_items))]

#![deny(missing_debug_implementations, missing_copy_implementations,
    trivial_casts, trivial_numeric_casts, unsafe_code,
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

use slog::{Level, LevelFilter, Logger, Duplicate, DrainExt};
use std::fs::{self, File};
use std::path::Path;

mod master;
mod slave;
mod helpers;
mod processors;
mod structs;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let yaml = load_yaml!("cli.yml");
    let m = App::from_yaml(yaml)
        .version(VERSION)
        .get_matches();

    if let Some(sub_m) = m.subcommand_matches("master") {
        run_master(sub_m);
    } else if let Some(sub_m) = m.subcommand_matches("slave") {
        run_slave(sub_m);
    }
}

fn run_master(m: &ArgMatches) {
    #[cfg_attr(feature="clippy", allow(option_unwrap_used))]
    let base_dir = get_base_dir(m.value_of("base_dir").unwrap()); // Unwrap is safe - required by clap
    #[cfg_attr(feature="clippy", allow(option_unwrap_used))]
    let remote_dir = m.value_of("remote_dir").unwrap(); // Unwrap is safe - required by clap
    let remote_port = m.value_of("port");
    let verbose_mode = m.is_present("verbose");
    let mut ignore_strings = get_ignore_strings(m);

    let log = setup_log(&base_dir, verbose_mode, true);
    info!(log, "Starting BindRS");

    master::run(&log,
                &base_dir,
                remote_dir,
                remote_port,
                &mut ignore_strings,
                verbose_mode)
}

fn run_slave(m: &ArgMatches) {
    #[cfg_attr(feature="clippy", allow(option_unwrap_used))]
    let base_dir = get_base_dir(m.value_of("base_dir").unwrap()); // Unwrap is safe - required by clap
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
    helpers::resolve_path(base_dir).unwrap_or_else(|| {
        helpers::print_error_and_exit("failed to find base directory");
        "".to_owned()
    })
}

fn setup_log(base_dir: &str, verbose_mode: bool, master_mode: bool) -> Logger {
    let mut path_buf = Path::new(base_dir).to_path_buf();
    path_buf.push(".bindrs");

    match fs::create_dir_all(path_buf.as_path()) {
        Ok(_) => (),
        Err(_) => helpers::print_error_and_exit("Failed to create .bindrs directory!"),
    }

    path_buf.push("bindrs");
    path_buf.set_extension("log");

    if let Ok(file) = File::create(path_buf.as_path()) {
        let stream = slog_stream::stream(file, slog_bunyan::new().build());

        let level = if verbose_mode {
            Level::Debug
        } else {
            Level::Info
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
    } else {
        helpers::print_error_and_exit("Failed to create log file.");
        panic!(); // For compilation
    }
}
