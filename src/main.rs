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
extern crate slog_term;
#[macro_use]
extern crate clap;
extern crate regex;
extern crate notify;
#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate byteorder;
extern crate filetime;
extern crate time;
extern crate tempdir;

use clap::{App, ArgMatches};
use slog::Drain;
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::sync::Mutex;

mod master;
mod slave;
mod helpers;
mod processors;
mod structs;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let yaml = load_yaml!("cli.yml");
    let m = App::from_yaml(yaml).version(VERSION).get_matches();

    if let Some(sub_m) = m.subcommand_matches("run") {
        run_master(sub_m);
    } else if let Some(sub_m) = m.subcommand_matches("slave") {
        run_slave(sub_m);
    }
}

fn run_master(m: &ArgMatches) {
    #[cfg_attr(feature="clippy", allow(option_unwrap_used))]
    // Unwrap is safe - required by clap
    let base_dir = get_base_dir(m.value_of("base_dir").unwrap());
    #[cfg_attr(feature="clippy", allow(option_unwrap_used))]
    let remote_dir = m.value_of("remote_dir").unwrap(); // Unwrap is safe - required by clap
    let remote_port = m.value_of("port");
    let verbose_mode = m.is_present("verbose");
    let mut ignore_strings = get_ignore_strings(m);

    let log = setup_log(&base_dir, verbose_mode, true);
    info!(log, "Starting BindRS");

    master::run(
        &log,
        &base_dir,
        remote_dir,
        remote_port,
        &mut ignore_strings,
        verbose_mode,
    )
}

fn run_slave(m: &ArgMatches) {
    #[cfg_attr(feature="clippy", allow(option_unwrap_used))]
    // Unwrap is safe - required by clap
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
    helpers::resolve_path(base_dir).unwrap_or_else(|| {
        helpers::print_error_and_exit("failed to find base directory");
        "".to_owned()
    })
}

fn setup_log(base_dir: &str, verbose_mode: bool, master_mode: bool) -> slog::Logger {
    let mut path_buf = Path::new(base_dir).to_path_buf();
    path_buf.push(".bindrs");

    match fs::create_dir_all(path_buf.as_path()) {
        Ok(_) => (),
        Err(_) => helpers::print_error_and_exit("Failed to create .bindrs directory!"),
    }

    path_buf.push("bindrs");
    path_buf.set_extension("log");

    let wrapped_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path_buf.as_path());

    if let Ok(file) = wrapped_file {
        let level = if verbose_mode {
            slog::Level::Debug
        } else {
            slog::Level::Info
        };

        let file_decorator = slog_term::PlainSyncDecorator::new(file);
        let file_drain = slog_term::FullFormat::new(file_decorator).build();
        let file_drain = slog::LevelFilter::new(file_drain, level);

        if master_mode {
            let term_decorator = slog_term::TermDecorator::new().build();
            let term_drain = Mutex::new(slog_term::CompactFormat::new(term_decorator).build())
                .fuse();
            let term_drain = slog::LevelFilter::new(term_drain, level);
            let drain = slog::Duplicate::new(file_drain, term_drain);

            slog::Logger::root(drain.fuse(), o!("version" => VERSION, "mode" => "master"))
        } else {
            slog::Logger::root(
                file_drain.fuse(),
                o!("version" => VERSION, "mode" => "slave"),
            )
        }
    } else {
        helpers::print_error_and_exit("Failed to create log file.");
        panic!(); // For compilation
    }
}
