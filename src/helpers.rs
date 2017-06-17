use regex::RegexSet;
#[cfg(test)]
use slog::Discard;
use slog::Logger;
use std::{thread, time};
#[cfg(test)]
use std::env::current_dir;
use std::fs::canonicalize;
use std::path::Path;
use std::process::exit;

pub fn resolve_path(dir: &str) -> Option<String> {
    match canonicalize(Path::new(dir)) {
        Ok(p) => Some(p.to_string_lossy().into_owned()),
        Err(_) => None,
    }
}

#[cfg_attr(feature = "clippy", allow(print_stdout))]
pub fn print_error_and_exit(msg: &str) {
    println!("{}", msg);
    exit(1);
}

pub fn log_error_and_exit(log: &Logger, msg: &str) {
    error!(log, "{}", msg);
    thread::sleep(time::Duration::from_millis(500));
    exit(1);
}

pub fn process_ignores(log: &Logger, vec: &mut Vec<String>) -> RegexSet {
    if vec.is_empty() {
        vec.push("^\\.git(?:/[^/]+)*$".to_owned());
    }
    vec.push("^\\.bindrs.*$".to_owned());

    vec_to_regex_set(log, vec)
}

fn vec_to_regex_set(log: &Logger, ignores: &[String]) -> RegexSet {
    match RegexSet::new(&convert_to_project_regex_strings(ignores)[..]) {
        Ok(rs) => rs,
        Err(e) => {
            log_error_and_exit(log, &format!("Provided regex failed to parse: {}", e));
            panic!() // For compilation
        }
    }
}

fn convert_to_project_regex_strings(ignores: &[String]) -> Vec<String> {
    let mut regexes: Vec<String> = vec![];

    for i in ignores.iter() {
        let mut ignore = i.clone();
        if !(ignore.starts_with('^') && ignore.ends_with('$')) {
            ignore = format!("^{}(?:/[^/]+)*$", ignore);
        }
        regexes.push(ignore)
    }

    regexes
}


#[cfg(test)]
mod tests {
    use super::*;

    fn test_logger() -> Logger {
        Logger::root(Discard, o!())
    }

    #[test]
    fn including_custom_ignore_skips_git() {
        let mut strings: Vec<String> = vec![];
        strings.push("^something$".to_owned());
        let regex_set = process_ignores(&test_logger(), &mut strings);
        assert!(regex_set.is_match("something"));
        assert!(regex_set.is_match(".bindrsasdf"));
        assert!(!regex_set.is_match(".git/something"));
    }

    #[test]
    fn excluding_custom_ignore_includes_git() {
        let mut strings: Vec<String> = vec![];
        let regex_set = process_ignores(&test_logger(), &mut strings);
        assert!(!regex_set.is_match("something"));
        assert!(regex_set.is_match(".bindrsasdf"));
        assert!(regex_set.is_match(".git/something"));
    }

    #[test]
    fn regex_strings_are_not_modified() {
        let mut strings: Vec<String> = vec![];
        strings.push("^something$".to_owned());
        let regex_set = process_ignores(&test_logger(), &mut strings);
        assert!(regex_set.is_match("something"));
        assert!(!regex_set.is_match("somethin"));
        assert!(!regex_set.is_match("somethingg"));
        assert!(!regex_set.is_match("something/somethingelse"));
    }

    #[test]
    fn non_regex_strings_are_modified() {
        let mut strings: Vec<String> = vec![];
        strings.push("something".to_owned());
        let regex_set = process_ignores(&test_logger(), &mut strings);
        assert!(regex_set.is_match("something"));
        assert!(!regex_set.is_match("somethin"));
        assert!(!regex_set.is_match("somethingg"));
        assert!(regex_set.is_match("something/somethingelse"));
    }

    #[test]
    fn resolve_path_canonicalize_correctly() {
        let mut path = current_dir().unwrap();
        path.push("src");
        assert_eq!(
            canonicalize(path.as_path()).unwrap().to_str().unwrap(),
            resolve_path("./src/processors/../../src").unwrap()
        );
    }
}
