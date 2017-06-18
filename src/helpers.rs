use regex::RegexSet;
use semver::Version;
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

pub fn compare_version_strings(log: &Logger, local_version_str: &str, remote_version_str: &str) {
    let local_version = Version::parse(local_version_str).unwrap_or_else(|e| {
        log_error_and_exit(
            log,
            &format!("Could not parse local version: {}", local_version_str),
        );
        panic!(e)
    });
    let remote_version = Version::parse(remote_version_str).unwrap_or_else(|e| {
        log_error_and_exit(
            log,
            &format!("Could not parse remote version: {}", remote_version_str),
        );
        panic!(e)
    });

    if local_version == remote_version {
        return;
    }

    if versions_are_compatible(&local_version, &remote_version) {
        warn!(
            log,
            "BindRS versions differ, consider updating older version to match the newer version. \
             Local: {} - Remote: {}",
            local_version_str,
            remote_version_str
        );
    } else {
        log_error_and_exit(
            log,
            &format!(
                "BindRS versions too different between local and remote. \
                 Please update older version to match newer version. Local: {} - Remote: {}",
                local_version_str,
                remote_version_str
            ),
        );
    }
}

fn versions_are_compatible(version_a: &Version, version_b: &Version) -> bool {
    version_a.major == version_b.major && version_a.minor == version_b.minor
}

fn vec_to_regex_set(log: &Logger, ignores: &[String]) -> RegexSet {
    RegexSet::new(&convert_to_project_regex_strings(ignores)[..]).unwrap_or_else(|e| {
        log_error_and_exit(log, &format!("Provided regex failed to parse: {}", e));
        panic!() // For compilation
    })
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

    #[test]
    fn versions_are_compatible_correctly_matches() {
        let base_version = Version::parse("1.2.3").unwrap();
        assert!(versions_are_compatible(
            &Version::parse("1.2.3").unwrap(),
            &base_version,
        ));
        assert!(versions_are_compatible(
            &Version::parse("1.2.0").unwrap(),
            &base_version,
        ));
        assert!(!versions_are_compatible(
            &Version::parse("1.3.3").unwrap(),
            &base_version,
        ));
        assert!(!versions_are_compatible(
            &Version::parse("2.2.3").unwrap(),
            &base_version,
        ));
    }
}
