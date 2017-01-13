use regex::{Regex, RegexSet};

pub mod remote_info;
mod rsync;

use self::remote_info::RemoteInfo;

pub fn run(base_dir: &str, remote_dir: &str, port: Option<&str>, ignores: RegexSet) {
    let remote_info = get_remote_info(remote_dir, port);
    rsync::run(base_dir, &remote_info, &ignores)
}

fn get_remote_info(remote_dir: &str, port: Option<&str>) -> RemoteInfo {
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
