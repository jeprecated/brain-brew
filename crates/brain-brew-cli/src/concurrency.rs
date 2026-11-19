//! Order-preserving concurrent map for independent, read-only work items.

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// Apply `work` to every item on scoped threads and return results in input order.
///
/// Workers share one queue, so uneven items balance across threads. Callers
/// inspect results in input order, so the first error they report is the one a
/// sequential loop would reach. A worker panic resumes on the calling thread.
pub(crate) fn map_in_order<T, R>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<R>
where
    T: Sync,
    R: Send,
{
    let workers = thread::available_parallelism()
        .map_or(1, NonZeroUsize::get)
        .min(items.len());
    if workers <= 1 {
        return items.iter().map(work).collect();
    }
    let next = AtomicUsize::new(0);
    let mut results = (0..items.len()).map(|_| None).collect::<Vec<_>>();
    thread::scope(|scope| {
        let handles = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = items.get(index) else {
                            return done;
                        };
                        done.push((index, work(item)));
                    }
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            let done = handle
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            for (index, result) in done {
                results[index] = Some(result);
            }
        }
    });
    results
        .into_iter()
        .map(|result| result.expect("every queued item is processed"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_keep_input_order() {
        let items = (0..257).collect::<Vec<u64>>();
        let results = map_in_order(&items, |item| item * item);
        assert_eq!(
            results,
            items.iter().map(|item| item * item).collect::<Vec<_>>()
        );
    }

    #[test]
    fn empty_input_maps_to_empty_output() {
        assert!(map_in_order(&[] as &[u8], |item| *item).is_empty());
    }
}
