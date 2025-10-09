use std::collections::HashSet;
use std::sync::Arc;

use crate::agent::config::PromptComponent;
use crate::agent::prompt::components::types::{ComponentDefinition, ComponentRegistry};

use super::{agent, dialogue, planning, system, task, tools, workspace};

/// Runtime registry mirroring the front-end component registry.
pub struct PromptComponentRegistry {
    components: ComponentRegistry,
    loaded: bool,
}

impl PromptComponentRegistry {
    pub fn new() -> Self {
        Self {
            components: ComponentRegistry::new(),
            loaded: false,
        }
    }

    /// Lazily load all component families.
    pub fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }

        self.register_many(agent::definitions());
        self.register_many(system::definitions());
        self.register_many(tools::definitions());
        self.register_many(task::definitions());
        self.register_many(planning::definitions());
        self.register_many(dialogue::definitions());
        self.register_many(workspace::definitions());

        self.loaded = true;
    }

    fn register_many(&mut self, defs: Vec<Arc<dyn ComponentDefinition>>) {
        for def in defs {
            self.components.insert(def.id(), Arc::clone(&def));
        }
    }

    pub fn get(&mut self, id: PromptComponent) -> Option<Arc<dyn ComponentDefinition>> {
        self.ensure_loaded();
        self.components.get(&id).map(Arc::clone)
    }

    pub fn get_all(&mut self) -> ComponentRegistry {
        self.ensure_loaded();
        self.components.clone()
    }

    pub fn sort_by_dependencies(
        &mut self,
        components: &[PromptComponent],
    ) -> Result<Vec<PromptComponent>, String> {
        self.ensure_loaded();

        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        fn visit(
            id: PromptComponent,
            registry: &ComponentRegistry,
            components: &[PromptComponent],
            sorted: &mut Vec<PromptComponent>,
            visited: &mut HashSet<PromptComponent>,
            visiting: &mut HashSet<PromptComponent>,
        ) -> Result<(), String> {
            if visiting.contains(&id) {
                return Err(format!("Circular dependency detected: {:?}", id));
            }
            if visited.contains(&id) {
                return Ok(());
            }

            visiting.insert(id.clone());
            if let Some(def) = registry.get(&id) {
                for dep in def.dependencies() {
                    if components.contains(dep) {
                        visit(dep.clone(), registry, components, sorted, visited, visiting)?;
                    }
                }
            }

            visiting.remove(&id);
            visited.insert(id.clone());
            sorted.push(id.clone());
            Ok(())
        }

        for component in components {
            if !visited.contains(component) {
                visit(
                    component.clone(),
                    &self.components,
                    components,
                    &mut sorted,
                    &mut visited,
                    &mut visiting,
                )?;
            }
        }

        Ok(sorted)
    }

    pub fn validate_dependencies(&mut self, components: &[PromptComponent]) -> Vec<String> {
        self.ensure_loaded();

        let mut errors = Vec::new();
        for component in components {
            match self.components.get(component) {
                Some(def) => {
                    for dep in def.dependencies() {
                        if !components.contains(dep) {
                            errors.push(format!(
                                "Component {:?} depends on {:?} which is missing from the selection",
                                component, dep
                            ));
                        }
                    }
                }
                None => errors.push(format!("Component does not exist: {:?}", component)),
            }
        }
        errors
    }
}

impl Default for PromptComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
