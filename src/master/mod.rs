use std::process::Stdio;
use std::{time, thread};
use std::process::{ChildStdout, ChildStdin};
use super::shared::{helpers, executor};
use self::remote_info::RemoteInfo;

pub mod remote_info;
mod rsync;

pub fn run(base_dir: &str,
           remote_dir: &str,
           port: Option<&str>,
           ignore_strings: &mut Vec<String>) {
    let base_dir = helpers::dir::resolve_path(base_dir);
    let ignores = helpers::process_ignores(ignore_strings);
    let remote_info = RemoteInfo::build(remote_dir, port);

    let base_dir = base_dir.unwrap_or_else(|| panic!("failed to find base directory"));
    validate_remote_info(&remote_info);

    rsync::run(&base_dir, &remote_info, &ignores);
    let (remote_reader, remote_writer) = start_remote_slave(&remote_info, &ignore_strings);
    executor::start(base_dir, ignores, remote_reader, remote_writer);
}

fn start_remote_slave(remote_info: &RemoteInfo,
                      ignores: &Vec<String>)
                      -> (ChildStdout, ChildStdin) {
    info!("Starting remote slave");
    let ignore_vec: Vec<String> = ignores.iter().map(|i| format!("--ignore \"{}\"", i)).collect();
    let cmd = format!("./bindrs slave {} {}",
                      remote_info.path,
                      ignore_vec.join(" "));

    let mut child = remote_info.generate_command(&mut remote_info.base_command(&cmd), &cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    thread::sleep(time::Duration::new(1, 0));
    let c_stdout = child.stdout.take().unwrap();
    let c_stdin = child.stdin.take().unwrap();

    // let _ = child.wait();

    (c_stdout, c_stdin)
}

fn validate_remote_info(remote_info: &RemoteInfo) {
    warn!("TODO: validate_remote_info");
    // let cmd = format!("[[ -d '{}' ]] && echo 'ok' || echo 'missing'", remote_info.path);
    // println!("{}", cmd);
    // let output = remote_info.generate_command(&mut remote_info.base_command(&cmd), &cmd).output().unwrap();
    // println!("{}", String::from_utf8_lossy(&output.stdout).to_mut());
    // output.
    // let base_dir = base_dir.unwrap_or_else(|| panic!("failed to find base directory"));
}
