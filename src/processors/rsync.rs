use helpers;
use regex::RegexSet;
use slog::Logger;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;
use structs::remote_info::RemoteInfo;
use tempdir::TempDir;

pub fn run(log: &Logger, base_dir: &str, remote_info: &RemoteInfo, ignores: &RegexSet) {
    let temp_dir = create_temp_dir(log, "rsync-data");
    let ignore_file_pathbuf = temp_dir.path().join("rsync-ignores");
    let ignore_file_path = ignore_file_pathbuf.as_path();
    let ignore_file_string_path = ignore_file_path.to_string_lossy().into_owned();

    build_rsync_ignore_file(log, ignore_file_path, base_dir, remote_info, ignores);
    let args_vec = rsync_args(base_dir, remote_info, &ignore_file_string_path);

    info!(log, "Running initial rsync");
    match Command::new("rsync").args(&args_vec).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stdout != "" {
                debug!(log, "{}", stdout);
            }
            if stderr != "" {
                helpers::log_error_and_exit(log, &stderr);
            }
            debug!(log, "Finished initial rsync");
        }
        Err(e) => helpers::log_error_and_exit(log, &format!("Failed to run rsync: {}", e)),
    }
}

fn build_rsync_ignore_file(
    log: &Logger,
    ignore_file_path: &Path,
    base_dir: &str,
    remote_info: &RemoteInfo,
    ignores: &RegexSet,
) {
    let mut ignore_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(ignore_file_path)
        .unwrap_or_else(|e| {
            helpers::log_error_and_exit(log, &format!("Could not create temp file: {}", e));
            panic!(e);
        });

    let mut folders = find_rsync_ignore_folders(log, base_dir, remote_info, ignores);
    folders.sort_by(|a, b| a.len().cmp(&b.len()));

    let mut written_folders: Vec<String> = vec![];
    for path in folders {
        let parent_file_written = written_folders.iter().any(|written_path| {
            path.starts_with(&format!("{}/", written_path))
        });

        if parent_file_written {
            // Don't write folder path if parent's folder has already been ignored
            continue;
        }

        written_folders.push(path.clone());
        if let Err(e) = writeln!(ignore_file, "{}", path) {
            helpers::log_error_and_exit(
                log,
                &format!("Could not append rsync ignore to temp file: {}", e),
            )
        }
    }
}

fn rsync_args(base_dir: &str, remote_info: &RemoteInfo, ignore_file_path: &str) -> Vec<String> {
    let mut args_vec: Vec<String> = vec!["-azv".to_owned()];

    args_vec.push("--exclude-from".to_owned());
    args_vec.push(ignore_file_path.to_owned());

    if remote_info.is_remote {
        args_vec.push("-e".to_owned());
        args_vec.push(format!("ssh -p {}", remote_info.port));
    }

    args_vec.push("--delete".to_owned());
    args_vec.push("--ignore-errors".to_owned());
    args_vec.push(format!("{}/", base_dir));
    args_vec.push(remote_info.full_path_trailing_slash());
    args_vec
}

fn find_rsync_ignore_folders(
    log: &Logger,
    base_dir: &str,
    remote_info: &RemoteInfo,
    ignores: &RegexSet,
) -> Vec<String> {
    let mut folders = match Command::new("find")
        .arg(base_dir)
        .arg("-type")
        .arg("d")
        .output() {
        Ok(o) => process_raw_file_list(base_dir, String::from_utf8_lossy(&o.stdout).to_mut()),
        Err(e) => {
            helpers::log_error_and_exit(log, &format!("Failed to run local find: {}", e));
            vec![]
        }
    };

    let cmd = &format!("find {} -type d", remote_info.path);
    match remote_info
        .generate_command(&mut remote_info.base_command(cmd), cmd)
        .output() {
        Ok(o) => {
            folders.append(&mut process_raw_file_list(
                &remote_info.path,
                String::from_utf8_lossy(&o.stdout).to_mut(),
            ))
        }
        Err(e) => helpers::log_error_and_exit(log, &format!("Failed to run remote find: {}", e)),
    }

    folders.sort();
    folders.dedup();
    folders
        .into_iter()
        .filter(|f| ignores.is_match(f))
        .collect()
}

fn create_temp_dir(log: &Logger, name: &str) -> TempDir {
    TempDir::new(name).unwrap_or_else(|e| {
        helpers::log_error_and_exit(log, &format!("Could not create temp directory: {}", e));
        panic!(e);
    })
}

#[cfg_attr(feature = "clippy", allow(filter_map))]
fn process_raw_file_list(base_dir: &str, output: &str) -> Vec<String> {
    let base_length = base_dir.len() + 1;
    output
        .split('\n')
        .skip(1)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().skip(base_length).collect())
        .collect()
}
