use super::bound_file::BoundFile;
use super::watcher::BindrsWatcher;
use regex::RegexSet;
use std::io::{Read, Write, BufWriter, BufReader};
use std::marker::Send;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use time;

pub fn start<R: Read + Send + 'static, W: Write + Send + 'static>(base_dir: String,
                                                                  ignores: RegexSet,
                                                                  reader: R,
                                                                  writer: W) {
    let lock: Arc<Mutex<Vec<(String, i64, i32)>>> = Arc::new(Mutex::new(vec![]));
    let lock_clone = lock.clone();

    let base_dir_clone = base_dir.clone();
    let child_1 =
        thread::spawn(move || { run_local_watcher(base_dir_clone, ignores, writer, lock_clone); });

    let base_dir_clone = base_dir.clone();
    let child_2 = thread::spawn(move || { run_remote_listener(base_dir_clone, reader, lock); });

    let _ = child_1.join().unwrap();
    let _ = child_2.join().unwrap();
}

fn run_local_watcher<W: Write>(base_dir: String,
                               ingores: RegexSet,
                               writer: W,
                               lock: Arc<Mutex<Vec<(String, i64, i32)>>>) {
    let mut writer = BufWriter::new(writer);
    let mut watcher = BindrsWatcher::new(&base_dir, &ingores);
    watcher.watch();
    let rx = watcher.rx.unwrap();
    loop {
        let (a, p) = rx.recv().unwrap();
        let full_str_path = format!("{}{}", base_dir, p);
        let full_path = Path::new(&full_str_path);
        let file_exists = full_path.exists();

        if file_exists && full_path.is_dir() {
            continue;
        }

        let p_clone = p.clone();

        {
            let mut recent_files = lock.lock().unwrap();
            let (now_s, now_ns) = {
                let now_spec = time::now().to_timespec();
                (now_spec.sec, now_spec.nsec)
            };

            recent_files.retain(|&(_, ref time_s, ref time_ns)| if now_s - time_s > 1 {
                false
            } else if now_s - time_s == 1 {
                now_ns - time_ns + 1000000000 /* 1e9 */ < 500000000 // 5e8
            } else {
                now_ns - time_ns < 500000000 // 5e8
            });

            if recent_files.iter().map(|&(ref path, _, _)| path).any(|&ref path| &p_clone == path) {
                continue;
            }
        }

        let bf = BoundFile::build_from_path_action(&base_dir, p, a);
        let _guard = lock.lock().unwrap();
        debug!("PROCESSING FILE CHANGE AT: {}", base_dir);
        bf.to_writer(&mut writer);
    }
}

fn run_remote_listener<R: Read>(base_dir: String,
                                reader: R,
                                lock: Arc<Mutex<Vec<(String, i64, i32)>>>) {
    let mut reader = BufReader::new(reader);
    loop {
        let bf = BoundFile::from_reader(&mut reader);
        let mut recent_files = lock.lock().unwrap();
        bf.save_to_disk(&base_dir);

        let (now_s, now_ns) = {
            let now_spec = time::now().to_timespec();
            (now_spec.sec, now_spec.nsec)
        };
        recent_files.push((bf.path, now_s, now_ns));
    }
}
