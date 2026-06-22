//! Typed, scale-aware IPC channels — Rust rewrite of `include/urte/channel.h`.
//!
//! The C implementation is backed by POSIX message queues. This portable Rust
//! library models the same contract with an in-process priority queue plus the
//! per-channel guardrail mask enforced on every send; an `mq`-backed transport
//! can be slotted behind the same API on Linux (see `kernel/urte_core.c`).

use std::collections::VecDeque;

use crate::error::{KernelError, Result};
use crate::types::{ScaleLevel, Trl};

/// Guardrail mask bits applied on every send (dependency-free flags type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuardMask {
    pub bits: u32,
}

impl GuardMask {
    pub const NONE: GuardMask = GuardMask { bits: 0x0000 };
    pub const SCALE: GuardMask = GuardMask { bits: 0x0001 };
    pub const TRL: GuardMask = GuardMask { bits: 0x0002 };
    pub const SIZE: GuardMask = GuardMask { bits: 0x0004 };
    pub const ALL: GuardMask = GuardMask { bits: 0x0001 | 0x0002 | 0x0004 };

    #[inline]
    pub fn contains(self, other: GuardMask) -> bool {
        (self.bits & other.bits) == other.bits
    }
}

#[derive(Debug, Clone)]
pub struct ChannelAttr {
    pub scale_level: ScaleLevel,
    pub required_trl: Trl,
    pub guardrail_mask: GuardMask,
    pub maxmsg: usize,
    pub msgsize: usize,
}

#[derive(Debug)]
pub struct Channel {
    pub name: String,
    attr: ChannelAttr,
    queue: VecDeque<(Vec<u8>, u32)>, // (payload, priority)
}

impl Channel {
    pub fn open(name: &str, attr: ChannelAttr) -> Result<Channel> {
        if name.is_empty() || !name.starts_with('/') {
            return Err(KernelError::Invalid("channel name must start with '/'".into()));
        }
        Ok(Channel { name: name.to_string(), attr, queue: VecDeque::new() })
    }

    /// Send a message. The guardrail mask runs before the message is enqueued.
    /// `sender_trl` is the caller's current TRL (0 = unset -> allowed).
    pub fn send(&mut self, msg: &[u8], prio: u32, sender_trl: Trl) -> Result<()> {
        if msg.is_empty() {
            return Err(KernelError::Invalid("empty message".into()));
        }
        let m = self.attr.guardrail_mask;
        if m.contains(GuardMask::SIZE) && self.attr.msgsize > 0 && msg.len() > self.attr.msgsize {
            return Err(KernelError::MsgSize);
        }
        if m.contains(GuardMask::TRL)
            && self.attr.required_trl > 0
            && sender_trl != 0
            && sender_trl < self.attr.required_trl
        {
            return Err(KernelError::Perm("sender TRL below channel requirement".into()));
        }
        if self.attr.maxmsg > 0 && self.queue.len() >= self.attr.maxmsg {
            return Err(KernelError::Again);
        }
        // Priority-ordered insert (highest priority dequeued first).
        let pos = self.queue.iter().position(|(_, p)| *p < prio).unwrap_or(self.queue.len());
        self.queue.insert(pos, (msg.to_vec(), prio));
        Ok(())
    }

    pub fn recv(&mut self) -> Result<(Vec<u8>, u32)> {
        self.queue.pop_front().ok_or(KernelError::Again)
    }

    pub fn scale(&self) -> ScaleLevel {
        self.attr.scale_level
    }
}
