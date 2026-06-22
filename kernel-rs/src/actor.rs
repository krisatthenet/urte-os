//! Elixir/OTP-style actor layer — a "process" model for Rust.
//!
//! Each actor is a lightweight process with a mailbox (a `crossbeam-channel`),
//! processed by one OS thread. Messages are handled one at a time (no shared
//! mutable state), exactly like an Elixir `GenServer`. A [`Supervisor`] provides
//! the OTP "let it crash" guarantee: if an actor panics while handling a
//! message, the supervisor restarts it from its factory (one-for-one), up to a
//! restart budget. The mailbox survives a restart, so senders keep their `Addr`.

use crossbeam_channel::{unbounded, Sender};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

pub type Pid = u64;

static NEXT_PID: AtomicU64 = AtomicU64::new(1);

enum Envelope<M> {
    Msg(M),
    Stop,
}

/// GenServer-like behaviour. Implementors define how to handle one message.
pub trait Actor: Send + 'static {
    type Msg: Send + 'static;

    /// Called once when the actor process starts (and after each restart).
    fn started(&mut self, _pid: Pid) {}
    /// Handle a single message. A panic here triggers supervised restart.
    fn handle(&mut self, msg: Self::Msg);
    /// Called when the actor stops (normal stop or terminal crash).
    fn stopped(&mut self, _pid: Pid) {}
}

/// A cloneable address (like an Elixir pid) used to send messages to an actor.
pub struct Addr<M> {
    pid: Pid,
    tx: Sender<Envelope<M>>,
}

impl<M> Clone for Addr<M> {
    fn clone(&self) -> Self {
        Addr { pid: self.pid, tx: self.tx.clone() }
    }
}

impl<M: Send + 'static> Addr<M> {
    pub fn pid(&self) -> Pid {
        self.pid
    }
    /// Asynchronous cast (fire-and-forget). Returns false if the actor is gone.
    pub fn send(&self, msg: M) -> bool {
        self.tx.send(Envelope::Msg(msg)).is_ok()
    }
    /// Ask the actor to stop after draining queued messages.
    pub fn stop(&self) {
        let _ = self.tx.send(Envelope::Stop);
    }
}

/// Owns the actor process threads so they can be joined on shutdown.
#[derive(Default)]
pub struct ActorSystem {
    handles: Mutex<Vec<JoinHandle<()>>>,
}

impl ActorSystem {
    pub fn new() -> Self {
        ActorSystem::default()
    }

    /// Spawn an unsupervised actor process. A panic in `handle` is caught and
    /// logged-by-discard; the actor keeps running with its prior state.
    pub fn spawn<A: Actor>(&self, mut actor: A) -> Addr<A::Msg> {
        let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = unbounded::<Envelope<A::Msg>>();
        let handle = thread::spawn(move || {
            actor.started(pid);
            while let Ok(env) = rx.recv() {
                match env {
                    Envelope::Msg(m) => {
                        let _ = catch_unwind(AssertUnwindSafe(|| actor.handle(m)));
                    }
                    Envelope::Stop => break,
                }
            }
            actor.stopped(pid);
        });
        self.handles.lock().unwrap().push(handle);
        Addr { pid, tx }
    }

    /// Join all actor process threads (blocks until every actor stops).
    pub fn join_all(&self) {
        let mut hs = self.handles.lock().unwrap();
        for h in hs.drain(..) {
            let _ = h.join();
        }
    }
}

/// OTP-style restart strategies (one-for-one supervises a single child).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restart {
    /// Restart on crash up to `max_restarts` times, then give up.
    Transient,
    /// Never restart; crash is terminal.
    Temporary,
}

pub struct Supervisor;

impl Supervisor {
    /// Supervise one child built by `factory`. On a panic while handling a
    /// message the child is rebuilt from the factory (one-for-one). The mailbox
    /// is preserved across restarts so the returned `Addr` stays valid.
    pub fn one_for_one<A, F>(
        system: &ActorSystem,
        factory: F,
        restart: Restart,
        max_restarts: u32,
    ) -> Addr<A::Msg>
    where
        A: Actor,
        F: Fn() -> A + Send + 'static,
    {
        let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = unbounded::<Envelope<A::Msg>>();
        let handle = thread::spawn(move || {
            let mut restarts = 0u32;
            let mut actor = factory();
            actor.started(pid);
            while let Ok(env) = rx.recv() {
                match env {
                    Envelope::Msg(m) => {
                        let res = catch_unwind(AssertUnwindSafe(|| actor.handle(m)));
                        if res.is_err() {
                            actor.stopped(pid);
                            restarts += 1;
                            if restart == Restart::Temporary || restarts > max_restarts {
                                return;
                            }
                            actor = factory(); // one-for-one restart
                            actor.started(pid);
                        }
                    }
                    Envelope::Stop => break,
                }
            }
            actor.stopped(pid);
        });
        system.handles.lock().unwrap().push(handle);
        Addr { pid, tx }
    }
}
