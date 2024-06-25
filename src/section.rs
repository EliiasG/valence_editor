use valence::{
    advancement::bevy_hierarchy::{BuildChildren, Children},
    entity::{
        block_display::{self, BlockDisplayEntityBundle},
        display::Scale,
        entity::Flags,
    },
    math::IVec3,
    prelude::*,
};

pub struct SectionPlugin;

impl Plugin for SectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_sections);
    }
}

#[derive(Component)]
pub struct Section {
    pub position: BlockPos,
    pub size: IVec3,
    pub layer: EntityLayerId,
    pub wall_width: f32,
    pub wall_block: BlockState,
    pub corner_size: f32,
    pub corner_block: BlockState,
    pub glowing_walls: bool,
    pub glowing_corners: bool,
}

impl Default for Section {
    fn default() -> Self {
        Self {
            position: BlockPos::new(0, 0, 0),
            size: IVec3::splat(1),
            layer: EntityLayerId::default(),
            wall_width: 0.25,
            wall_block: BlockState::WHITE_CONCRETE,
            corner_size: 0.35,
            corner_block: BlockState::RED_CONCRETE,
            glowing_walls: true,
            glowing_corners: true,
        }
    }
}

#[derive(Component)]
struct SectionWall(u8);

#[derive(Component)]
struct SectionCorner(u8);

fn update_sections(
    mut commands: Commands,
    mut sections: Query<
        (Entity, &Section, Option<&Children>),
        Or<(Changed<Section>, Changed<Children>)>,
    >,
    mut walls: Query<(
        &SectionWall,
        &mut block_display::BlockState,
        &mut Position,
        &mut Scale,
        &mut Flags,
        &mut EntityLayerId,
    )>,
    mut corners: Query<
        (
            &SectionCorner,
            &mut block_display::BlockState,
            &mut Position,
            &mut Scale,
            &mut Flags,
            &mut EntityLayerId,
        ),
        Without<SectionWall>,
    >,
) {
    for (entity, section, children) in &mut sections {
        if children.is_none() {
            commands.entity(entity).with_children(|builder| {
                for i in 0..12 {
                    builder.spawn((BlockDisplayEntityBundle::default(), SectionWall(i)));
                }
                for i in 0..8 {
                    builder.spawn((BlockDisplayEntityBundle::default(), SectionCorner(i)));
                }
            });
            continue;
        }
        let children = children.unwrap();
        for child in children.iter() {
            // walls
            if let Ok((
                wall,
                mut wall_state,
                mut wall_pos,
                mut wall_scale,
                mut wall_flags,
                mut wall_layer,
            )) = walls.get_mut(*child)
            {
                let mut entity_flags = Flags::default();
                entity_flags.set_glowing(section.glowing_walls);
                *wall_state = block_display::BlockState(section.wall_block);
                *wall_flags = entity_flags;
                *wall_layer = section.layer;
                *wall_scale = match wall.0 / 4 {
                    0 => Scale(Vec3::new(
                        section.wall_width,
                        section.size.y as f32,
                        section.wall_width,
                    )),
                    1 => Scale(Vec3::new(
                        section.size.x as f32,
                        section.wall_width,
                        section.wall_width,
                    )),
                    2 => Scale(Vec3::new(
                        section.wall_width,
                        section.wall_width,
                        section.size.z as f32,
                    )),
                    _ => Scale(Vec3::ZERO),
                };
                let a = wall.0 % 4 / 2;
                let b = wall.0 % 2;
                *wall_pos = Position(
                    match wall.0 / 4 {
                        0 => center_wall(
                            DVec3::new(
                                (a as i32 * section.size.x) as f64,
                                (section.size.y) as f64 / 2.0,
                                (b as i32 * section.size.z) as f64,
                            ),
                            wall_scale.0.into(),
                        ),
                        1 => center_wall(
                            DVec3::new(
                                (section.size.x) as f64 / 2.0,
                                (a as i32 * section.size.y) as f64,
                                (b as i32 * section.size.z) as f64,
                            ),
                            wall_scale.0.into(),
                        ),
                        2 => center_wall(
                            DVec3::new(
                                (b as i32 * section.size.x) as f64,
                                (a as i32 * section.size.y) as f64,
                                (section.size.z) as f64 / 2.0,
                            ),
                            wall_scale.0.into(),
                        ),
                        _ => DVec3::ZERO,
                    } + DVec3::new(
                        section.position.x as f64,
                        section.position.y as f64,
                        section.position.z as f64,
                    ),
                );
            }

            // corners
            if let Ok((
                corner,
                mut corner_state,
                mut corner_pos,
                mut corner_scale,
                mut corner_flags,
                mut corner_layer,
            )) = corners.get_mut(*child)
            {
                let mut entity_flags = Flags::default();
                entity_flags.set_glowing(section.glowing_corners);
                *corner_state = block_display::BlockState(section.corner_block);
                *corner_flags = entity_flags;
                *corner_layer = section.layer;
                *corner_scale = Scale(Vec3::splat(section.corner_size));
                *corner_pos = center_corner(
                    section.position
                        + IVec3::new(
                            (corner.0 as i32 % 2) * section.size.x,
                            (corner.0 as i32 / 4) * section.size.y,
                            ((corner.0 / 2) as i32 % 2) * section.size.z,
                        ),
                    section.corner_size,
                );
            }
        }
    }
}

fn center_corner(pos: BlockPos, size: f32) -> Position {
    let offset = -size as f64 * 0.5;
    Position(
        [
            pos.x as f64 + offset,
            pos.y as f64 + offset,
            pos.z as f64 + offset,
        ]
        .into(),
    )
}

fn center_wall(pos: DVec3, size: DVec3) -> DVec3 {
    pos - size / 2.0
}
