use valence::{
    entity::{
        block_display::{self, BlockDisplayEntityBundle},
        display::Scale,
        entity::Flags,
    },
    prelude::*,
};

pub struct OriginPlugin;

impl Plugin for OriginPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, origin_system);
    }
}

/// marker component for the origin
#[derive(Component)]
pub struct Origin {
    pub position: BlockPos,
}

pub fn spawn_origin(commands: &mut Commands, layer: Entity, position: BlockPos) {
    let mut entity_flags = Flags::default();
    entity_flags.set_glowing(true);

    commands.spawn((
        BlockDisplayEntityBundle {
            block_display_block_state: block_display::BlockState(BlockState::RED_CONCRETE),
            display_scale: Scale(Vec3::new(0.35, 0.35, 0.35)),
            entity_flags,
            layer: EntityLayerId(layer),
            ..Default::default()
        },
        Origin { position },
    ));
}

fn origin_system(mut query: Query<(&mut Position, &Origin, &Scale)>) {
    for (mut position, origin, scale) in query.iter_mut() {
        position.0 = DVec3::new(
            origin.position.x as f64 + 0.5,
            origin.position.y as f64 + 0.5,
            origin.position.z as f64 + 0.5,
        ) - DVec3::from(scale.0) * 0.5;
    }
}
