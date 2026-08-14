use engine_core::ecs::*;
use engine_core::game::GameCtxMut;
use std::collections::HashSet;

pub(crate) fn prune_hidden_dependency_components(
    ctx: &mut GameCtxMut<'_>,
    entity: Entity,
    removed_type_name: &str,
) {
    let pending = dependency_closure(removed_type_name);
    prune_dependency_components(ctx, entity, &pending, true);
}

pub(crate) fn prune_recorded_dependency_components(
    ctx: &mut GameCtxMut<'_>,
    entity: Entity,
    created_dependency_type_names: &[&'static str],
) {
    prune_dependency_components(ctx, entity, created_dependency_type_names, false);
}

pub(crate) fn present_dependency_closure(
    ecs: &Ecs,
    entity: Entity,
    root_type_name: &str,
) -> Vec<&'static str> {
    dependency_closure(root_type_name)
        .into_iter()
        .filter(|type_name| {
            COMPONENTS
                .iter()
                .find(|reg| reg.type_name == *type_name)
                .is_some_and(|reg| (reg.has)(ecs, entity))
        })
        .collect()
}

pub(crate) fn dependency_closure(type_name: &str) -> Vec<&'static str> {
    let Some(root) = COMPONENTS.iter().find(|r| r.type_name == type_name) else {
        return Vec::new();
    };

    let mut pending: Vec<_> = root.deps.to_vec();
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    while let Some(dep_type_name) = pending.pop() {
        if !seen.insert(dep_type_name) {
            continue;
        }

        result.push(dep_type_name);

        if let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == dep_type_name) {
            pending.extend(reg.deps.iter().copied());
        }
    }

    result
}

fn prune_dependency_components(
    ctx: &mut GameCtxMut<'_>,
    entity: Entity,
    pending: &[&'static str],
    keep_first_class: bool,
) {
    loop {
        let mut removed_any = false;

        for &type_name in pending {
            if keep_first_class && component_is_first_class(type_name) {
                continue;
            }

            let Some(reg) = COMPONENTS.iter().find(|r| r.type_name == type_name) else {
                continue;
            };

            if !(reg.has)(ctx.ecs(), entity)
                || component_has_dependents(type_name, entity, ctx.ecs())
            {
                continue;
            }

            Ecs::remove_component_by_type_name(ctx, entity, type_name);
            removed_any = true;
        }

        if !removed_any {
            break;
        }
    }
}
