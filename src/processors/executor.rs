use helpers;
use regex::RegexSet;
use slog::Logger;
use std::io::{Read, Write, BufWriter, BufReader};
use std::marker::Send;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, TryRecvError, Receiver};
use std::thread::{self, sleep};
use std::time::Duration;
use structs::bound_file::{BoundFile, FileAction};
use structs::watcher::BindrsWatcher;
use time;

pub fn start<R: Read + Send + 'static, W: Write + Send + 'static>(
    log: &Logger,
    base_dir: &str,
    ignores: RegexSet,
    reader: R,
    writer: W,
) {
    let lock: Arc<Mutex<Vec<(String, i64, i32)>>> = Arc::new(Mutex::new(vec![]));
    let lock_clone = lock.clone();

    let sync_count: Arc<Mutex<(u32, u32)>> = Arc::new(Mutex::new((0, 0)));

    let base_dir_clone = base_dir.to_owned();
    let log_clone = log.clone();
    let sync_count_clone = sync_count.clone();
    let child_1 = thread::spawn(move || {
        run_local_watcher(
            &log_clone,
            &base_dir_clone,
            &ignores,
            writer,
            lock_clone,
            sync_count_clone,
        );
    });

    let base_dir_clone = base_dir.to_owned();
    let log_clone = log.clone();
    let sync_count_clone = sync_count.clone();
    let child_2 = thread::spawn(move || {
        run_remote_listener(&log_clone, &base_dir_clone, reader, lock, sync_count_clone);
    });

    let log_clone = log.clone();
    let (status_log_tx, status_log_rx) = mpsc::channel();
    let child_3 = thread::spawn(move || {
        run_status_logger(&log_clone, sync_count, &status_log_rx);
    });

    info!(log, "Ready!");

    let _ = child_1.join();
    let _ = child_2.join();
    status_log_tx.send(()).unwrap_or_default();
    let _ = child_3.join();
    info!(log, "BindRS Stopping");
}

fn run_local_watcher<W: Write>(
    log: &Logger,
    base_dir: &str,
    ingores: &RegexSet,
    writer: W,
    lock: Arc<Mutex<Vec<(String, i64, i32)>>>,
    sync_count: Arc<Mutex<(u32, u32)>>,
) {
    let mut writer = BufWriter::new(writer);
    let mut watcher = BindrsWatcher::new(base_dir, ingores);
    watcher.watch(log);
    let rx = watcher.rx.unwrap_or_else(|| {
        helpers::log_error_and_exit(log, "Couldn't get local receive channel off local watcher");
        panic!();
    });

    loop {
        let (a, p) = rx.recv().unwrap_or_else(|e| {
            helpers::log_error_and_exit(
                log,
                &format!("Failed to receive message from local watcher: {}", e),
            );
            panic!(e)
        });
        let full_str_path = format!("{}/{}", base_dir, p);
        let full_path = Path::new(&full_str_path);

        if a == FileAction::CreateUpdate && full_path.is_dir() {
            continue;
        }

        let p_clone = p.clone();

        {
            let mut recent_files = lock.lock().unwrap_or_else(|_| {
                helpers::log_error_and_exit(log, "Failed to aquire local fs lock, lock poisoned");
                panic!()
            });
            let (now_s, now_nano_s) = {
                let now_spec = time::now().to_timespec();
                (now_spec.sec, now_spec.nsec)
            };

            recent_files.retain(|&(_, ref time_s, ref time_nano_s)| {
                if now_s - time_s > 1 {
                    false
                } else if now_s - time_s == 1 {
                    now_nano_s - time_nano_s + 1000000000 /* 1e9 */ < 500000000 // 5e8
                } else {
                    now_nano_s - time_nano_s < 500000000 // 5e8
                }
            });

            if recent_files.iter().map(|&(ref path, _, _)| path).any(
                |&ref path| &p_clone == path,
            )
            {
                continue;
            }
        }

        let _guard = lock.lock().unwrap_or_else(|_| {
            helpers::log_error_and_exit(log, "Failed to aquire local fs lock, lock poisoned");
            panic!()
        });

        if a == FileAction::CreateUpdate && !full_path.exists() {
            debug!(log, "Skipping sending {} as file does not exist", p);
        } else {
            let bf = BoundFile::build_from_path_action(base_dir, p, a);
            debug!(log, "Sending {} to remote", bf.path);
            bf.to_writer(&mut writer);

            {
                let mut synced_nums = sync_count.lock().unwrap_or_else(|_| {
                    helpers::log_error_and_exit(
                        log,
                        "Failed to aquire sync count lock, lock poisoned",
                    );
                    panic!()
                });
                synced_nums.0 += 1;
            }
        }
    }
}

fn run_remote_listener<R: Read>(
    log: &Logger,
    base_dir: &str,
    reader: R,
    lock: Arc<Mutex<Vec<(String, i64, i32)>>>,
    sync_count: Arc<Mutex<(u32, u32)>>,
) {
    let mut reader = BufReader::new(reader);
    loop {
        let bf = BoundFile::from_reader(&mut reader);
        let mut recent_files = lock.lock().unwrap_or_else(|_| {
            helpers::log_error_and_exit(log, "Failed to aquire local fs lock, lock poisoned");
            panic!()
        });
        debug!(log, "Receiving {} from remote", bf.path);
        bf.save_to_disk(base_dir);

        let (now_s, now_nano_s) = {
            let now_spec = time::now().to_timespec();
            (now_spec.sec, now_spec.nsec)
        };
        recent_files.push((bf.path, now_s, now_nano_s));

        {
            let mut synced_nums = sync_count.lock().unwrap_or_else(|_| {
                helpers::log_error_and_exit(log, "Failed to aquire sync count lock, lock poisoned");
                panic!()
            });
            synced_nums.1 += 1;
        }
    }
}

fn run_status_logger(log: &Logger, sync_count: Arc<Mutex<(u32, u32)>>, rx: &Receiver<()>) {
    loop {
        sleep(Duration::from_millis(1000));
        match rx.try_recv() {
            Ok(_) |
            Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => (),
        }

        {
            let mut synced_nums = sync_count.lock().unwrap_or_else(|_| {
                helpers::log_error_and_exit(log, "Failed to aquire sync count lock, lock poisoned");
                panic!()
            });

            let to_log = synced_nums.0 > 0 || synced_nums.1 > 0;
            let mut message: Vec<String> = vec![];

            if synced_nums.0 > 0 {
                message.push(format!("Sent {} file/s.", synced_nums.0));
            }

            if synced_nums.1 > 0 {
                message.push(format!("Received {} file/s.", synced_nums.1));
            }

            if to_log {
                info!(log, "{}", message.join(" "));
                *synced_nums = (0, 0)
            }
        }
    }
}
