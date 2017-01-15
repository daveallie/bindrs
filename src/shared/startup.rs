use regex::RegexSet;
use std::thread;
use std::sync::mpsc::channel;

pub fn startup_common(base_dir: &str, ignores: &RegexSet) {
    let mut watcher = super::watcher::BindrsWatcher::new(base_dir, ignores);
    watcher.watch();
}
