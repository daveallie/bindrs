use regex::RegexSet;

pub fn startup_common(base_dir: &str, ignores: &RegexSet) {
    let mut watcher = super::watcher::BindrsWatcher::new(base_dir, ignores);
    watcher.watch();
}
