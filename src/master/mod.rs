use self::remote_info::RemoteInfo;
use super::shared::{helpers, executor};
use slog::Logger;
use std::{time, thread};
use std::process::{ChildStdout, ChildStdin};
use std::process::Stdio;

pub mod remote_info;
mod rsync;

pub fn run(log: &Logger,
           base_dir: &str,
           remote_dir: &str,
           port: Option<&str>,
           ignore_strings: &mut Vec<String>) {
    let ignores = helpers::process_ignores(ignore_strings);
    let remote_info = RemoteInfo::build(remote_dir, port);

    validate_remote_info(&log, &remote_info);
    rsync::run(&log, &base_dir, &remote_info, &ignores);
    let (remote_reader, remote_writer) = start_remote_slave(&log, &remote_info, &ignore_strings);
    executor::start(&log,
                    base_dir.to_owned(),
                    ignores,
                    remote_reader,
                    remote_writer);
}

fn start_remote_slave(log: &Logger,
                      remote_info: &RemoteInfo,
                      ignores: &Vec<String>)
                      -> (ChildStdout, ChildStdin) {
    info!(log, "Starting remote slave");
    let ignore_vec: Vec<String> = ignores.iter().map(|i| format!("--ignore \"{}\"", i)).collect();
    let cmd = format!("bindrs slave {} {}", remote_info.path, ignore_vec.join(" "));

    let mut child = remote_info.generate_command(&mut remote_info.base_command(&cmd), &cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    thread::sleep(time::Duration::new(1, 0));
    let c_stdout = child.stdout.take().unwrap();
    let c_stdin = child.stdin.take().unwrap();

    (c_stdout, c_stdin)
}

fn validate_remote_info(log: &Logger, remote_info: &RemoteInfo) {
    let cmd = "which bindrs 2>/dev/null";
    if get_cmd_output(remote_info, &cmd) == "" {
        helpers::log_error_and_exit(&log,
                                    "Please install BindRS on the remote machine and add it to \
                                     the path");
    }

    let cmd = "bindrs -V";
    if get_cmd_output(remote_info, &cmd) != format!("BindRS {}", ::VERSION) {
        helpers::log_error_and_exit(&log,
                                    "Please make sure your local and remote versions of BindRS \
                                     match");
    }
}

fn get_cmd_output(remote_info: &RemoteInfo, cmd: &str) -> String {
    let output =
        remote_info.generate_command(&mut remote_info.base_command(&cmd), &cmd).output().unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}
