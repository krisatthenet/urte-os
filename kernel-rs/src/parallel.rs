//! Parallel-programming utilities built on `rayon`, plus hyperthread detection.

use rayon::prelude::*;

/// Number of logical CPUs (hyperthreads) available — the natural default width
/// for the parallel scheduler and the SQL mesh.
pub fn cpu_count() -> usize {
    num_cpus::get()
}

/// Data-parallel map over a work-stealing thread pool.
pub fn par_map<T, R, F>(items: Vec<T>, f: F) -> Vec<R>
where
    T: Send,
    R: Send,
    F: Fn(T) -> R + Sync + Send,
{
    items.into_par_iter().map(f).collect()
}

/// Parallel reduce: map each item then fold with `combine`.
pub fn par_reduce<T, R, M, C>(items: Vec<T>, identity: R, map: M, combine: C) -> R
where
    T: Send,
    R: Send + Clone + Sync,
    M: Fn(T) -> R + Sync + Send,
    C: Fn(R, R) -> R + Sync + Send,
{
    items
        .into_par_iter()
        .map(map)
        .reduce(|| identity.clone(), combine)
}
