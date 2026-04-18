use crate::engine::game_instance::GameInstance;
use bishop::prelude::Vec2;
use engine_core::ecs::ecs::Ecs;
use engine_core::ecs::entity::{get_children, Entity};
use engine_core::ecs::{update_entity_position, SubPixel, Transform};

pub(crate) fn reposition_entity(
    game_instance: &mut GameInstance,
    entity: Entity,
    target_position: Vec2,
) {
    let moved_entities = collect_subtree_entities(&game_instance.game.ecs, entity);
    update_entity_position(&mut game_instance.game.ecs, entity, target_position);

    for moved_entity in moved_entities {
        if let Some(sub_pixel) = game_instance.game.ecs.get_mut::<SubPixel>(moved_entity) {
            sub_pixel.x = 0.0;
            sub_pixel.y = 0.0;
        }

        if let Some(position) = game_instance
            .game
            .ecs
            .get::<Transform>(moved_entity)
            .map(|transform| transform.position)
        {
            game_instance.prev_positions.insert(moved_entity, position);
        }
    }
}

fn collect_subtree_entities(ecs: &Ecs, entity: Entity) -> Vec<Entity> {
    let mut entities = vec![entity];

    for child in get_children(ecs, entity) {
        entities.extend(collect_subtree_entities(ecs, child));
    }

    entities
}

#[cfg(test)]
mod tests {
    use super::*;
    use bishop::prelude::Vec2;
    use engine_core::prelude::*;

    #[test]
    fn reposition_entity_moves_entity_clears_subpixel_and_preserves_velocity() {
        let mut game_instance = crate::engine::game_instance::GameInstance {
            game: Game::default(),
            prev_positions: std::collections::HashMap::new(),
        };
        let entity = game_instance
            .game
            .ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(4.0, 5.0),
                ..Default::default()
            })
            .with(Velocity { x: 3.0, y: -2.0 })
            .with(SubPixel { x: 0.25, y: -0.5 })
            .finish();
        game_instance
            .prev_positions
            .insert(entity, Vec2::new(-8.0, 9.0));

        reposition_entity(&mut game_instance, entity, Vec2::new(22.0, 31.0));

        assert_eq!(
            game_instance
                .game
                .ecs
                .get::<Transform>(entity)
                .map(|transform| transform.position),
            Some(Vec2::new(22.0, 31.0))
        );
        assert_eq!(
            game_instance
                .game
                .ecs
                .get::<Velocity>(entity)
                .map(|velocity| (velocity.x, velocity.y)),
            Some((3.0, -2.0))
        );
        assert_eq!(
            game_instance
                .game
                .ecs
                .get::<SubPixel>(entity)
                .map(|sub_pixel| (sub_pixel.x, sub_pixel.y)),
            Some((0.0, 0.0))
        );
        assert_eq!(
            game_instance.prev_positions.get(&entity).copied(),
            Some(Vec2::new(22.0, 31.0))
        );
    }

    #[test]
    fn reposition_entity_moves_children_with_parent() {
        let mut game_instance = crate::engine::game_instance::GameInstance {
            game: Game::default(),
            prev_positions: std::collections::HashMap::new(),
        };
        let parent = game_instance
            .game
            .ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(10.0, 12.0),
                ..Default::default()
            })
            .finish();
        let child = game_instance
            .game
            .ecs
            .create_entity()
            .with(Transform {
                position: Vec2::new(13.0, 14.0),
                ..Default::default()
            })
            .with(SubPixel { x: 0.5, y: -0.25 })
            .finish();
        set_parent(&mut game_instance.game.ecs, child, parent);
        game_instance
            .prev_positions
            .insert(parent, Vec2::new(10.0, 12.0));
        game_instance
            .prev_positions
            .insert(child, Vec2::new(13.0, 14.0));

        reposition_entity(&mut game_instance, parent, Vec2::new(20.0, 30.0));

        let child = game_instance
            .game
            .ecs
            .get::<Children>(parent)
            .and_then(|children| children.entities.first().copied())
            .unwrap();
        assert_eq!(
            game_instance
                .game
                .ecs
                .get::<Transform>(child)
                .map(|transform| transform.position),
            Some(Vec2::new(23.0, 32.0))
        );
        assert_eq!(
            game_instance
                .game
                .ecs
                .get::<SubPixel>(child)
                .map(|sub_pixel| (sub_pixel.x, sub_pixel.y)),
            Some((0.0, 0.0))
        );
        assert_eq!(
            game_instance.prev_positions.get(&child).copied(),
            Some(Vec2::new(23.0, 32.0))
        );
    }
}
