use helpers;
use notify::{DebouncedEvent, RecommendedWatcher, Watcher, RecursiveMode, watcher};
use regex::RegexSet;
use slog::Logger;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use structs::bound_file::FileAction;

#[cfg_attr(feature = "clippy", allow(stutter))]
pub struct BindrsWatcher {
    pub rx: Option<Receiver<(FileAction, String)>>,
    dir: String,
    ignores: RegexSet,
    watcher: Option<RecommendedWatcher>,
    watch_loop_tx: Option<Sender<u8>>,
    thread: Option<JoinHandle<()>>,
}

impl BindrsWatcher {
    pub fn new(base_dir: &str, ignores: &RegexSet) -> BindrsWatcher {
        BindrsWatcher {
            rx: None,
            watch_loop_tx: None,
            dir: base_dir.to_owned(),
            ignores: ignores.to_owned(),
            watcher: None,
            thread: None,
        }
    }

    pub fn watch(&mut self, log: &Logger) {
        let (final_tx, final_rx) = channel();
        let (notify_tx, notify_rx) = channel();
        let (watch_loop_tx, watch_loop_rx) = channel();

        let mut watcher = match watcher(notify_tx, Duration::from_millis(200)) {
            Ok(w) => w,
            Err(e) => {
                helpers::log_error_and_exit(log, &format!("Failed to create watcher: {}", e));
                panic!(e);
            }
        };
        watcher
            .watch(&self.dir, RecursiveMode::Recursive)
            .unwrap_or_else(|e| {
                helpers::log_error_and_exit(log, &format!("Failed to watch {}: {}", self.dir, e))
            });
        self.watcher = Some(watcher);
        self.watch_loop_tx = Some(watch_loop_tx);
        self.rx = Some(final_rx);

        self.thread = {
            let dir_length = self.dir.len() + 1;
            let ignores = self.ignores.clone();
            let log_clone = log.clone();
            Some(thread::spawn(move || loop {
                let event = notify_rx.recv().unwrap_or_else(|e| {
                    helpers::log_error_and_exit(&log_clone, &format!("watch error: {}", e));
                    panic!(e);
                });
                match watch_loop_rx.try_recv() {
                    Ok(_) |
                    Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => (),
                };
                let actions = match event {
                    DebouncedEvent::Create(p) |
                    DebouncedEvent::Write(p) => vec![(FileAction::CreateUpdate, p)],
                    DebouncedEvent::Remove(p) => vec![(FileAction::Delete, p)],
                    DebouncedEvent::Rename(p1, p2) => {
                        vec![(FileAction::Delete, p1), (FileAction::CreateUpdate, p2)]
                    }
                    _ => vec![],
                };

                let filtered_actions =
                    actions
                        .into_iter()
                        .filter_map(|(t, p)| match p.to_str() {
                            Some(path) => {
                                let short_path: String = path.chars().skip(dir_length).collect();
                                Some((t, short_path))
                            }
                            None => None,
                        })
                        .filter(|&(_, ref short_path)| !ignores.is_match(short_path));

                for (t, p) in filtered_actions {
                    let _ = final_tx.send((t, p.to_owned()));
                }
            }))
        };
    }
}
