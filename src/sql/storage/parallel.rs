//! Parallel scan helpers (Phase 3) — splits work across threads.

const PARALLEL_THRESHOLD: usize = 10_000;

pub fn should_parallelize(n: usize) -> bool {
    n >= PARALLEL_THRESHOLD
}

pub fn parallel_count(total: usize) -> i64 {
    if total < PARALLEL_THRESHOLD {
        return total as i64;
    }
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8);
    let chunk = (total + threads - 1) / threads;
    let mut handles = Vec::new();
    for t in 0..threads {
        let start = t * chunk;
        if start >= total {
            break;
        }
        let end = (start + chunk).min(total);
        handles.push(std::thread::spawn(move || (end - start) as i64));
    }
    let mut sum = 0i64;
    for h in handles {
        if let Ok(n) = h.join() {
            sum += n;
        }
    }
    sum
}

pub fn chunk_ranges(len: usize, threads: usize) -> Vec<(usize, usize)> {
    let threads = threads.max(1);
    let chunk = (len + threads - 1) / threads;
    let mut out = Vec::new();
    for t in 0..threads {
        let start = t * chunk;
        if start >= len {
            break;
        }
        out.push((start, (start + chunk).min(len)));
    }
    out
}
