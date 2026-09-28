//! A small pool of worker threads for loading pictures in the background:
//! the threads share one queue, so a long deck never starts a thread per
//! picture, and the slide on screen can jump the queue.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

/// Keys waiting for a worker, and whether the pool has been dropped.
struct Queue<K> {
    keys: VecDeque<K>,
    closed: bool,
}

struct Shared<K> {
    queue: Mutex<Queue<K>>,
    wake: Condvar,
}

/// Up to `workers` threads running `work` on queued keys, first in, first
/// out (urgent keys first). Threads start as work arrives and stop when
/// the pool is dropped.
pub struct Pool<K> {
    shared: Arc<Shared<K>>,
    work: Arc<dyn Fn(K) + Send + Sync>,
    workers: usize,
    started: usize,
}

impl<K: PartialEq + Send + 'static> Pool<K> {
    pub fn new(workers: usize, work: impl Fn(K) + Send + Sync + 'static) -> Self {
        Self {
            shared: Arc::new(Shared {
                queue: Mutex::new(Queue {
                    keys: VecDeque::new(),
                    closed: false,
                }),
                wake: Condvar::new(),
            }),
            work: Arc::new(work),
            workers: workers.max(1),
            started: 0,
        }
    }

    /// Queue `key`; `urgent` puts it at the front.
    pub fn push(&mut self, key: K, urgent: bool) {
        let Ok(mut q) = self.shared.queue.lock() else {
            return;
        };
        if urgent {
            q.keys.push_front(key);
        } else {
            q.keys.push_back(key);
        }
        let waiting = q.keys.len();
        drop(q);
        self.shared.wake.notify_one();
        if self.started < self.workers && waiting > 0 {
            self.spawn();
        }
    }

    /// Move `key` to the front if it is still waiting.
    pub fn hurry(&self, key: &K) {
        let Ok(mut q) = self.shared.queue.lock() else {
            return;
        };
        if let Some(i) = q.keys.iter().position(|k| k == key)
            && i > 0
            && let Some(k) = q.keys.remove(i)
        {
            q.keys.push_front(k);
        }
    }

    fn spawn(&mut self) {
        let shared = self.shared.clone();
        let work = self.work.clone();
        self.started += 1;
        std::thread::spawn(move || {
            while let Some(key) = next(&shared) {
                work(key);
            }
        });
    }
}

/// The next key to work on, waiting for one; `None` once the pool is gone.
fn next<K>(shared: &Shared<K>) -> Option<K> {
    let mut q = shared.queue.lock().ok()?;
    loop {
        if q.closed {
            return None;
        }
        if let Some(k) = q.keys.pop_front() {
            return Some(k);
        }
        q = shared.wake.wait(q).ok()?;
    }
}

impl<K> Drop for Pool<K> {
    fn drop(&mut self) {
        if let Ok(mut q) = self.shared.queue.lock() {
            q.closed = true;
            q.keys.clear();
        }
        self.shared.wake.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    #[test]
    fn every_key_is_worked_on_by_at_most_the_pool_s_threads() {
        let done = Arc::new(Mutex::new(Vec::new()));
        let running = Arc::new(AtomicUsize::new(0));
        let most = Arc::new(AtomicUsize::new(0));
        let mut pool = {
            let (done, running, most) = (done.clone(), running.clone(), most.clone());
            Pool::new(3, move |k: usize| {
                let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                most.fetch_max(now, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(2));
                running.fetch_sub(1, Ordering::SeqCst);
                done.lock().unwrap().push(k);
            })
        };
        for k in 0..40 {
            pool.push(k, false);
        }
        assert_eq!(pool.started, 3, "one thread per worker, not per picture");
        let start = Instant::now();
        while done.lock().unwrap().len() < 40 && start.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut got = done.lock().unwrap().clone();
        got.sort_unstable();
        assert_eq!(got, (0..40).collect::<Vec<_>>());
        assert!(most.load(Ordering::SeqCst) <= 3);
    }

    #[test]
    fn urgent_and_hurried_keys_go_first() {
        // a pool whose worker is held up, so the queue can be inspected
        let gate = Arc::new(Mutex::new(()));
        let held = gate.lock().unwrap();
        let order = Arc::new(Mutex::new(Vec::new()));
        let mut pool = {
            let (gate, order) = (gate.clone(), order.clone());
            Pool::new(1, move |k: u32| {
                let _open = gate.lock().unwrap();
                order.lock().unwrap().push(k);
            })
        };
        pool.push(0, false);
        // wait until the worker holds key 0 and is blocked on the gate
        let start = Instant::now();
        while !pool.shared.queue.lock().unwrap().keys.is_empty()
            && start.elapsed() < Duration::from_secs(5)
        {
            std::thread::sleep(Duration::from_millis(1));
        }
        pool.push(1, false);
        pool.push(2, false);
        pool.push(3, true);
        pool.hurry(&2);
        let queued: Vec<u32> = pool
            .shared
            .queue
            .lock()
            .unwrap()
            .keys
            .iter()
            .copied()
            .collect();
        assert_eq!(queued, [2, 3, 1]);
        drop(held);
    }
}
