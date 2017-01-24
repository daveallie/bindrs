use notify::{DebouncedEvent, RecommendedWatcher, Watcher, RecursiveMode, watcher};
use regex::RegexSet;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct BindrsWatcher {
    pub rx: Option<Receiver<(u8, String)>>,
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
            let dir_length = self.dir.len();
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
                    DebouncedEvent::Write(p) => vec![(0, p)],
                    DebouncedEvent::Remove(p) => vec![(1, p)],
                    DebouncedEvent::Rename(p1, p2) => vec![(1, p1), (0, p2)],
                    _ => vec![],
                };

                let filtered_actions = actions.into_iter()
                    .map(|(ref t, ref p)| {
                        let short_path: String =
                            p.to_str().unwrap().chars().skip(dir_length).collect();
                        (*t as u8, short_path)
                    })
                    .filter(|&(_, ref short_path)| !ignores.is_match(&short_path));

                for (ref t, ref p) in filtered_actions {
                    let _ = final_tx.send((*t, p.to_owned()));
                }
            }))
        };
    }

    pub fn unwatch(&mut self) {
        if self.watcher.is_some() && self.thread.is_some() && self.watch_loop_tx.is_some() {
            let _ = self.watch_loop_tx.clone().unwrap().send(1);
            self.thread = None;
            self.watcher.take().unwrap().unwatch(&self.dir).unwrap();
            self.watcher = None;
        }
    }
}
