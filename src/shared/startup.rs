use regex::RegexSet;
use std::thread;

pub fn startup_common(base_dir: String, ignores: RegexSet) {
    thread::spawn(move || {
        let mut watcher = super::watcher::BindrsWatcher::new(&base_dir, &ignores);
        watcher.watch();
        let rx = watcher.rx.unwrap();
        loop {
            match rx.recv() {
                Ok((t, p)) => println!("{} => {}", t, p),
                Err(e) => panic!("Error: {}", e)
            }
        }
    });
}
