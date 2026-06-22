//! Error types. The Rust rewrite keeps POSIX-style failure semantics: each
//! variant maps to a classic `errno` so the FFI boundary can stay faithful to
//! `include/urte/*.h` (returns -1 / sets errno).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// EINVAL — invalid argument.
    Invalid(String),
    /// EPERM — operation not permitted (guardrail / privilege).
    Perm(String),
    /// EBADF — bad descriptor.
    BadFd,
    /// ENOMEM — out of kernel memory.
    NoMem,
    /// EEXIST — already exists.
    Exists,
    /// ESRCH — no such process.
    NoProc,
    /// ENOSYS — not implemented.
    NoSys,
    /// EMSGSIZE — message too large.
    MsgSize,
    /// EAGAIN — would block.
    Again,
}

impl KernelError {
    /// POSIX errno value for this error.
    pub fn errno(&self) -> i32 {
        match self {
            KernelError::Invalid(_) => 22, // EINVAL
            KernelError::Perm(_) => 1,     // EPERM
            KernelError::BadFd => 9,       // EBADF
            KernelError::NoMem => 12,      // ENOMEM
            KernelError::Exists => 17,     // EEXIST
            KernelError::NoProc => 3,      // ESRCH
            KernelError::NoSys => 38,      // ENOSYS
            KernelError::MsgSize => 90,    // EMSGSIZE
            KernelError::Again => 11,      // EAGAIN
        }
    }
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::Invalid(m) => write!(f, "EINVAL: {m}"),
            KernelError::Perm(m) => write!(f, "EPERM: {m}"),
            KernelError::BadFd => write!(f, "EBADF"),
            KernelError::NoMem => write!(f, "ENOMEM"),
            KernelError::Exists => write!(f, "EEXIST"),
            KernelError::NoProc => write!(f, "ESRCH"),
            KernelError::NoSys => write!(f, "ENOSYS"),
            KernelError::MsgSize => write!(f, "EMSGSIZE"),
            KernelError::Again => write!(f, "EAGAIN"),
        }
    }
}

impl std::error::Error for KernelError {}

pub type Result<T> = std::result::Result<T, KernelError>;
