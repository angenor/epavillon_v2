//! Registre des travaux différés, indexé par le nom de la tâche.
//!
//! Un module déclare ses travaux ; le worker les monte sans les connaître.

use kernel::jobs::JobHandler;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct JobRegistry {
    handlers: HashMap<&'static str, Arc<dyn JobHandler>>,
}

impl JobRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_all(mut self, handlers: impl IntoIterator<Item = Arc<dyn JobHandler>>) -> Self {
        for handler in handlers {
            self.handlers.insert(handler.task(), handler);
        }
        self
    }

    pub fn get(&self, task: &str) -> Option<&dyn JobHandler> {
        self.handlers.get(task).map(|h| h.as_ref())
    }
}
