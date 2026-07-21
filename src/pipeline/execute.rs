use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rayon::Scope;

use crate::pipeline::aggregate::{AggregateCtx, DepMap};
use crate::pipeline::cache::Cache;
use crate::pipeline::callbacks::Callbacks;
use crate::pipeline::completed::CompletedNode;
use crate::pipeline::compute::ComputeCtx;
use crate::pipeline::request::NodeRequest;
use crate::pipeline::stage::StageSpec;

#[derive(Debug)]
pub struct Cancelled;

pub fn execute_stages(
    nodes: Vec<NodeRequest>,
    on_completed: impl Fn(CompletedNode) + Send + Sync + 'static,
    on_batch_progress: Option<Arc<dyn Fn(f64, String) + Send + Sync + 'static>>,
    cache: &Arc<Mutex<Cache>>,
) -> Result<(), Cancelled> {
    if nodes.is_empty() {
        if let Some(cb) = &on_batch_progress {
            cb(1.0, String::new());
        }
        return Ok(());
    }

    let mut groups: HashMap<String, Vec<NodeRequest>> = HashMap::new();
    for node in nodes {
        groups.entry(node.key.clone()).or_default().push(node);
    }

    let total = groups.len();
    let mut node_by_key: HashMap<String, NodeRequest> = HashMap::new();
    let mut shadows: HashMap<String, Vec<u64>> = HashMap::new();

    for (key, mut group) in groups {
        let primary = group.remove(0);
        let shadow_list: Vec<u64> =
            group.into_iter().map(|n| n.generation_id).collect();
        shadows.insert(key.clone(), shadow_list);
        node_by_key.insert(key, primary);
    }

    let all_keys: HashSet<String> = node_by_key.keys().cloned().collect();
    let mut deps_remaining: HashMap<String, usize> = HashMap::new();
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    for (key, node) in &node_by_key {
        let in_batch: Vec<String> = collect_deps(&node.stage)
            .into_iter()
            .filter(|d| all_keys.contains(d))
            .collect();
        for d in &in_batch {
            dependents.entry(d.clone()).or_default().push(key.clone());
        }
        deps_remaining.insert(key.clone(), in_batch.len());
    }

    let shared = Arc::new(SharedState {
        node_by_key: Mutex::new(node_by_key),
        deps_remaining: Mutex::new(deps_remaining),
        dependents,
        shadows,
        dep_map: Mutex::new(HashMap::with_capacity(total)),
        progress: Mutex::new(HashMap::with_capacity(total)),
        completed_count: Mutex::new(0usize),
        cache: Arc::clone(cache),
        on_completed: Arc::new(on_completed),
        on_batch: on_batch_progress,
        any_cancelled: AtomicBool::new(false),
        total,
    });

    let initial: Vec<String> = shared
        .deps_remaining
        .lock()
        .unwrap()
        .iter()
        .filter_map(|(k, c)| if *c == 0 { Some(k.clone()) } else { None })
        .collect();

    rayon::scope(|s| {
        for key in initial {
            spawn_one(s, &shared, key);
        }
    });

    let leftover: Vec<String> =
        shared.node_by_key.lock().unwrap().keys().cloned().collect();
    for key in leftover {
        deliver_synthetic_completion(&shared, &key);
    }

    if let Some(cb) = &shared.on_batch {
        cb(1.0, String::new());
    }

    if shared.any_cancelled.load(Ordering::SeqCst) {
        Err(Cancelled)
    } else {
        Ok(())
    }
}

struct SharedState {
    node_by_key: Mutex<HashMap<String, NodeRequest>>,
    deps_remaining: Mutex<HashMap<String, usize>>,
    dependents: HashMap<String, Vec<String>>,
    shadows: HashMap<String, Vec<u64>>,
    dep_map: Mutex<DepMap>,
    progress: Mutex<HashMap<String, f64>>,
    completed_count: Mutex<usize>,
    cache: Arc<Mutex<Cache>>,
    on_completed: Arc<dyn Fn(CompletedNode) + Send + Sync + 'static>,
    on_batch: Option<Arc<dyn Fn(f64, String) + Send + Sync + 'static>>,
    any_cancelled: AtomicBool,
    total: usize,
}

fn spawn_one(s: &Scope<'_>, shared: &Arc<SharedState>, key: String) {
    let shared = Arc::clone(shared);

    s.spawn(move |s| {
        let node = match shared.node_by_key.lock().unwrap().remove(&key) {
            Some(n) => n,
            None => return,
        };

        let NodeRequest {
            key: node_key,
            generation_id,
            version_token: _,
            stage,
            callbacks,
        } = node;

        let scheduled_epoch = shared
            .cache
            .lock()
            .map(|c| c.get_epoch(&node_key))
            .unwrap_or(0);

        let wrapper = ProgressWrapper {
            inner: &*callbacks,
            key: node_key.clone(),
            shared: &shared,
        };

        let result = {
            let deps_lock = shared.dep_map.lock().unwrap();
            dispatch_stage(stage, &wrapper, &shared, &node_key, &deps_lock)
        };

        let current_epoch = shared
            .cache
            .lock()
            .map(|c| c.get_epoch(&node_key))
            .unwrap_or(0);

        let output_arc = if scheduled_epoch != current_epoch {
            let node = CompletedNode::err(
                node_key.clone(),
                generation_id,
                "superseded".to_string(),
            );
            (shared.on_completed)(node);
            None
        } else {
            match result {
                Err(e) => {
                    let cancelled = e == "cancelled";
                    if cancelled {
                        shared.any_cancelled.store(true, Ordering::SeqCst);
                    }
                    let node =
                        CompletedNode::err(node_key.clone(), generation_id, e);
                    (shared.on_completed)(node);
                    None
                }
                Ok(boxed) => {
                    let arc: Arc<dyn Any + Send + Sync> = boxed.into();
                    shared
                        .dep_map
                        .lock()
                        .unwrap()
                        .insert(node_key.clone(), Arc::clone(&arc));
                    let node = CompletedNode::ok(
                        node_key.clone(),
                        generation_id,
                        Arc::clone(&arc),
                    );
                    (shared.on_completed)(node);
                    Some(arc)
                }
            }
        };

        if let Some(ref output_arc) = output_arc {
            let shadows =
                shared.shadows.get(&node_key).cloned().unwrap_or_default();
            for shadow_gen in shadows {
                let node = CompletedNode::ok(
                    node_key.clone(),
                    shadow_gen,
                    Arc::clone(output_arc),
                );
                (shared.on_completed)(node);
            }
        }

        {
            let mut pm = shared.progress.lock().unwrap();
            pm.remove(&node_key);
        }
        {
            let mut cc = shared.completed_count.lock().unwrap();
            *cc += 1;
        }
        emit_batch_progress(&shared);

        let new_ready: Vec<String> = {
            let mut dr = shared.deps_remaining.lock().unwrap();
            shared
                .dependents
                .get(&node_key)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|dep| {
                    let count = dr.entry(dep.clone()).or_insert(0);
                    if *count > 0 {
                        *count -= 1;
                    }
                    if *count == 0 {
                        Some(dep)
                    } else {
                        None
                    }
                })
                .collect()
        };

        for dep_key in new_ready {
            spawn_one(s, &shared, dep_key);
        }
    });
}

fn dispatch_stage(
    stage: StageSpec,
    callbacks: &dyn Callbacks,
    shared: &Arc<SharedState>,
    node_key: &str,
    deps: &DepMap,
) -> Result<Box<dyn Any + Send + Sync>, String> {
    match stage {
        StageSpec::Compute { mut compute_fn } => {
            let cache_key = compute_fn.cache_key(node_key);
            if let Some(key) = &cache_key {
                let mut c = shared.cache.lock().map_err(|e| e.to_string())?;
                if let Some(cached) = c.get(key) {
                    let restored = compute_fn.restore_from_cache(&**cached);
                    if let Ok(out) = restored {
                        return Ok(out);
                    }
                }
            }

            let mut ctx = ComputeCtx { callbacks, deps };
            let result = compute_fn.run(&mut ctx);

            if let Ok(ref out) = result {
                if let Some(key) = cache_key {
                    if let Some(entry) =
                        compute_fn.prepare_cache_entry(out.as_ref())
                    {
                        if let Ok(mut c) = shared.cache.lock() {
                            let size = 1024;
                            c.insert(key, entry, size);
                        }
                    }
                }
            }
            result
        }
        StageSpec::Aggregate { mut aggregate_fn } => {
            let cache_key = aggregate_fn.cache_key(node_key);
            if let Some(key) = &cache_key {
                let mut c = shared.cache.lock().map_err(|e| e.to_string())?;
                if let Some(cached) = c.get(key) {
                    let restored = aggregate_fn.restore_from_cache(&**cached);
                    if let Ok(out) = restored {
                        return Ok(out);
                    }
                }
            }

            let mut agg_ctx = AggregateCtx::new(callbacks);
            let result = aggregate_fn.run(&mut agg_ctx, deps);

            if let Ok(ref out) = result {
                if let Some(key) = cache_key {
                    if let Some(entry) =
                        aggregate_fn.prepare_cache_entry(out.as_ref())
                    {
                        if let Ok(mut c) = shared.cache.lock() {
                            let size = 1024;
                            c.insert(key, entry, size);
                        }
                    }
                }
            }
            result
        }
    }
}

fn collect_deps(stage: &StageSpec) -> Vec<String> {
    stage.source_keys()
}

fn deliver_synthetic_completion(shared: &Arc<SharedState>, key: &str) {
    let gen = shared
        .node_by_key
        .lock()
        .unwrap()
        .get(key)
        .map(|n| n.generation_id)
        .unwrap_or(0);

    let node = CompletedNode::err(
        key.to_string(),
        gen,
        "unsatisfiable dependency".to_string(),
    );
    (shared.on_completed)(node);

    let shadows = shared.shadows.get(key).cloned().unwrap_or_default();
    for shadow_gen in shadows {
        let node = CompletedNode::err(
            key.to_string(),
            shadow_gen,
            "unsatisfiable dependency".to_string(),
        );
        (shared.on_completed)(node);
    }
}

fn emit_batch_progress(shared: &Arc<SharedState>) {
    let cb = match &shared.on_batch {
        Some(cb) => cb,
        None => return,
    };
    let in_flight: f64 =
        shared.progress.lock().unwrap().values().copied().sum();
    let done = *shared.completed_count.lock().unwrap() as f64;
    let frac = if shared.total > 0 {
        ((done + in_flight) / shared.total as f64).min(1.0)
    } else {
        0.0
    };
    cb(frac, String::new());
}

struct ProgressWrapper<'a> {
    inner: &'a dyn Callbacks,
    key: String,
    shared: &'a Arc<SharedState>,
}

// ProgressWrapper is only used within a single rayon scope (one thread at a time)
unsafe impl<'a> Send for ProgressWrapper<'a> {}

impl<'a> Callbacks for ProgressWrapper<'a> {
    fn report_progress(&self, frac: f64, msg: &str) {
        self.inner.report_progress(frac, msg);
        self.shared
            .progress
            .lock()
            .unwrap()
            .insert(self.key.clone(), frac);
        emit_batch_progress(self.shared);
    }

    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    fn emit_chunk(&self, chunk: Box<dyn Any + Send + Sync>) {
        self.inner.emit_chunk(chunk);
    }
}
