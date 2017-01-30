use regex::RegexSet;
use slog::Logger;
use std::{thread, time};
use std::fs::canonicalize;
use std::path::Path;
use std::process::exit;

pub fn resolve_path(dir: &str) -> Option<String> {
    match canonicalize(Path::new(dir)) {
        Ok(p) => Some(p.to_string_lossy().into_owned()),
        Err(_) => None,
    }
}

pub fn print_error_and_exit(msg: &str) {
    println!("{}", msg);
    exit(1);
}

pub fn log_error_and_exit(log: &Logger, msg: &str) {
    error!(log, msg);
    thread::sleep(time::Duration::from_millis(500));
    exit(1);
}

pub fn process_ignores(vec: &mut Vec<String>) -> RegexSet {
    if vec.len() == 0 {
        vec.push("^\\.git(?:/[^/]+)*$".to_owned());
    }
    vec.push("^\\.bindrs.*$".to_owned());

    vec_to_regex_set(&vec)
}

fn vec_to_regex_set(ignores: &Vec<String>) -> RegexSet {
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
