use helpers;
use regex::Regex;
use slog::Logger;
use std::io;
use std::process::Command;

pub struct RemoteInfo {
    pub is_remote: bool,
    pub path: String,
    pub user: String,
    pub host: String,
    pub port: String,
}

impl RemoteInfo {
    pub fn build(remote_dir: &str, port: Option<&str>) -> Self {
        #[cfg_attr(feature="clippy", allow(result_unwrap_used))]
        // Unwrap is safe - hard coded string
        let regex = Regex::new("([^@]+)@([^:]+):(.+)").unwrap();
        if let Some(captures) = regex.captures(remote_dir) {
            Self {
                is_remote: true,
                // Unwrap is safe - capture group exists in regex
                path: captures.get(3).unwrap().as_str().to_owned(),
                user: captures.get(1).unwrap().as_str().to_owned(),
                host: captures.get(2).unwrap().as_str().to_owned(),
                port: match port {
                    Some(p) => p.to_owned(),
                    None => "22".to_owned(),
                },
            }
        } else {
            Self {
                is_remote: false,
                path: remote_dir.to_owned(),
                user: "".to_owned(),
                host: "".to_owned(),
                port: "".to_owned(),
            }
        }
    }

    pub fn base_command(&self, cmd: &str) -> Command {
        if self.is_remote {
            Command::new("ssh")
        } else {
            let mut iter = cmd.split_whitespace();
            let main_cmd = iter.next().unwrap_or("");

            Command::new(main_cmd)
        }
    }

    pub fn generate_command<'a>(&self, command: &'a mut Command, cmd: &str) -> &'a mut Command {
        if self.is_remote {
            command
                .arg("-q")
                .arg(format!("{}@{}", self.user, self.host))
                .arg("-p")
                .arg(&self.port)
                .arg("-C")
                .arg(cmd)
        } else {
            let iter = cmd.split_whitespace();
            let mut args = vec![];
            for arg in iter.skip(1) {
                args.push(arg)
            }

            command.args(&args)
        }
    }

    pub fn full_path(&self) -> String {
        if self.is_remote {
            format!("{}@{}:{}", self.user, self.host, self.path)
        } else {
            self.path.clone()
        }
    }

    pub fn full_path_trailing_slash(&self) -> String {
        format!("{}/", self.full_path())
    }

    pub fn check_cmd_output(
        &self,
        log: &Logger,
        cmd: &str,
        wanted_output: &[String],
        match_output: bool,
    ) -> Result<String, ()> {
        match self.get_cmd_output(cmd) {
            Ok(output) => {
                if match_output ^ wanted_output.contains(&output) {
                    Err(())
                } else {
                    Ok(output)
                }
            }
            Err(e) => {
                helpers::log_error_and_exit(log, &format!("Failed to run '{}' on remote: {}", cmd, e));
                panic!(); // For compilation
            }
        }
    }

    pub fn get_cmd_output(&self, cmd: &str) -> Result<String, io::Error> {
        let output = self.generate_command(&mut self.base_command(cmd), cmd)
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }



    pub fn run_cmd(&self, cmd: &str) -> bool {
        match self.generate_command(&mut self.base_command(cmd), cmd)
            .status() {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    }
}
