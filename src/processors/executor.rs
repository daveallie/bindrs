use chan_signal::{self, Signal};
use helpers;
use regex::RegexSet;
use slog::Logger;
use std::io::{Read, Write, BufWriter, BufReader};
use std::marker::Send;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, TryRecvError, Receiver, Sender};
use std::thread::{self, sleep};
use std::time::Duration;
use structs::bound_file::{BoundFile, FileAction};
use structs::watcher::BindrsWatcher;
use time;

type LocalTx = Sender<Option<(FileAction, String)>>;

pub fn start<R: Read + Send + 'static, W: Write + Send + 'static>(
    log: &Logger,
    base_dir: &str,
    ignores: RegexSet,
    reader: R,
    writer: W,
) {
    let lock: Arc<Mutex<Vec<(String, i64, i32)>>> = Arc::new(Mutex::new(vec![]));
    let sync_count: Arc<Mutex<(u32, u32)>> = Arc::new(Mutex::new((0, 0)));
    let local_watcher_kill_tx: Arc<Mutex<Option<LocalTx>>> = Arc::new(Mutex::new(None));

    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);

    let base_dir_clone = base_dir.to_owned();
    let log_clone = log.clone();
    let lock_clone = lock.clone();
    let sync_count_clone = sync_count.clone();
    let local_watcher_kill_tx_clone = local_watcher_kill_tx.clone();
    let child_1 = thread::spawn(move || {
        run_local_watcher(
            &log_clone,
            &base_dir_clone,
            &ignores,
            writer,
            lock_clone,
            sync_count_clone,
            local_watcher_kill_tx_clone,
        );
    });

    let base_dir_clone = base_dir.to_owned();
    let log_clone = log.clone();
    let sync_count_clone = sync_count.clone();
    let local_watcher_kill_tx_clone = local_watcher_kill_tx.clone();
    let child_2 = thread::spawn(move || {
        run_remote_listener(
            &log_clone,
            &base_dir_clone,
            reader,
            lock,
            sync_count_clone,
            local_watcher_kill_tx_clone,
        );
    });

    let log_clone = log.clone();
    let (status_log_tx, status_log_rx) = mpsc::channel();
    let child_3 = thread::spawn(move || {
        run_status_logger(&log_clone, sync_count, &status_log_rx);
    });

    info!(log, "Ready!");

    // master local watcher gets None through receiver after interrupt signal comes through.
    // master local watcher sends 0 bytes to slave remote watcher and closes.
    // slave remote watcher sends None to slave local watcher and closes.
    // save local watcher sends 0 bytes to master remote watcher and closes.
    // master remote watcher tries to send 0 to master local watcher (which is no longer open),
    //   and closes
    // Both programs have ended.

    let log_clone = log.clone();
    let local_watcher_kill_tx_clone = local_watcher_kill_tx.clone();
    thread::spawn(move || {
        signal.recv();
        send_local_watch_kill_if_possible(&log_clone, local_watcher_kill_tx_clone);
        status_log_tx.send(()).unwrap_or_default();
    });

    let _ = child_1.join();
    let _ = child_2.join();
    let _ = child_3.join();
    info!(log, "BindRS Stopping");
}

fn run_local_watcher<W: Write>(
    log: &Logger,
    base_dir: &str,
    ignores: &RegexSet,
    writer: W,
    lock: Arc<Mutex<Vec<(String, i64, i32)>>>,
    sync_count: Arc<Mutex<(u32, u32)>>,
    local_watcher_kill_tx: Arc<Mutex<Option<LocalTx>>>,
) {
    let mut writer = BufWriter::new(writer);
    let mut watcher = BindrsWatcher::new(base_dir, ignores);
    watcher.watch(log);
    let rx = watcher.rx.unwrap_or_else(|| {
        helpers::log_error_and_exit(log, "Couldn't get rx channel off local watcher");
        panic!();
    });

    {
        let mut tx = local_watcher_kill_tx.lock().unwrap_or_else(|_| {
            helpers::log_error_and_exit(log, "Failed to acquire local fs lock, lock poisoned");
            panic!()
        });
        *tx = watcher.tx;
    }

    loop {
        let option = rx.recv().unwrap_or_else(|e| {
            helpers::log_error_and_exit(
                log,
                &format!("Failed to receive message from local watcher: {}", e),
            );
            panic!(e)
        });

        // None is passed when program is exiting
        if option.is_none() {
            // Send 0 length data to signal end of program
            helpers::write_content(log, &mut writer, &[0; 0]);

            let mut tx = local_watcher_kill_tx.lock().unwrap_or_else(|_| {
                helpers::log_error_and_exit(log, "Failed to acquire local fs lock, lock poisoned");
                panic!()
            });
            *tx = None;

            break;
        }

        // Safe, just check if it's none
        let (a, p) = option.expect("Failed to unwrap local watcher result");

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
            bf.to_writer(log, &mut writer);

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
    local_watcher_kill_tx: Arc<Mutex<Option<LocalTx>>>,
) {
    let mut reader = BufReader::new(reader);
    loop {
        let bf = BoundFile::from_reader(log, &mut reader);

        if bf.is_none() {
            send_local_watch_kill_if_possible(log, local_watcher_kill_tx);
            break;
        }

        // Safe, just check if it's none
        let bf = bf.expect("Could not unwrap bound file from remote");

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

fn send_local_watch_kill_if_possible(
    log: &Logger,
    local_watcher_kill_tx: Arc<Mutex<Option<LocalTx>>>,
) {
    let mut tx = local_watcher_kill_tx.lock().unwrap_or_else(|_| {
        helpers::log_error_and_exit(log, "Failed to acquire local fs lock, lock poisoned");
        panic!()
    });

    if tx.is_some() {
        let tx2 = tx.take().expect("Can't unwrap present option");
        match tx2.send(None) {
            _ => (),
        }
    }
}
