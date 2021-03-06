use helpers;
use processors::{executor, rsync};
use slog::Logger;
use std::{time, thread};
use std::process::{Stdio, ChildStdout, ChildStdin};
use structs::remote_info::RemoteInfo;

pub fn run(
    log: &Logger,
    base_dir: &str,
    remote_dir: &str,
    port: Option<&str>,
    ignore_strings: &mut Vec<String>,
    verbose_mode: bool,
) {
    let ignores = helpers::process_ignores(log, ignore_strings);
    let remote_info = RemoteInfo::build(remote_dir, port);

    validate_remote_directory(log, &remote_info);
    let bindrs_path = validate_remote_bindrs(log, &remote_info, false);
    rsync::run(log, base_dir, &remote_info, &ignores);
    let (remote_reader, remote_writer) = start_remote_slave(
        log,
        &remote_info,
        &bindrs_path,
        ignore_strings,
        verbose_mode,
    );
    executor::start(log, base_dir, ignores, remote_reader, remote_writer);
}

fn start_remote_slave(
    log: &Logger,
    remote_info: &RemoteInfo,
    bindrs_path: &str,
    ignores: &mut Vec<String>,
    verbose_mode: bool,
) -> (ChildStdout, ChildStdin) {
    info!(log, "Starting remote slave");
    let ignore_args: Vec<String> = ignores
        .iter()
        .map(|i| format!("--ignore \"{}\"", i))
        .collect();

    let mut cmd = format!(
        "{} slave {} {}",
        bindrs_path,
        remote_info.path,
        ignore_args.join(" ")
    );

    if verbose_mode {
        cmd += " -v"
    }

    if let Ok(mut child) = remote_info
        .generate_command(&mut remote_info.base_command(&cmd), &cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        thread::sleep(time::Duration::new(1, 0));
        #[cfg_attr(feature="clippy", allow(option_unwrap_used))]
        let c_stdout = child.stdout.take().unwrap(); // Unwrap is safe - provided in child spawn
        #[cfg_attr(feature="clippy", allow(option_unwrap_used))]
        let c_stdin = child.stdin.take().unwrap(); // Unwrap is safe - provided in child spawn

        (c_stdout, c_stdin)
    } else {
        helpers::log_error_and_exit(log, "Failed to spawn a child");
        panic!(); // For compilation
    }
}

fn validate_remote_directory(log: &Logger, remote_info: &RemoteInfo) {
    log_error_and_exit_on_bad_command_output(
        log,
        remote_info,
        &format!("test -d {} || echo 'bad'", remote_info.path),
        &["bad".to_string()],
        false,
        "Remote directory does not exist, please create it",
    );
}

fn validate_remote_bindrs(log: &Logger, remote_info: &RemoteInfo, download_attempted: bool) -> String {
    let bindrs_path = match remote_info.check_cmd_output(
        log,
        "which bindrs",
        &["bindrs not found".to_string(), "".to_string()],
        false,
    ) {
        Ok(path) => path,
        Err(_) => {
            if let Ok(path) = remote_info.check_cmd_output(
                log,
                &format!("PATH={}/.bindrs:$PATH which bindrs", remote_info.path),
                &["bindrs not found".to_string(), "".to_string()],
                false,
            )
            {
                path
            } else {
                if !download_attempted {
                    warn!(
                        log,
                        "BindRS missing on remote, attempting to download to .bindrs dir"
                    );
                    if helpers::download_bindrs(log, remote_info) {
                        return validate_remote_bindrs(log, remote_info, true);
                    }
                }

                helpers::log_error_and_exit(
                    log,
                    "Please install BindRS on the remote machine and add it to the path",
                );
                panic!() // For compilation
            }
        }
    };

    match remote_info.get_cmd_output(&format!("{} --version", bindrs_path)) {
        Ok(mut output) => helpers::compare_version_strings(log, ::VERSION, &output.split_off(7)),
        Err(e) => {
            helpers::log_error_and_exit(
                log,
                &format!("Failed to get BindRS version from remote: {}", e),
            )
        }
    };

    bindrs_path
}

fn log_error_and_exit_on_bad_command_output(
    log: &Logger,
    remote_info: &RemoteInfo,
    cmd: &str,
    wanted_output: &[String],
    match_output: bool,
    bad_output_error: &str,
) {
    match remote_info.check_cmd_output(log, cmd, wanted_output, match_output) {
        Ok(_) => (),
        Err(_) => helpers::log_error_and_exit(log, bad_output_error),
    }
}
