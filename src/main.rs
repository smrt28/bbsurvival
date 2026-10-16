mod world;

use bevy::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};
use world::{Cell, STEP_SECONDS, Tile, World, cell_hash};

const CELL_SIZE: f32 = 32.0;
const VIEW_X: i64 = 18;
const VIEW_Y: i64 = 14;

#[derive(Resource)]
struct Game {
    world: World,
    last_tick: f64,
}

#[derive(Resource)]
struct TileArt {
    dirt: Handle<Mesh>,
    stone: Handle<Mesh>,
    creature: Handle<Mesh>,
    dirt_colors: Vec<Handle<ColorMaterial>>,
    stone_color: Handle<ColorMaterial>,
    creature_color: Handle<ColorMaterial>,
}

#[derive(Component)]
struct TileVisual(Cell);

#[derive(Component)]
struct PlayerVisual;

#[derive(Component)]
struct StatusText;

fn main() {
    let seed = std::env::args()
        .nth(1)
        .map(|argument| argument.parse::<u64>().expect("seed must be a u64 number"))
        .unwrap_or_else(new_seed);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Boulder Dash Survival".into(),
                resolution: (1024, 768).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.045, 0.065, 0.09)))
        .insert_resource(Time::<Fixed>::from_seconds(STEP_SECONDS))
        .insert_resource(Game {
            world: World::new(seed),
            last_tick: 0.0,
        })
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, simulate)
        .add_systems(Update, (controls, render_world, update_status).chain())
        .run();
}

fn new_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    let dirt = meshes.add(Rectangle::new(CELL_SIZE - 1.0, CELL_SIZE - 1.0));
    let stone = meshes.add(RegularPolygon::new(14.5, 8));
    let creature = meshes.add(RegularPolygon::new(13.0, 3));
    let dirt_colors: Vec<_> = (0..5)
        .map(|shade| {
            let shade = shade as f32 * 0.018;
            materials.add(Color::srgb(0.39 + shade, 0.23 + shade, 0.12 + shade))
        })
        .collect();
    let stone_color = materials.add(Color::srgb(0.57, 0.64, 0.70));
    let creature_color = materials.add(Color::srgb(0.83, 0.43, 0.95));
    for y in -VIEW_Y..=VIEW_Y {
        for x in -VIEW_X..=VIEW_X {
            commands.spawn((
                TileVisual(Cell::new(x, y)),
                Mesh2d(dirt.clone()),
                MeshMaterial2d(dirt_colors[0].clone()),
                Transform::default(),
            ));
        }
    }
    commands.insert_resource(TileArt {
        dirt,
        stone,
        creature,
        dirt_colors,
        stone_color,
        creature_color,
    });

    let body = meshes.add(Rectangle::new(20.0, 23.0));
    let helmet = meshes.add(Circle::new(11.0));
    let eye = meshes.add(Circle::new(2.0));
    let teal = materials.add(Color::srgb(0.15, 0.82, 0.72));
    let gold = materials.add(Color::srgb(1.0, 0.76, 0.23));
    let ink = materials.add(Color::srgb(0.04, 0.09, 0.13));
    commands
        .spawn((
            PlayerVisual,
            Transform::from_xyz(0.0, 0.0, 5.0),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh2d(body),
                MeshMaterial2d(teal),
                Transform::from_xyz(0.0, -3.0, 0.0),
            ));
            parent.spawn((
                Mesh2d(helmet),
                MeshMaterial2d(gold),
                Transform::from_xyz(0.0, 5.0, 0.1),
            ));
            for x in [-4.0, 4.0] {
                parent.spawn((
                    Mesh2d(eye.clone()),
                    MeshMaterial2d(ink.clone()),
                    Transform::from_xyz(x, 3.0, 0.2),
                ));
            }
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                padding: UiRect::all(px(14)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.04, 0.06, 0.94)),
        ))
        .with_children(|parent| {
            parent.spawn((
                StatusText,
                Text::new(""),
                TextFont {
                    font_size: 17.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.94, 0.97)),
            ));
        });
}

fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    fixed: Res<Time<Fixed>>,
    mut game: ResMut<Game>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    if keys.just_pressed(KeyCode::KeyR) || keys.just_pressed(KeyCode::KeyN) {
        let seed = if keys.just_pressed(KeyCode::KeyN) {
            new_seed()
        } else {
            game.world.seed
        };
        game.world = World::new(seed);
        game.last_tick = fixed.elapsed_secs_f64() + fixed.overstep().as_secs_f64();
    }
}

fn simulate(keys: Res<ButtonInput<KeyCode>>, fixed: Res<Time<Fixed>>, mut game: ResMut<Game>) {
    if game.world.dead {
        return;
    }
    let direction = if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        Cell::new(0, 1)
    } else if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        Cell::new(0, -1)
    } else if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        Cell::new(-1, 0)
    } else if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        Cell::new(1, 0)
    } else {
        Cell::default()
    };
    game.world.step(direction);
    game.last_tick = fixed.elapsed_secs_f64();
}

fn render_world(
    game: Res<Game>,
    fixed: Res<Time<Fixed>>,
    art: Res<TileArt>,
    mut tiles: Query<(
        &TileVisual,
        &mut Mesh2d,
        &mut MeshMaterial2d<ColorMaterial>,
        &mut Transform,
        &mut Visibility,
    )>,
    mut player: Query<&mut Visibility, (With<PlayerVisual>, Without<TileVisual>)>,
) {
    let world = &game.world;
    // Keep the camera at the origin and render relative to the player. Large
    // world coordinates never become imprecise f32 absolute positions.
    let alpha = ((fixed.elapsed_secs_f64() + fixed.overstep().as_secs_f64() - game.last_tick)
        / STEP_SECONDS)
        .clamp(0.0, 1.0) as f32;
    let camera_offset = relative(world.previous_player, world.player) * (1.0 - alpha);
    let moving_stones: std::collections::HashMap<_, _> = world
        .stone_motion
        .iter()
        .map(|motion| (motion.to, motion.from))
        .collect();
    let creatures: std::collections::HashMap<_, _> = world
        .creatures
        .iter()
        .map(|creature| (creature.position, creature))
        .collect();

    for (visual, mut mesh, mut material, mut transform, mut visibility) in &mut tiles {
        let cell = world.player.offset(visual.0.x, visual.0.y);
        let tile = world.tile(cell);
        let creature = creatures.get(&cell);
        if creature.is_none() && matches!(tile, None | Some(Tile::Empty)) {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Inherited;
        let mut position = relative(cell, world.player);
        transform.rotation = Quat::IDENTITY;
        let mut depth = 0.0;
        if let Some(creature) = creature {
            mesh.0 = art.creature.clone();
            material.0 = art.creature_color.clone();
            position += relative(creature.previous_position, cell) * (1.0 - alpha);
            transform.rotation = Quat::from_rotation_z(
                (creature.heading.y as f32).atan2(creature.heading.x as f32)
                    - std::f32::consts::FRAC_PI_2,
            );
            depth = 2.0;
        } else if tile == Some(Tile::Stone) {
            mesh.0 = art.stone.clone();
            material.0 = art.stone_color.clone();
            if let Some(from) = moving_stones.get(&cell) {
                position += relative(*from, cell) * (1.0 - alpha);
            }
        } else {
            mesh.0 = art.dirt.clone();
            material.0 = art.dirt_colors[(cell_hash(world.seed, cell) % 5) as usize].clone();
        }
        transform.translation = (position - camera_offset).extend(depth);
    }
    for mut visibility in &mut player {
        *visibility = if world.dead && alpha >= 1.0 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

fn relative(cell: Cell, origin: Cell) -> Vec2 {
    Vec2::new((cell.x - origin.x) as f32, (cell.y - origin.y) as f32) * CELL_SIZE
}

fn update_status(game: Res<Game>, mut text: Query<&mut Text, With<StatusText>>) {
    if !game.is_changed() {
        return;
    }
    for mut text in &mut text {
        text.0 = format!(
            "BOULDER DASH SURVIVAL   |   {}\nSeed: {}   Position: {}, {}   Chunks: {}\nWASD / arrows: move & dig    R: restart    N: new world    Esc: quit",
            if game.world.dead {
                "CRUSHED! Press R to retry"
            } else {
                "Explore. Dig. Watch the stones above you."
            },
            game.world.seed,
            game.world.player.x,
            game.world.player.y,
            game.world.chunk_count(),
        );
    }
}
