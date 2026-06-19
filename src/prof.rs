use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

thread_local! {
    static PROF: RefCell<ProfileData> = RefCell::new(ProfileData::new());
}

struct ProfileData {
    timers: HashMap<String, Instant>,
    accum: HashMap<String, Duration>,
    counts: HashMap<String, u64>,
}

impl ProfileData {
    fn new() -> Self {
        Self {
            timers: HashMap::new(),
            accum: HashMap::new(),
            counts: HashMap::new(),
        }
    }
}

pub fn prof_start(name: &str) {
    PROF.with(|p| {
        p.borrow_mut()
            .timers
            .insert(name.to_string(), Instant::now());
    });
}

pub fn prof_end(name: &str) {
    PROF.with(|p| {
        let mut data = p.borrow_mut();
        if let Some(start) = data.timers.remove(name) {
            let elapsed = start.elapsed();
            *data.accum.entry(name.to_string()).or_insert(Duration::ZERO) +=
                elapsed;
            *data.counts.entry(name.to_string()).or_insert(0) += 1;
        }
    });
}

pub fn prof_report() {
    PROF.with(|p| {
        let data = p.borrow();
        let mut entries: Vec<_> = data.accum.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));

        println!("\n=== PROFILING SUMMARY ===");
        let total: Duration = entries.iter().map(|(_, d)| *d).sum();
        for (name, dur) in &entries {
            let count = data.counts.get(*name).unwrap_or(&0);
            let pct = dur.as_secs_f64() / total.as_secs_f64() * 100.0;
            let avg_ms = dur.as_secs_f64() * 1000.0 / *count as f64;
            println!(
                "{:>30}: {:>8.1}ms ({:>5.1}%) {:>6} calls, avg {:.3}ms",
                name,
                dur.as_secs_f64() * 1000.0,
                pct,
                count,
                avg_ms
            );
        }
        println!("{:>30}: {:>8.1}ms", "TOTAL", total.as_secs_f64() * 1000.0);
        println!("===========================\n");
    });
}

pub struct ProfGuard(&'static str);

pub fn prof_guard(name: &'static str) -> ProfGuard {
    prof_start(name);
    ProfGuard(name)
}

impl Drop for ProfGuard {
    fn drop(&mut self) {
        prof_end(self.0);
    }
}
