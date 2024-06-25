#![allow(clippy::type_complexity)]

use commands::CommandPlugin;
use origin::OriginPlugin;
use section::{Section, SectionPlugin};
use valence::command::scopes::CommandScopes;
use valence::interact_block::InteractBlockEvent;
use valence::inventory::HeldItem;
use valence::math::IVec3;
use valence::op_level::OpLevel;
use valence::prelude::*;
use valence::spawn::IsFlat;
use valence_vstruc as structure;

mod commands;
mod origin;
mod section;
//mod structure;
const SPAWN_Y: i32 = 64;

#[derive(Component)]
struct Bounds;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SectionPlugin)
        .add_plugins(OriginPlugin)
        .add_plugins(CommandPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                digging,
                place_blocks,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -15..15 {
        for x in -15..15 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    layer.chunk.set_block([0, SPAWN_Y, 0], BlockState::BEDROCK);

    let layer_entity = commands.spawn(layer).id();

    origin::spawn_origin(&mut commands, layer_entity, BlockPos::new(0, SPAWN_Y, 0));

    commands.spawn((
        Section {
            position: BlockPos::new(0, SPAWN_Y, 0),
            layer: EntityLayerId(layer_entity),
            corner_block: BlockState::LIME_CONCRETE,
            wall_block: BlockState::LIME_CONCRETE,
            corner_size: 0.15,
            wall_width: 0.15,
            glowing_walls: false,
            glowing_corners: false,
            ..Default::default()
        },
        Bounds,
    ));
}

fn init_clients(
    mut clients: Query<
        (
            &mut Client,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
            &mut IsFlat,
            &mut CommandScopes,
            &mut OpLevel,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut client,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
        mut is_flat,
        mut permissions,
        mut op_level,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.5, f64::from(SPAWN_Y) + 1.0, 0.5]);
        *game_mode = GameMode::Creative;
        is_flat.0 = true;
        permissions.add("valence.admin");
        op_level.set(4);
    }
}

fn digging(
    clients: Query<&GameMode>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<DiggingEvent>,
    mut bounds: Query<&mut Section, With<Bounds>>,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        let Ok(game_mode) = clients.get(event.client) else {
            continue;
        };

        if (*game_mode == GameMode::Creative && event.state == DiggingState::Start)
            || (*game_mode == GameMode::Survival && event.state == DiggingState::Stop)
        {
            layer.set_block(event.position, BlockState::AIR);
        }
        shrink(&mut bounds.single_mut(), &layer)
    }
}

fn place_blocks(
    mut clients: Query<(&Inventory, &HeldItem, &Look)>,
    mut bounds: Query<&mut Section, With<Bounds>>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<InteractBlockEvent>,
) {
    let mut layer = layers.single_mut();

    for event in events.read() {
        let Ok((inventory, held, look)) = clients.get_mut(event.client) else {
            continue;
        };
        if event.hand != Hand::Main {
            continue;
        }

        // get the held item
        let slot_id = held.slot();
        let stack = inventory.slot(slot_id);
        if stack.is_empty() {
            // no item in the slot
            continue;
        };

        let Some(block_kind) = BlockKind::from_item_kind(stack.item) else {
            // can't place this item as a block
            continue;
        };
        let real_pos = event.position.get_in_direction(event.face);

        let half = if event.face == Direction::Up {
            PropValue::Bottom
        } else if event.face == Direction::Down {
            PropValue::Top
        } else if event.cursor_pos.y < 0.5 {
            PropValue::Bottom
        } else {
            PropValue::Top
        };
        let state = block_kind
            .to_state()
            .set(
                PropName::Axis,
                match event.face {
                    Direction::Down | Direction::Up => PropValue::Y,
                    Direction::North | Direction::South => PropValue::Z,
                    Direction::West | Direction::East => PropValue::X,
                },
            )
            .set(
                PropName::Facing,
                match look_to_dir(look) {
                    Direction::South => PropValue::South,
                    Direction::West => PropValue::West,
                    Direction::East => PropValue::East,
                    _ => PropValue::North,
                },
            )
            .set(PropName::Half, half)
            .set(PropName::Type, half);
        //.set(PropName::Facing, event.client);

        layer.set_block(real_pos, state);
        include(&mut bounds.single_mut(), real_pos);
    }
}

pub fn look_to_dir(look: &Look) -> Direction {
    let dir = look.yaw % 360.0;
    let dir = if dir < 0.0 { dir + 360.0 } else { dir };
    let mut block_dir = Direction::North;
    if (0.0..45.0).contains(&dir) || (315.0..360.0).contains(&dir) {
        block_dir = Direction::South;
    }
    if (45.0..135.0).contains(&dir) {
        block_dir = Direction::West;
    }
    if (225.0..315.0).contains(&dir) {
        block_dir = Direction::East;
    }
    block_dir
}

fn shrink(section: &mut Section, layer: &ChunkLayer) {
    for (dir, start, size) in [
        (
            IVec3::new(1, 0, 0),
            IVec3::ZERO,
            IVec3::new(1, section.size.y, section.size.z),
        ),
        (
            IVec3::new(-1, 0, 0),
            IVec3::new(section.size.x - 1, 0, 0),
            IVec3::new(1, section.size.y, section.size.z),
        ),
        (
            IVec3::new(0, 1, 0),
            IVec3::ZERO,
            IVec3::new(section.size.x, 1, section.size.z),
        ),
        (
            IVec3::new(0, -1, 0),
            IVec3::new(0, section.size.y - 1, 0),
            IVec3::new(section.size.x, 1, section.size.z),
        ),
        (
            IVec3::new(0, 0, 1),
            IVec3::ZERO,
            IVec3::new(section.size.x, section.size.y, 1),
        ),
        (
            IVec3::new(0, 0, -1),
            IVec3::new(0, 0, section.size.z - 1),
            IVec3::new(section.size.x, section.size.y, 1),
        ),
    ] {
        if has_block(layer, section.position + start, size) {
            continue;
        }
        section.size -= dir.abs();
        if section.size.x == 0 {
            section.size.x = 1;
            return;
        }
        if section.size.y == 0 {
            section.size.y = 1;
            return;
        }
        if section.size.z == 0 {
            section.size.z = 1;
            return;
        }
        if dir.x == 1 || dir.y == 1 || dir.z == 1 {
            section.position = section.position + dir;
        }
        for _ in 0..6 {
            shrink(section, layer)
        }
        return;
    }
}

fn has_block(layer: &ChunkLayer, start: BlockPos, size: IVec3) -> bool {
    for x in 0..size.x {
        for y in 0..size.y {
            for z in 0..size.z {
                let pos = start + IVec3::new(x, y, z);
                let block = layer
                    .block(pos)
                    .map(|block| block.state != BlockState::AIR)
                    .unwrap_or(false);
                if block {
                    return true;
                }
            }
        }
    }
    false
}

fn include(section: &mut Section, pos: BlockPos) {
    let corner = section.position + (section.size - 1);
    let pos = IVec3::new(pos.x, pos.y, pos.z);
    let diff = pos - corner;
    section.size += IVec3::new(diff.x, diff.y, diff.z).max(IVec3::ZERO);
    let diff = section.position - pos;
    let diff = IVec3::new(diff.x, diff.y, diff.z).max(IVec3::ZERO);
    section.position = section.position - diff;
    section.size += diff;
}
