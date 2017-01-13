use std::process::Command;
use regex::Regex;

pub struct RemoteInfo {
    pub is_remote: bool,
    pub path: String,
    pub user: String,
    pub host: String,
    pub port: String
}

impl RemoteInfo {
    pub fn build(remote_dir: &str, port: Option<&str>) -> RemoteInfo {
        let regex = Regex::new("([^@]+)@([^:]+):(.+)").unwrap();
        if let Some(captures) = regex.captures(remote_dir) {
            RemoteInfo {
                is_remote: true,
                path: captures.get(3).unwrap().as_str().to_owned(),
                user: captures.get(1).unwrap().as_str().to_owned(),
                host: captures.get(2).unwrap().as_str().to_owned(),
                port: match port {
                    Some(p) => p.to_owned(),
                    None => "22".to_owned()
                }
            }
        } else {
            RemoteInfo {
                is_remote: false,
                path: remote_dir.to_owned(),
                user: "".to_owned(),
                host: "".to_owned(),
                port: "".to_owned()
            }
        }
    }

    pub fn base_command(&self, cmd: &str) -> Command {
        if self.is_remote {
            Command::new("ssh")
        } else {
            let mut iter = cmd.split_whitespace();
            let main_cmd = iter.next().unwrap();

            Command::new(main_cmd)
        }
    }

    pub fn generate_command<'a>(&self, command: &'a mut Command, cmd: &str) -> &'a mut Command {
        if self.is_remote {
            command.arg("-q").arg(format!("{}@{}", self.user, self.host)).arg("-p").arg(&self.port).arg("-C").arg(cmd)
        } else {
            let iter = cmd.split_whitespace();
            let mut args = vec![];
            for arg in iter.skip(1) {
                args.push(arg)
            };

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
}