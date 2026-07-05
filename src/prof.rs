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
    /// Nesting depth.  depth == 0 means no profiled function is running.
    depth: u32,
    /// Wall clock started when depth went 0→1.
    wall_start: Option<Instant>,
    /// Wall time accumulated during depth==0→1→…→0 intervals.
    wall_total: Duration,
}

impl ProfileData {
    fn new() -> Self {
        Self {
            timers: HashMap::new(),
            accum: HashMap::new(),
            counts: HashMap::new(),
            depth: 0,
            wall_start: None,
            wall_total: Duration::ZERO,
        }
    }
}

pub fn prof_start(name: &str) {
    PROF.with(|p| {
        let mut data = p.borrow_mut();
        if data.depth == 0 {
            data.wall_start = Some(Instant::now());
        }
        data.depth += 1;
        data.timers.insert(name.to_string(), Instant::now());
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
        data.depth = data.depth.saturating_sub(1);
        if data.depth == 0 {
            if let Some(ws) = data.wall_start.take() {
                data.wall_total += ws.elapsed();
            }
        }
    });
}

pub fn prof_report() {
    PROF.with(|p| {
        let data = p.borrow();
        let mut entries: Vec<_> = data.accum.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));

        // Include in-flight wall time if we're still inside a profiled
        // function (prof_report is often called from within one).
        let mut wall = data.wall_total;
        if data.depth > 0 {
            if let Some(ws) = data.wall_start {
                wall += ws.elapsed();
            }
        }
        let wall_ms = wall.as_secs_f64() * 1000.0;

        println!("\n=== PROFILING SUMMARY ===");
        for (name, dur) in &entries {
            let display = name.strip_prefix("raygeo::").unwrap_or(name);
            let count = data.counts.get(*name).unwrap_or(&0);
            let pct = if wall_ms > 0.0 {
                dur.as_secs_f64() * 100.0 / wall.as_secs_f64()
            } else {
                0.0
            };
            let avg_ms = dur.as_secs_f64() * 1000.0 / *count as f64;
            println!(
                "{:>60}: {:>8.1}ms ({:>5.1}%) {:>6} calls, avg {:.3}ms",
                display,
                dur.as_secs_f64() * 1000.0,
                pct,
                count,
                avg_ms
            );
        }
        println!("{:>60}: {:>8.1}ms", "WALL", wall_ms);
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
