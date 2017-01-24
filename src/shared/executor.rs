use super::bound_file::BoundFile;
use super::watcher::BindrsWatcher;
use regex::RegexSet;
use std::io::{Read, Write, BufWriter, BufReader};
use std::marker::Send;
use std::sync::{Arc, Mutex};
use std::thread;

pub fn start<R: Read + Send + 'static, W: Write + Send + 'static>(base_dir: String,
                                                                  ignores: RegexSet,
                                                                  reader: R,
                                                                  writer: W) {
    let lock = Arc::new(Mutex::new(0));
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
                               lock: Arc<Mutex<u8>>) {
    let mut writer = BufWriter::new(writer);
    let mut watcher = BindrsWatcher::new(&base_dir, &ingores);
    watcher.watch();
    let rx = watcher.rx.unwrap();
    loop {
        let (a, p) = rx.recv().unwrap();
        let bf = BoundFile::build_from_path_action(&base_dir, p, a);
        let _guard = lock.lock().unwrap();
        bf.to_writer(&mut writer);
    }
}

fn run_remote_listener<R: Read>(base_dir: String, reader: R, lock: Arc<Mutex<u8>>) {
    let mut reader = BufReader::new(reader);
    loop {
        let bf = BoundFile::from_reader(&mut reader);
        let _guard = lock.lock().unwrap();
        bf.save_to_disk(&base_dir);
    }
}
