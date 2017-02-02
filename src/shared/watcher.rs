use notify::{DebouncedEvent, RecommendedWatcher, Watcher, RecursiveMode, watcher};
use regex::RegexSet;
use shared::bound_file::FileAction;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

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

    pub fn watch(&mut self) {
        let (final_tx, final_rx) = channel();
        let (notify_tx, notify_rx) = channel();
        let (watch_loop_tx, watch_loop_rx) = channel();

        let mut watcher = watcher(notify_tx, Duration::from_millis(200)).unwrap();
        watcher.watch(&self.dir, RecursiveMode::Recursive).unwrap();
        self.watcher = Some(watcher);
        self.watch_loop_tx = Some(watch_loop_tx);
        self.rx = Some(final_rx);

        self.thread = {
            let dir_length = self.dir.len() + 1;
            let ignores = self.ignores.clone();
            Some(thread::spawn(move || loop {
                let event = notify_rx.recv().unwrap_or_else(|e| panic!("watch error: {:?}", e));
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

                let filtered_actions = actions.into_iter()
                    .map(|(t, p)| {
                        let short_path: String =
                            p.to_str().unwrap().chars().skip(dir_length).collect();
                        (t, short_path)
                    })
                    .filter(|&(_, ref short_path)| !ignores.is_match(&short_path));

                for (t, p) in filtered_actions {
                    let _ = final_tx.send((t, p.to_owned()));
                }
            }))
        };
    }
}
