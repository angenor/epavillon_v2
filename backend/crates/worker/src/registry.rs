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

    /// Les files à écouter, dédoublonnées et **ordonnées** : la boucle les
    /// parcourt dans le même ordre à chaque tour, ce qui rend son comportement
    /// reproductible d'une exécution à l'autre.
    ///
    /// La file par défaut y figure toujours, même sans gestionnaire qui la
    /// déclare : c'est celle où atterrit tout travail enfilé sans précision.
    pub fn queues(&self) -> Vec<String> {
        let mut files: Vec<String> = self
            .handlers
            .values()
            .map(|h| h.queue().to_owned())
            .chain(std::iter::once(kernel::jobs::DEFAULT_QUEUE.to_owned()))
            .collect();
        files.sort();
        files.dedup();
        files
    }
}
