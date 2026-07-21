use std::any::Any;

use crate::cnc::execution::callbacks::OpsCallbacksAdapter;
use crate::cnc::execution::specs::AggregateOutput;
use crate::ops::assembly::AssemblyOutput;
use crate::ops::convert::{EncodeCtx, EncodeOutput, Encoder};
use crate::pipeline::cache::CacheKey;
use crate::pipeline::compute::{Compute, ComputeCtx};

pub struct EncoderCompute {
    pub encoder: Box<dyn Encoder>,
    pub source_key: String,
}

impl Compute for EncoderCompute {
    fn run(
        &mut self,
        ctx: &mut ComputeCtx,
    ) -> Result<Box<dyn Any + Send + Sync>, String> {
        let upstream = ctx.deps.get(&self.source_key).ok_or_else(|| {
            format!("missing dependency: {}", self.source_key)
        })?;

        let ops = upstream
            .downcast_ref::<AssemblyOutput>()
            .map(|a| &a.ops)
            .or_else(|| {
                upstream.downcast_ref::<AggregateOutput>().map(|a| &a.ops)
            })
            .ok_or_else(|| {
                format!("cannot get Ops from dep: {}", self.source_key)
            })?;

        if ctx.callbacks.is_cancelled() {
            return Err("cancelled".to_string());
        }

        let adapter = OpsCallbacksAdapter {
            inner: ctx.callbacks,
        };
        let mut encode_ctx = EncodeCtx {
            ops,
            callbacks: &adapter,
        };
        let output = self.encoder.encode(&mut encode_ctx)?;
        Ok(Box::new(output))
    }

    fn source_keys(&self) -> Vec<String> {
        vec![self.source_key.clone()]
    }

    fn cache_key(&self, tag: &str) -> Option<CacheKey> {
        Some(CacheKey::new(tag))
    }

    fn restore_from_cache(
        &mut self,
        cached: &(dyn Any + Send + Sync),
    ) -> Result<Box<dyn Any + Send + Sync>, String> {
        let output =
            cached.downcast_ref::<EncodeOutput>().ok_or_else(|| {
                "cache type mismatch: expected EncodeOutput".to_string()
            })?;
        Ok(Box::new(output.clone()))
    }

    fn prepare_cache_entry(
        &self,
        output: &(dyn Any + Send + Sync),
    ) -> Option<Box<dyn Any + Send + Sync>> {
        let output = output.downcast_ref::<EncodeOutput>()?;
        Some(Box::new(output.clone()))
    }

    fn name(&self) -> &'static str {
        self.encoder.name()
    }
}
