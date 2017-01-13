use std::process::{Command, Output};
use std::io;
use regex::{Regex, RegexSet};

mod rsync;

pub struct RemoteDirInfo {
    is_remote: bool,
    path: String,
    user: String,
    host: String,
    port: String
}

impl RemoteDirInfo {
    fn run_command(&self, cmd: &str) -> Result<Output, io::Error> {
        if self.is_remote {
            Command::new("ssh").arg("-q").arg(format!("{}@{}", self.user, self.host)).arg("-p").arg(&self.port).arg("-C").arg(cmd).output()
        } else {
            let mut iter = cmd.split_whitespace();
            let main_cmd = iter.next().unwrap();
            let mut args = vec![];
            for arg in iter {
                args.push(arg)
            };

            Command::new(main_cmd).args(&args).output()
        }
    }

    fn full_path(&self) -> String {
        if self.is_remote {
            format!("{}@{}:{}", self.user, self.host, self.path)
        } else {
            self.path.clone()
        }
    }
}

pub fn run(base_dir: &str, remote_dir: &str, port: Option<&str>, ignores: RegexSet) {
    let remote_info = get_remote_info(remote_dir, port);
    rsync::run(base_dir, &remote_info, &ignores)
}

fn get_remote_info(remote_dir: &str, port: Option<&str>) -> RemoteDirInfo {
    let regex = Regex::new("([^@]+)@([^:]+):(.+)").unwrap();
    if let Some(captures) = regex.captures(remote_dir) {
        RemoteDirInfo {
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
        RemoteDirInfo {
            is_remote: false,
            path: remote_dir.to_owned(),
            user: "".to_owned(),
            host: "".to_owned(),
            port: "".to_owned()
        }
    }
}