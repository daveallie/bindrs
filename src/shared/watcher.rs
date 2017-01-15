use regex::RegexSet;
use notify::{DebouncedEvent, RecommendedWatcher, Watcher, RecursiveMode, watcher};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use std::thread::{self, JoinHandle};
use std::sync::mpsc::TryRecvError;

pub struct BindrsWatcher {
    pub rx: Receiver<(u8, String)>,
    final_tx: Sender<(u8, String)>,
    watch_loop_rx: Receiver<u8>,
    watch_loop_tx: Sender<u8>,
    dir: String,
    ignores: RegexSet,
    notify_rx: Receiver<DebouncedEvent>,
    watcher: RecommendedWatcher,
    thread: Option<JoinHandle<()>>
}

impl BindrsWatcher {
    pub fn new(base_dir: &str, ignores: &RegexSet) -> BindrsWatcher {
        let (tx, notify_rx) = channel();
        let (final_tx, rx) = channel();
        let (watch_loop_tx, watch_loop_rx) = channel();

        BindrsWatcher {
            rx: rx,
            final_tx: final_tx,
            watch_loop_tx: watch_loop_tx,
            watch_loop_rx: watch_loop_rx,
            dir: base_dir.to_owned(),
            ignores: ignores.to_owned(),
            notify_rx: notify_rx,
            watcher: watcher(tx, Duration::from_millis(200)).unwrap(),
            thread: None
        }
    }

    pub fn watch(&mut self) {
        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        self.watcher.watch(&self.dir, RecursiveMode::Recursive).unwrap();

        self.thread = Some(thread::spawn(move || {
            let dir_length = self.dir.len();
            loop {
                let event = self.notify_rx.recv().unwrap_or_else(|e| panic!("watch error: {:?}", e));
                match self.watch_loop_rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => ()
                };
                let actions = match event {
                    DebouncedEvent::Create(p) |
                    DebouncedEvent::Write(p) => vec![(0, p)],
                    DebouncedEvent::Remove(p) => vec![(1, p)],
                    DebouncedEvent::Rename(p1, p2) => vec![(1, p1), (0, p2)],
                    _ => vec![]
                };

                let filtered_actions = actions.into_iter().map(|(ref t, ref p)| {
                    let short_path: String = p.to_str().unwrap().chars().skip(dir_length).collect();
                    (*t as u8, short_path)
                }).filter(|&(_, ref short_path)| !self.ignores.is_match(&short_path));

                for (ref t, ref p) in filtered_actions {
                    let _ = self.final_tx.send((*t, p.to_owned()));
                }
            }
        }));
    }

    pub fn unwatch(&mut self) {
        if  self.thread.is_some() {
            self.watch_loop_tx.send(1);
            self.thread = None;
        }

        self.watcher.unwatch(&self.dir).unwrap();
    }
}
