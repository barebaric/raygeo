use crate::pipeline::callbacks::Callbacks;
use crate::pipeline::stage::StageSpec;

pub struct NodeRequest {
    pub key: String,
    pub generation_id: u64,
    pub version_token: u64,
    pub stage: StageSpec,
    pub callbacks: Box<dyn Callbacks>,
    pub cacheable: bool,
}

impl std::fmt::Debug for NodeRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeRequest")
            .field("key", &self.key)
            .field("generation_id", &self.generation_id)
            .field("version_token", &self.version_token)
            .field("stage", &self.stage)
            .field("callbacks", &"<Callbacks>")
            .field("cacheable", &self.cacheable)
            .finish()
    }
}

impl NodeRequest {
    pub fn new(
        key: impl Into<String>,
        generation_id: u64,
        version_token: u64,
        stage: StageSpec,
        callbacks: Box<dyn Callbacks>,
        cacheable: bool,
    ) -> Self {
        NodeRequest {
            key: key.into(),
            generation_id,
            version_token,
            stage,
            callbacks,
            cacheable,
        }
    }
}
