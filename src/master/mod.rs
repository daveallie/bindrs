use std::process::Stdio;

pub mod remote_info;
mod rsync;

use self::remote_info::RemoteInfo;

pub fn run(base_dir: &str, remote_dir: &str, port: Option<&str>, ignore_strings: &mut Vec<String>) {
    let ignores = super::shared::helpers::process_ignores(ignore_strings);
    let remote_info = RemoteInfo::build(remote_dir, port);
    rsync::run(base_dir, &remote_info, &ignores);
    start_remote_slave(&remote_info, &ignore_strings);
}

fn start_remote_slave(remote_info: &RemoteInfo, ignores: &Vec<String>) {
    let ignore_vec: Vec<String> = ignores.iter().map(|i| format!("--ignore \"{}\"", i)).collect();
    let cmd = format!("bindrs slave {} {}", remote_info.path, ignore_vec.join(" "));

    let mut command = remote_info.generate_command(&mut remote_info.base_command(&cmd), &cmd)
                                 .stdin(Stdio::piped())
                                 .stdout(Stdio::piped())
                                 .spawn().unwrap();
}
