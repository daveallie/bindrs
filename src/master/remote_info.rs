use std::io;
use std::process::{Command, Output};

pub struct RemoteInfo {
    pub is_remote: bool,
    pub path: String,
    pub user: String,
    pub host: String,
    pub port: String
}

impl RemoteInfo {
    pub fn run_command(&self, cmd: &str) -> Result<Output, io::Error> {
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

    pub fn full_path(&self) -> String {
        if self.is_remote {
            format!("{}@{}:{}", self.user, self.host, self.path)
        } else {
            self.path.clone()
        }
    }
}
