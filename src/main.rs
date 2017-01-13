#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate clap;
extern crate regex;

use clap::{App, ArgMatches};
use regex::RegexSet;

mod master;
mod slave;

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
    let base_dir = m.value_of("base_dir").unwrap();
    let remote_dir = m.value_of("remote_dir").unwrap();
    let remote_port = m.value_of("port");

    let ignores = match m.values_of("ignore") {
        Some(i) => {
            let mut vec: Vec<String> = i.into_iter().map(|str| str.to_owned()).collect();
            vec.push("^\\.bindrs.*$".to_owned());
            process_ignores(&vec)
        },
        None => {
            RegexSet::new(&["^\\.git(?:/[^/]+)*$", "^\\.bindrs.*$"]).unwrap()
        }
    };

    master::run(base_dir, remote_dir, remote_port, ignores)
}

fn run_slave(m: &ArgMatches) {
    debug!("Running in slave mode");
    let base_dir = m.value_of("base_dir").unwrap();

    let ignores = match m.values_of("ignore") {
        Some(i) => {
            let vec = i.into_iter().map(|str| str.to_owned()).collect();
            process_ignores(&vec)
        },
        None => {
            let tmp: &[&str; 0] = &[];
            RegexSet::new(tmp).unwrap()
        }
    };

    slave::run(base_dir, ignores)
}

fn process_ignores(ignores: &Vec<String>) -> RegexSet {
    let mut regexes: Vec<String> = vec![];

    for i in ignores.iter() {
        let mut ignore = i.clone();
        if !(ignore.starts_with("^") && ignore.ends_with("$")) {
            ignore = format!("^{}(?:/[^/]+)*$", ignore);
        }
        regexes.push(ignore)
    }

    RegexSet::new(&regexes[..]).unwrap()
}