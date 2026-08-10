use std::any::Any;

use crate::cnc::execution::callbacks::OpsCallbacksAdapter;
use crate::cnc::execution::specs::AggregateOutput;
use crate::ops::assembly::AssemblyOutput;
use crate::ops::convert::{EncodeCtx, Encoder};
use crate::pipeline::cache::CacheKey;
use crate::pipeline::completed::PipelineError;
use crate::pipeline::compute::{Compute, ComputeCtx};

pub struct EncoderCompute {
    pub encoder: Box<dyn Encoder>,
    pub source_key: String,
}

impl Compute for EncoderCompute {
    fn run(
        &mut self,
        ctx: &mut ComputeCtx,
    ) -> Result<Box<dyn Any + Send + Sync>, PipelineError> {
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
            return Err(PipelineError::Cancelled);
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

    fn cache_key(&self, _tag: &str) -> Option<CacheKey> {
        // Encode is the final pipeline stage: its output is consumed
        // immediately by the caller and likely needs recomputation
        // anyway, since otherwise the caller would likely not have
        // re-started the pipeline in the first place.
        None
    }

    fn name(&self) -> &str {
        self.encoder.name()
    }
}
