use helpers;
use regex::RegexSet;
use slog::Logger;
use std::process::Command;
use structs::remote_info::RemoteInfo;

pub fn run(log: &Logger, base_dir: &str, remote_info: &RemoteInfo, ignores: &RegexSet) {
    let args = rsync_args(log, base_dir, remote_info, ignores);

    info!(log, "Running initial rsync");
    match Command::new("rsync").args(&args).output() {
        Ok(output) => {
            debug!(log, "{}", String::from_utf8_lossy(&output.stdout));
            debug!(log, "Finished initial rsync");
        }
        Err(e) => helpers::log_error_and_exit(log, &format!("Failed to run rsync: {}", e)),
    }
}

fn rsync_args(
    log: &Logger,
    base_dir: &str,
    remote_info: &RemoteInfo,
    ignores: &RegexSet,
) -> Vec<String> {
    let mut args: Vec<String> = vec!["-azv".to_owned()];

    for path in find_rsync_ignore_folders(log, base_dir, remote_info, ignores) {
        args.push("--exclude".to_owned());
        args.push(path);
    }

    if remote_info.is_remote {
        args.push(format!(" -e \"ssh -p {}\"", remote_info.port));
    }

    args.push("--delete".to_owned());
    args.push("--ignore-errors".to_owned());
    args.push(format!("{}/", base_dir));
    args.push(remote_info.full_path_trailing_slash());
    args
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
