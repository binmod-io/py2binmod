use std::path::Path;

pub trait FileIgnoreStrategy: Send + Sync {
    fn should_ignore(&self, path: &Path) -> bool;
}