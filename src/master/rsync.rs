use master::remote_info::RemoteInfo;
use std::process::Command;
use regex::RegexSet;

pub fn run(base_dir: &str, remote_info: &RemoteInfo, ignores: &RegexSet) {
    let args = rsync_args(base_dir, remote_info, ignores);

    info!("Running initial rsync");
    let output = Command::new("rsync").args(&args).output().unwrap_or_else(|e| {
        panic!("failed to run rsync: {}", e)
    });

    debug!("{}", String::from_utf8_lossy(&output.stdout));
    debug!("Finished initial rsync");
}

fn rsync_args(base_dir: &str, remote_info: &RemoteInfo, ignores: &RegexSet) -> Vec<String> {
    let mut args: Vec<String> = vec!["-azv".to_owned()];

    for path in find_rsync_ignore_folders(base_dir, remote_info, ignores) {
        args.push("--exclude".to_owned());
        args.push(path);
    }

    if remote_info.is_remote {
        args.push(format!(" -e \"ssh -p {}\"", remote_info.port));
    }

    args.push("--delete".to_owned());
    args.push(format!("{}/", base_dir));
    args.push(remote_info.full_path() + "/");
    args
}

fn find_rsync_ignore_folders(base_dir: &str, remote_info: &RemoteInfo, ignores: &RegexSet) -> Vec<String> {
    let output = Command::new("find").arg(base_dir).arg("-type").arg("d").output().unwrap_or_else(|e| {
        panic!("failed to run local find: {}", e)
    });
    let mut folders = process_raw_file_list(base_dir, String::from_utf8_lossy(&output.stdout).to_mut());

    let cmd = &format!("find {} -type d", remote_info.path);
    let output = remote_info.generate_command(&mut remote_info.base_command(cmd), cmd).output().unwrap_or_else(|e| {
        panic!("failed to run remote find: {}", e)
    });

    folders.append(&mut process_raw_file_list(&remote_info.path, String::from_utf8_lossy(&output.stdout).to_mut()));

    folders.sort();
    folders.dedup();
    folders.into_iter().filter(|f| ignores.is_match(f)).collect()
}

fn process_raw_file_list(base_dir: &str, output: &str) -> Vec<String> {
    let base_length = base_dir.len() + 1;
    output.split("\n").skip(1)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().skip(base_length).collect())
        .collect()
}
