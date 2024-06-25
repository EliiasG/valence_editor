use std::{
    fs,
    path::{Path, PathBuf},
};

use valence::{
    command::{parsers::GreedyString, AddCommand, CommandScopeRegistry},
    math::IVec3,
    prelude::*,
    text::color::NamedColor,
};

use command::handler::CommandResultEvent;
use command_macros::Command;
use valence::{command, command_macros};

use crate::{origin::Origin, section::Section, structure::Structure, Bounds};
pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            |mut command_scopes: ResMut<CommandScopeRegistry>| {
                command_scopes.link("valence.admin", "valence.command")
            },
        )
        .add_command::<OriginCommand>()
        .add_command::<SaveCommand>()
        .add_command::<LoadCommand>()
        .add_command::<PathCommand>()
        .add_command::<NewCommand>()
        .add_systems(
            Update,
            (
                handle_origin_command,
                handle_save_command,
                handle_load_command,
                handle_path_command,
                handle_new_command,
            ),
        )
        .insert_resource(CurrentPath(None));
    }
}

#[derive(Command, Debug, Clone)]
#[paths("save {path?}", "s {path?}")]
#[scopes("valence.command.save")]
struct SaveCommand {
    path: Option<GreedyString>,
}

#[derive(Command, Debug, Clone)]
#[paths("load {path}", "l {path}")]
#[scopes("valence.command.load")]
struct LoadCommand {
    path: GreedyString,
}

#[derive(Command, Debug, Clone)]
#[paths("path", "p")]
#[scopes("valence.command.path")]
struct PathCommand;

#[derive(Command, Debug, Clone)]
#[paths("new")]
#[scopes("valence.command.new")]
struct NewCommand;

#[derive(Resource)]
struct CurrentPath(Option<PathBuf>);

#[derive(Command, Debug, Clone)]
#[paths("origin", "o")]
#[scopes("valence.command.origin")]
enum OriginCommand {
    #[paths("up", "u")]
    Up,
    #[paths("down", "d")]
    Down,
    #[paths("north", "n")]
    North,
    #[paths("south", "s")]
    South,
    #[paths("east", "e")]
    East,
    #[paths("west", "w")]
    West,
    #[paths("forward", "f")]
    Forward,
    #[paths("back", "b")]
    Back,
    #[paths("here", "h")]
    Here,
}

fn handle_origin_command(
    mut events: EventReader<CommandResultEvent<OriginCommand>>,
    mut origin: Query<&mut Origin>,
    sender: Query<(&Look, &Position)>,
) {
    let mut origin = origin.single_mut();

    for event in events.read() {
        let (look, pos) = match sender.get(event.executor) {
            Ok(v) => v,
            Err(_) => continue,
        };
        origin.position = match event.result {
            OriginCommand::Up => origin.position.offset(0, 1, 0),
            OriginCommand::Down => origin.position.offset(0, -1, 0),
            OriginCommand::North => origin.position.offset(0, 0, -1),
            OriginCommand::South => origin.position.offset(0, 0, 1),
            OriginCommand::East => origin.position.offset(0, 0, -1),
            OriginCommand::West => origin.position.offset(-1, 0, 0),
            OriginCommand::Forward => origin.position.get_in_direction(super::look_to_dir(look)),
            OriginCommand::Back => {
                origin
                    .position
                    .get_in_direction(match super::look_to_dir(look) {
                        Direction::East => Direction::West,
                        Direction::West => Direction::East,
                        Direction::North => Direction::South,
                        _ => Direction::North,
                    })
            }
            OriginCommand::Here => BlockPos::new(
                pos.x.floor() as i32,
                pos.y.floor() as i32,
                pos.z.floor() as i32,
            ),
        }
    }
}

fn handle_save_command(
    mut events: EventReader<CommandResultEvent<SaveCommand>>,
    origin: Query<&Origin>,
    section: Query<&Section, With<Bounds>>,
    layer: Query<&ChunkLayer>,
    mut sender: Query<&mut Client>,
    mut current_path: ResMut<CurrentPath>,
) {
    let origin = origin.single();
    let section = section.single();
    let layer = layer.single();
    for event in events.read() {
        let mut client = match sender.get_mut(event.executor) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Some(path) = event
            .result
            .path
            .as_ref()
            .map(|str| string_to_path_buf(&str.0))
            .or(current_path.0.clone())
        {
            let structure =
                Structure::from_section(layer, section.position, section.size, origin.position);

            if let Err(e) = fs::create_dir_all(&path.parent().unwrap_or(Path::new("")))
                .or(fs::write(&path, structure.serialize()))
            {
                client_error(
                    &mut client,
                    format!("an error occured while trying to save: {}", e),
                );
            } else {
                client_info(
                    &mut client,
                    format!("saved structure to '{}'", path.display()),
                )
            }

            current_path.0 = Some(path);
        } else {
            client_error(
                &mut client,
                "you must specify a path the first time you save".into(),
            )
        }
    }
}

fn handle_load_command(
    mut events: EventReader<CommandResultEvent<LoadCommand>>,
    mut origin: Query<&mut Origin>,
    mut section: Query<&mut Section, With<Bounds>>,
    mut layer: Query<&mut ChunkLayer>,
    mut sender: Query<&mut Client>,
    mut current_path: ResMut<CurrentPath>,
) {
    let mut origin = origin.single_mut();
    let mut section = section.single_mut();
    let mut layer = layer.single_mut();
    for event in events.read() {
        let mut client = match sender.get_mut(event.executor) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let path = string_to_path_buf(&event.result.path.0);
        let data = match fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                client_error(
                    &mut client,
                    format!("error while trying to read structure data: {}", e),
                );
                continue;
            }
        };
        let structure = match Structure::deserialize(&data) {
            Ok(s) => s,
            Err(e) => {
                client_error(
                    &mut client,
                    format!("error while trying serialize structure: {}", e),
                );
                continue;
            }
        };
        load_structure(&mut origin, &mut section, &mut layer, &structure);
        client_info(
            &mut client,
            format!("loaded structure '{}'", path.display()),
        );
        current_path.0 = Some(path);
    }
}

fn load_structure(
    origin: &mut Origin,
    section: &mut Section,
    layer: &mut ChunkLayer,
    structure: &Structure,
) {
    clear(layer, section);
    origin.position = BlockPos::new(0, super::SPAWN_Y, 0);
    structure.render_to_layer(layer, origin.position);
    section.size = structure.size;
    section.position = origin.position - structure.origin_pos;
}

fn clear(layer: &mut ChunkLayer, section: &mut Section) {
    for x in 0..section.size.x {
        for y in 0..section.size.y {
            for z in 0..section.size.z {
                let pos = section.position + IVec3::new(x, y, z);
                layer.set_block(pos, BlockState::AIR);
            }
        }
    }
}

fn handle_path_command(
    mut events: EventReader<CommandResultEvent<PathCommand>>,
    mut sender: Query<&mut Client>,
    current_path: Res<CurrentPath>,
) {
    for event in events.read() {
        let message = if let Some(path) = &current_path.0 {
            format!("current path: '{}'", path.display())
        } else {
            "no path selected".into()
        };
        let _ = sender
            .get_mut(event.executor)
            .map(|mut client| client_info(&mut client, message));
    }
}

fn handle_new_command(
    mut events: EventReader<CommandResultEvent<NewCommand>>,
    mut origin: Query<&mut Origin>,
    mut section: Query<&mut Section, With<Bounds>>,
    mut layer: Query<&mut ChunkLayer>,
    mut sender: Query<&mut Client>,
    mut current_path: ResMut<CurrentPath>,
) {
    let mut origin = origin.single_mut();
    let mut section = section.single_mut();
    let mut layer = layer.single_mut();
    for event in events.read() {
        let mut client = match sender.get_mut(event.executor) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let structure = Structure {
            size: IVec3::new(1, 1, 1),
            origin_pos: IVec3::ZERO,
            blocks: vec![BlockState::BEDROCK],
        };
        load_structure(&mut origin, &mut section, &mut layer, &structure);
        client_info(&mut client, "created new structure".into());
        current_path.0 = None;
    }
}

fn string_to_path_buf(string: &str) -> PathBuf {
    let mut buf = Path::new(string).to_path_buf();
    if buf.extension().map(|ext| ext == "vstruc").unwrap_or(false) {
        buf
    } else {
        buf.set_extension("vstruc");
        buf
    }
}

fn client_error(client: &mut Client, message: String) {
    client.send_chat_message(
        "[Error] "
            .color(NamedColor::DarkRed)
            .bold()
            .add_child(message.not_bold().color(NamedColor::Red)),
    );
}

fn client_info(client: &mut Client, message: String) {
    client.send_chat_message(
        "[Info] "
            .color(NamedColor::White)
            .bold()
            .add_child(message.not_bold().color(Color::Reset)),
    );
}
