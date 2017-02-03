use helpers;
use processors::{executor, rsync};
use slog::Logger;
use std::{time, thread, io};
use std::process::{Stdio, ChildStdout, ChildStdin};
use structs::remote_info::RemoteInfo;

pub fn run(log: &Logger,
           base_dir: &str,
           remote_dir: &str,
           port: Option<&str>,
           ignore_strings: &mut Vec<String>,
           verbose_mode: bool) {
    let ignores = helpers::process_ignores(log, ignore_strings);
    let remote_info = RemoteInfo::build(remote_dir, port);

    validate_remote_info(&log, &remote_info);
    rsync::run(&log, &base_dir, &remote_info, &ignores);
    let (remote_reader, remote_writer) =
        start_remote_slave(&log, &remote_info, &ignore_strings, verbose_mode);
    executor::start(&log,
                    base_dir.to_owned(),
                    ignores,
                    remote_reader,
                    remote_writer);
}

fn start_remote_slave(log: &Logger,
                      remote_info: &RemoteInfo,
                      ignores: &Vec<String>,
                      verbose_mode: bool)
                      -> (ChildStdout, ChildStdin) {
    info!(log, "Starting remote slave");
    let ignore_vec: Vec<String> = ignores.iter().map(|i| format!("--ignore \"{}\"", i)).collect();
    let mut cmd = format!("bindrs slave {} {}", remote_info.path, ignore_vec.join(" "));

    if verbose_mode {
        cmd += " -v"
    }

    let mut child = match remote_info.generate_command(&mut remote_info.base_command(&cmd), &cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn() {
        Ok(c) => c,
        Err(_) => {
            helpers::log_error_and_exit(log, "Failed to spawn a child");
            panic!(); // For compilation
        }
    };

    thread::sleep(time::Duration::new(1, 0));
    let c_stdout = child.stdout.take().unwrap(); // Unwrap is safe - provided in child spawn
    let c_stdin = child.stdin.take().unwrap(); // Unwrap is safe - provided in child spawn

    (c_stdout, c_stdin)
}

fn validate_remote_info(log: &Logger, remote_info: &RemoteInfo) {
    check_cmd_output(log,
                     remote_info,
                     "which bindrs",
                     "bindrs not found",
                     false,
                     "Please install BindRS on the remote machine and add it to the path");

    check_cmd_output(log,
                     remote_info,
                     "bindrs -V",
                     &format!("BindRS {}", ::VERSION),
                     true,
                     "Please make sure your local and remote versions of BindRS match");

    check_cmd_output(log,
                     remote_info,
                     &format!("test -d {} || echo 'bad'", remote_info.path),
                     "bad",
                     false,
                     "Remote directory does not exist, please create it");
}

fn check_cmd_output(log: &Logger,
                    remote_info: &RemoteInfo,
                    cmd: &str,
                    wanted_output: &str,
                    match_output: bool,
                    bad_output_error: &str) {
    match get_cmd_output(remote_info, cmd) {
        Ok(output) => {
            if match_output ^ (output == wanted_output) {
                helpers::log_error_and_exit(&log, bad_output_error);
            }
        }
        Err(_) => helpers::log_error_and_exit(&log, &format!("Failed to run '{}' on remote", cmd)),
    }
}

fn get_cmd_output(remote_info: &RemoteInfo, cmd: &str) -> Result<String, io::Error> {
    let output = try!(remote_info.generate_command(&mut remote_info.base_command(&cmd), &cmd)
        .output());
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
