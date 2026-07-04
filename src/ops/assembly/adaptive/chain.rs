/// Generic priority-ordered strategy runner used by both resume and
/// routing orchestrators.
///
/// Owns the `(strategy, source_code)` array and per-strategy detail
/// codes.  [`run`](StrategyChain::run) calls each strategy in order
/// via the supplied `try_fn`; on success the optional `post_hook` may
/// accept, transform, or reject the result.  The first acceptance
/// returns `(source, result)`.
pub struct StrategyChain<S, Source: Copy, const N: usize> {
    entries: [(S, Source); N],
    details: [u8; N],
}

impl<S: Copy, Source: Copy, const N: usize> StrategyChain<S, Source, N> {
    pub fn new(entries: [(S, Source); N]) -> Self {
        Self {
            entries,
            details: [0; N],
        }
    }

    pub fn details(&self) -> &[u8; N] {
        &self.details
    }

    /// Run each strategy in priority order.
    ///
    /// * `try_fn(idx, strategy, source, detail)` — invoke the strategy;
    ///   set `*detail` to the failure code on `None`.
    /// * `post_hook(idx, strategy, source, detail, result)` — optional
    ///   gate applied only when `try_fn` returns `Some`.  Return
    ///   `Some(transformed)` to accept, or `None` to reject and
    ///   continue to the next strategy.
    pub fn run<R>(
        &mut self,
        mut try_fn: impl FnMut(usize, S, Source, &mut u8) -> Option<R>,
        mut post_hook: Option<
            impl FnMut(usize, S, Source, &mut u8, R) -> Option<R>,
        >,
    ) -> Option<(Source, R)> {
        for (idx, (s, source)) in self.entries.iter().enumerate() {
            self.details[idx] = 0;
            let outcome = try_fn(idx, *s, *source, &mut self.details[idx]);
            if let Some(r) = outcome {
                let accepted = match &mut post_hook {
                    Some(hook) => {
                        hook(idx, *s, *source, &mut self.details[idx], r)
                    }
                    None => Some(r),
                };
                if let Some(accepted_r) = accepted {
                    return Some((*source, accepted_r));
                }
            }
        }
        None
    }
}
