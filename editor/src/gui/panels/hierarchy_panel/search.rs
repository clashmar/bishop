use engine_core::ecs::{get_children, Ecs, Entity, Name};
use std::collections::HashSet;
use widgets::matches_query_normalized;

pub(crate) fn visible_roots(
    roots: &[Entity],
    visible_entities: Option<&HashSet<Entity>>,
) -> Vec<Entity> {
    roots
        .iter()
        .copied()
        .filter(|root| visible_entities.is_none_or(|visible| visible.contains(root)))
        .collect()
}

pub(crate) fn filter_hierarchy(roots: &[Entity], query: &str, ecs: &Ecs) -> HashSet<Entity> {
    let query_lowercase = query.to_lowercase();
    let mut visible = HashSet::new();
    for &root in roots {
        collect_visible_entities(root, &query_lowercase, ecs, &mut visible);
    }
    visible
}

fn collect_visible_entities(
    entity: Entity,
    query_lowercase: &str,
    ecs: &Ecs,
    visible: &mut HashSet<Entity>,
) -> bool {
    let name = ecs
        .get::<Name>(entity)
        .map(|name| name.to_string())
        .unwrap_or_else(|| format!("{entity:?}"));
    let mut has_visible_descendant = false;
    for child in get_children(ecs, entity) {
        if collect_visible_entities(child, query_lowercase, ecs, visible) {
            has_visible_descendant = true;
        }
    }

    let is_visible = matches_query_normalized(&name, query_lowercase) || has_visible_descendant;
    if is_visible {
        visible.insert(entity);
    }
    is_visible
}

#[derive(Default)]
pub(crate) struct HierarchySearchState {
    query: String,
    expanded_before_search: Option<HashSet<Entity>>,
}

impl HierarchySearchState {
    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    pub(crate) fn is_active(&self) -> bool {
        !self.query.is_empty()
    }

    pub(crate) fn set_query(&mut self, query: &str, expanded: &mut HashSet<Entity>) {
        if self.query.is_empty() && !query.is_empty() {
            self.expanded_before_search = Some(expanded.clone());
        } else if !self.query.is_empty() && query.is_empty() {
            if let Some(expansion) = self.expanded_before_search.take() {
                *expanded = expansion;
            }
        }

        self.query = query.to_string();
    }
}
