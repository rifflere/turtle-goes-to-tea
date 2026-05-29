use bevy::{prelude::*, sprite::ColorMaterial};

// ── Gameplay constants ────────────────────────────────────────────────────────
const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;
const TURTLE_RADIUS: f32 = 25.0;
const TEA_SIZE: f32 = 40.0;
// Rhythm movement — all distances in pixels, all times in seconds.
//
// BASE_STEP is sized so the first-frame burst ≈ half the turtle's diameter.
// burst = step_size * (1 - FRICTION_FREE) ≈ step_size * 0.4 → 60 * 0.4 = 24 px ≈ turtle radius.
const BASE_STEP: f32 = 60.0;  // pixels per tap at neutral rhythm
const MIN_STEP: f32 = 5.0;    // fully-degraded step (mashing causes this)
const MAX_STEP: f32 = 100.0;  // peak step for sustained good rhythm
const STEP_REWARD: f32 = 10.0;  // bonus per sweet-zone tap  (~+17 % of base)
const STEP_PENALTY: f32 = 10.0; // penalty per too-fast tap  (~-17 % of base)
const TOO_FAST_SECS: f64 = 0.20;     // < 200 ms between taps = mashing
const SWEET_LO_SECS: f64 = 0.30;     // sweet zone: 300 – 700 ms
const SWEET_HI_SECS: f64 = 0.70;
const RHYTHM_RESET_SECS: f64 = 1.50; // pause longer than this → step resets to base
// Friction coefficients (per frame at 60 fps).
// FRICTION_FREE = 0.60 → glide decays in ~4 frames, making each tap feel like a crisp hop.
// FRICTION_HELD = 0.20 → near-instant brake when holding.
const FRICTION_HELD: f32 = 0.20;
const FRICTION_FREE: f32 = 0.60;
// Velocity multiplier: converts step_size (px) to initial velocity (px/s).
// Derived so that total glide distance ≈ step_size pixels.
// total = velocity / (60 * (1 - FRICTION_FREE)) → velocity = step_size * 60 * (1 - FRICTION_FREE)
const VELOCITY_SCALE: f32 = 60.0 * (1.0 - FRICTION_FREE);

// ── App state ─────────────────────────────────────────────────────────────────
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum AppState {
    #[default]
    Menu,
    Playing,
    Win,
}

// ── Components ────────────────────────────────────────────────────────────────
#[derive(Component)] struct Turtle;
#[derive(Component)] struct Tea;

#[derive(Component)] struct GameEntity;

#[derive(Component)] struct MenuRoot;
#[derive(Component)] struct WinRoot;

#[derive(Component)] struct TitleText;

#[derive(Component)]
struct MenuItem {
    index: usize,
}

#[derive(Component, Clone, Copy)]
enum ButtonAction {
    Play,
    PlayAgain,
    MainMenu,
    Exit,
}

// ── Resources ─────────────────────────────────────────────────────────────────
#[derive(Resource)]
struct MenuNav {
    index: usize,
    max: usize,
}

#[derive(Resource)]
struct TurtleMovement {
    velocity: f32,
    step_size: f32,
    last_tap_time: f64,
}

impl Default for TurtleMovement {
    fn default() -> Self {
        Self {
            velocity: 0.0,
            step_size: BASE_STEP,
            last_tap_time: 0.0,
        }
    }
}

fn on_menu_or_win(state: Res<State<AppState>>) -> bool {
    matches!(state.get(), AppState::Menu | AppState::Win)
}

// ── Entry point ───────────────────────────────────────────────────────────────
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Turtle Goes to Tea".into(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.22, 0.60, 0.22)))
        .init_state::<AppState>()
        .init_resource::<TurtleMovement>()
        .add_systems(Startup, spawn_camera)
        .add_systems(OnEnter(AppState::Menu), spawn_menu)
        .add_systems(OnExit(AppState::Menu), cleanup::<MenuRoot>)
        .add_systems(OnEnter(AppState::Playing), spawn_game)
        .add_systems(OnExit(AppState::Playing), cleanup::<GameEntity>)
        .add_systems(OnEnter(AppState::Win), spawn_win_screen)
        .add_systems(OnExit(AppState::Win), cleanup::<WinRoot>)
        .add_systems(
            Update,
            animate_title.run_if(in_state(AppState::Menu)),
        )
        .add_systems(
            Update,
            (menu_navigation, update_button_visuals, wobble_buttons)
                .run_if(on_menu_or_win),
        )
        .add_systems(
            Update,
            (move_turtle, check_collision)
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        .run();
}

// ── Startup ───────────────────────────────────────────────────────────────────
fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn cleanup<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

// ── Menu screen ───────────────────────────────────────────────────────────────
fn spawn_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(MenuNav { index: 0, max: 2 });

    let font = asset_server.load("fonts/whimsical.ttf");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(22.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.14, 0.43, 0.14)),
            MenuRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Turtle Goes to Tea"),
                TextFont {
                    font: font.clone(),
                    font_size: 72.0,
                    ..default()
                },
                TextColor(Color::srgb(0.96, 0.82, 0.22)),
                Node {
                    margin: UiRect::bottom(Val::Px(48.0)),
                    ..default()
                },
                TitleText,
            ));

            spawn_button(root, "Play", 0, ButtonAction::Play, font.clone());
            spawn_button(root, "Exit", 1, ButtonAction::Exit, font.clone());
        });
}

// ── Win screen ────────────────────────────────────────────────────────────────
fn spawn_win_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(MenuNav { index: 0, max: 3 });

    let font = asset_server.load("fonts/whimsical.ttf");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(22.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.14, 0.43, 0.14)),
            WinRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Tea time!"),
                TextFont {
                    font: font.clone(),
                    font_size: 88.0,
                    ..default()
                },
                TextColor(Color::srgb(0.96, 0.82, 0.22)),
                Node {
                    margin: UiRect::bottom(Val::Px(36.0)),
                    ..default()
                },
            ));

            spawn_button(root, "Play Again", 0, ButtonAction::PlayAgain, font.clone());
            spawn_button(root, "Main Menu", 1, ButtonAction::MainMenu, font.clone());
            spawn_button(root, "Exit", 2, ButtonAction::Exit, font.clone());
        });
}

fn spawn_button(
    parent: &mut ChildBuilder,
    label: &str,
    index: usize,
    action: ButtonAction,
    font: Handle<Font>,
) {
    parent
        .spawn((
            Node {
                width: Val::Px(250.0),
                height: Val::Px(68.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.88, 0.86, 0.81)),
            BorderColor(Color::srgb(0.76, 0.72, 0.65)),
            BorderRadius::all(Val::Px(10.0)),
            Button,
            MenuItem { index },
            action,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font,
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::srgb(0.22, 0.14, 0.06)),
            ));
        });
}

// ── Menu animation systems ────────────────────────────────────────────────────

fn animate_title(time: Res<Time>, mut query: Query<&mut TextColor, With<TitleText>>) {
    for mut color in &mut query {
        let t = (time.elapsed_secs() * 1.4).sin() * 0.5 + 0.5;
        color.0 = Color::srgb(0.98, 0.72 + t * 0.20, 0.08 + t * 0.14);
    }
}

fn update_button_visuals(nav: Res<MenuNav>, mut query: Query<(&MenuItem, &mut BackgroundColor)>) {
    for (item, mut bg) in &mut query {
        bg.0 = if item.index == nav.index {
            Color::srgb(0.98, 0.96, 0.91)
        } else {
            Color::srgb(0.84, 0.82, 0.77)
        };
    }
}

fn wobble_buttons(
    time: Res<Time>,
    nav: Res<MenuNav>,
    mut query: Query<(&MenuItem, &mut Transform)>,
) {
    for (item, mut transform) in &mut query {
        if item.index == nav.index {
            let angle = (time.elapsed_secs() * 5.5).sin() * 0.045;
            let scale = 1.0 + (time.elapsed_secs() * 5.5).sin().abs() * 0.028;
            transform.rotation = Quat::from_rotation_z(angle);
            transform.scale = Vec3::splat(scale);
        } else {
            transform.rotation = Quat::IDENTITY;
            transform.scale = Vec3::ONE;
        }
    }
}

// ── Menu / win navigation ─────────────────────────────────────────────────────
fn menu_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut nav: ResMut<MenuNav>,
    interaction_query: Query<(&Interaction, &MenuItem), Changed<Interaction>>,
    button_query: Query<(&MenuItem, &ButtonAction)>,
    mut next_state: ResMut<NextState<AppState>>,
    mut app_exit: EventWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        nav.index = nav.index.saturating_sub(1);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        nav.index = (nav.index + 1).min(nav.max - 1);
    }

    for (interaction, item) in &interaction_query {
        if matches!(*interaction, Interaction::Hovered | Interaction::Pressed) {
            nav.index = item.index;
        }
    }

    let confirm_keyboard = keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space);
    let confirm_mouse = interaction_query
        .iter()
        .any(|(i, item)| *i == Interaction::Pressed && item.index == nav.index);

    if confirm_keyboard || confirm_mouse {
        for (item, action) in &button_query {
            if item.index == nav.index {
                match action {
                    ButtonAction::Play | ButtonAction::PlayAgain => {
                        next_state.set(AppState::Playing);
                    }
                    ButtonAction::MainMenu => {
                        next_state.set(AppState::Menu);
                    }
                    ButtonAction::Exit => {
                        app_exit.send(AppExit::Success);
                    }
                }
                break;
            }
        }
    }
}

// ── Backdrop ──────────────────────────────────────────────────────────────────
// Evaluates the road's vertical position at a given x.
// Two low-frequency sine waves give a gentle, natural undulation.
fn road_y_at(x: f32) -> f32 {
    (x * 0.008).sin() * 12.0 + (x * 0.020).sin() * 5.0
}

fn spawn_backdrop(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
) {
    let half_w = WINDOW_WIDTH;
    let half_h = WINDOW_HEIGHT / 2.0;

    // ── Sky ──────────────────────────────────────────────────────────────────
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(half_w, half_h))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.53, 0.81, 0.92)))),
        Transform::from_xyz(0.0, half_h / 2.0, -10.0),
        GameEntity,
    ));

    // ── Ground ───────────────────────────────────────────────────────────────
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(half_w, half_h))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.28, 0.62, 0.18)))),
        Transform::from_xyz(0.0, -half_h / 2.0, -10.0),
        GameEntity,
    ));

    // ── Dirt road ────────────────────────────────────────────────────────────
    // Approximated as a series of overlapping short rectangles, each rotated
    // to follow the local slope of road_y_at(). Sits at z = -8 so it's above
    // the ground quad (-10) and grass (-9) but behind the turtle/tea (0+).
    let road_color = materials.add(ColorMaterial::from(Color::srgb(0.52, 0.36, 0.18)));
    let road_edge_color = materials.add(ColorMaterial::from(Color::srgb(0.42, 0.28, 0.12)));

    let road_width: f32  = 90.0;
    let edge_width: f32  = 6.0;
    let seg_len: f32     = 14.0;
    let seg_overlap: f32 = 2.0;

    let road_start_x = -(WINDOW_WIDTH / 2.0) - seg_len;
    let road_end_x   =   WINDOW_WIDTH / 2.0  + seg_len;
    let mut rx = road_start_x;

    while rx < road_end_x {
        let mid_x = rx + seg_len * 0.5;
        let dy    = road_y_at(rx + 1.0) - road_y_at(rx);
        let angle = dy.atan2(1.0);
        let cy    = road_y_at(mid_x);
        let t     = Transform::from_xyz(mid_x, cy, -8.0)
            .with_rotation(Quat::from_rotation_z(angle));

        // Main dirt fill
        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(seg_len + seg_overlap, road_width))),
            MeshMaterial2d(road_color.clone()),
            t,
            GameEntity,
        ));

        // Darker edge strips along the top and bottom of the road
        let offset = (road_width - edge_width) / 2.0;

        let mut edge_top = t;
        edge_top.translation.y += angle.cos() * offset;
        edge_top.translation.x -= angle.sin() * offset;

        let mut edge_bot = t;
        edge_bot.translation.y -= angle.cos() * offset;
        edge_bot.translation.x += angle.sin() * offset;

        for edge_t in [edge_top, edge_bot] {
            commands.spawn((
                Mesh2d(meshes.add(Rectangle::new(seg_len + seg_overlap, edge_width))),
                MeshMaterial2d(road_edge_color.clone()),
                edge_t,
                GameEntity,
            ));
        }

        rx += seg_len;
    }

    // ── Clouds ───────────────────────────────────────────────────────────────
    // Each cloud is a cluster of overlapping ellipses spawned as children of
    // an invisible parent so the whole thing moves as one unit.
    // z = -9.5 puts clouds behind the grass strip but in front of the sky quad.
    let cloud_color = materials.add(ColorMaterial::from(Color::srgb(0.97, 0.97, 0.98)));

    // (centre_x, centre_y, scale) — y is in the upper half of the screen
    let cloud_defs: &[(f32, f32, f32)] = &[
        (-280.0, 180.0, 1.0),
        (  60.0, 210.0, 0.75),
        ( 260.0, 155.0, 1.2),
    ];

    // Puff offsets relative to each cloud centre: (dx, dy, rx, ry)
    let puffs: &[(f32, f32, f32, f32)] = &[
        (  0.0,  0.0, 38.0, 22.0), // main body
        (-30.0, -6.0, 26.0, 17.0), // left lobe
        ( 30.0, -4.0, 28.0, 18.0), // right lobe
        (-14.0, 10.0, 22.0, 13.0), // left top bump
        ( 14.0, 12.0, 24.0, 14.0), // right top bump
    ];

    for &(cx, cy, scale) in cloud_defs {
        commands
            .spawn((
                Transform::from_xyz(cx, cy, -9.5),
                Visibility::default(),
                GameEntity,
            ))
            .with_children(|p| {
                for &(dx, dy, rx, ry) in puffs {
                    p.spawn((
                        Mesh2d(meshes.add(Ellipse::new(rx * scale, ry * scale))),
                        MeshMaterial2d(cloud_color.clone()),
                        Transform::from_xyz(dx * scale, dy * scale, 0.0),
                    ));
                }
            });
    }
}

// ── Gameplay ──────────────────────────────────────────────────────────────────
fn spawn_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut movement: ResMut<TurtleMovement>,
) {
    *movement = TurtleMovement::default();

    spawn_backdrop(&mut commands, &mut meshes, &mut materials);

    // ── Turtle ───────────────────────────────────────────────────────────────
    let t_body   = materials.add(ColorMaterial::from(Color::srgb(0.20, 0.54, 0.12)));
    let t_shell  = materials.add(ColorMaterial::from(Color::srgb(0.11, 0.34, 0.05)));
    let t_head   = materials.add(ColorMaterial::from(Color::srgb(0.26, 0.64, 0.16)));
    let t_eye    = materials.add(ColorMaterial::from(Color::srgb(0.06, 0.06, 0.06)));

    let m_leg    = meshes.add(Ellipse::new(5.0, 11.0));
    let m_tail   = meshes.add(Ellipse::new(6.0, 4.0));
    let m_body   = meshes.add(Ellipse::new(26.0, 15.0));
    let m_shell  = meshes.add(Ellipse::new(20.0, 13.0));
    let m_neck   = meshes.add(Ellipse::new(7.0, 6.0));
    let m_head   = meshes.add(Circle::new(10.0));
    let m_eye    = meshes.add(Circle::new(2.5));

    commands
        .spawn((
            Transform::from_xyz(-350.0, 0.0, 0.0),
            Visibility::default(),
            Turtle,
            GameEntity,
        ))
        .with_children(|p| {
            for (x, angle) in [(-13.0_f32, 0.35_f32), (-4.0, 0.15),
                                (  8.0_f32,-0.15_f32), (18.0,-0.35)] {
                p.spawn((
                    Mesh2d(m_leg.clone()),
                    MeshMaterial2d(t_body.clone()),
                    Transform::from_xyz(x, -22.0, -0.1)
                        .with_rotation(Quat::from_rotation_z(angle)),
                ));
            }
            p.spawn((
                Mesh2d(m_tail),
                MeshMaterial2d(t_body.clone()),
                Transform::from_xyz(-30.0, -2.0, -0.1),
            ));
            p.spawn((
                Mesh2d(m_body),
                MeshMaterial2d(t_body.clone()),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
            p.spawn((
                Mesh2d(m_shell),
                MeshMaterial2d(t_shell),
                Transform::from_xyz(-2.0, 5.0, 0.1),
            ));
            p.spawn((
                Mesh2d(m_neck),
                MeshMaterial2d(t_body.clone()),
                Transform::from_xyz(22.0, 2.0, 0.05),
            ));
            p.spawn((
                Mesh2d(m_head),
                MeshMaterial2d(t_head),
                Transform::from_xyz(30.0, 4.0, 0.0),
            ));
            p.spawn((
                Mesh2d(m_eye),
                MeshMaterial2d(t_eye),
                Transform::from_xyz(35.5, 7.5, 0.2),
            ));
        });

    // ── Teacup ───────────────────────────────────────────────────────────────
    let c_porcelain = materials.add(ColorMaterial::from(Color::srgb(0.95, 0.92, 0.87)));
    let c_shadow    = materials.add(ColorMaterial::from(Color::srgb(0.76, 0.73, 0.68)));
    let c_tea       = materials.add(ColorMaterial::from(Color::srgb(0.52, 0.28, 0.07)));

    let m_saucer     = meshes.add(Ellipse::new(28.0, 6.0));
    let m_hbar       = meshes.add(Rectangle::new(5.0, 20.0));
    let m_harm       = meshes.add(Rectangle::new(10.0, 5.0));
    let m_cup        = meshes.add(Rectangle::new(30.0, 38.0));
    let m_tea_surf   = meshes.add(Ellipse::new(12.0, 4.5));
    let m_rim        = meshes.add(Rectangle::new(34.0, 6.0));

    commands
        .spawn((
            Transform::from_xyz(300.0, 0.0, 0.0),
            Visibility::default(),
            Tea,
            GameEntity,
        ))
        .with_children(|p| {
            p.spawn((
                Mesh2d(m_saucer),
                MeshMaterial2d(c_shadow.clone()),
                Transform::from_xyz(0.0, -24.0, 0.0),
            ));
            p.spawn((
                Mesh2d(m_hbar),
                MeshMaterial2d(c_porcelain.clone()),
                Transform::from_xyz(22.0, 0.0, -0.1),
            ));
            p.spawn((
                Mesh2d(m_harm.clone()),
                MeshMaterial2d(c_porcelain.clone()),
                Transform::from_xyz(17.0, 8.5, -0.1),
            ));
            p.spawn((
                Mesh2d(m_harm),
                MeshMaterial2d(c_porcelain.clone()),
                Transform::from_xyz(17.0, -8.5, -0.1),
            ));
            p.spawn((
                Mesh2d(m_cup),
                MeshMaterial2d(c_porcelain.clone()),
                Transform::from_xyz(0.0, -3.0, 0.1),
            ));
            p.spawn((
                Mesh2d(m_tea_surf),
                MeshMaterial2d(c_tea),
                Transform::from_xyz(0.0, 14.5, 0.2),
            ));
            p.spawn((
                Mesh2d(m_rim),
                MeshMaterial2d(c_shadow),
                Transform::from_xyz(0.0, 16.0, 0.3),
            ));
        });
}

fn move_turtle(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Turtle>>,
    time: Res<Time>,
    mut movement: ResMut<TurtleMovement>,
) {
    let tapped_left  = keyboard.just_pressed(KeyCode::ArrowLeft)  || keyboard.just_pressed(KeyCode::KeyA);
    let tapped_right = keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD);
    let held_left    = keyboard.pressed(KeyCode::ArrowLeft)  || keyboard.pressed(KeyCode::KeyA);
    let held_right   = keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD);

    let tapped = tapped_left || tapped_right;
    let held   = (held_left || held_right) && !tapped;

    if tapped {
        let direction: f32 = match (tapped_left, tapped_right) {
            (true, false) => -1.0,
            (false, true) =>  1.0,
            _ => 0.0,
        };

        if direction != 0.0 {
            let now      = time.elapsed_secs() as f64;
            let interval = now - movement.last_tap_time;
            movement.last_tap_time = now;

            movement.step_size = if interval < TOO_FAST_SECS {
                (movement.step_size - STEP_PENALTY).max(MIN_STEP)
            } else if (SWEET_LO_SECS..=SWEET_HI_SECS).contains(&interval) {
                (movement.step_size + STEP_REWARD).min(MAX_STEP)
            } else if interval > RHYTHM_RESET_SECS {
                BASE_STEP
            } else {
                movement.step_size
            };

            movement.velocity = direction * movement.step_size * VELOCITY_SCALE;
        }
    }

    let per_frame_decay = if held { FRICTION_HELD } else { FRICTION_FREE };
    movement.velocity *= per_frame_decay.powf(time.delta_secs() * 60.0);

    let Ok(mut transform) = query.get_single_mut() else { return; };
    transform.translation.x += movement.velocity * time.delta_secs();

    let half = WINDOW_WIDTH / 2.0;
    transform.translation.x = transform
        .translation
        .x
        .clamp(-half + TURTLE_RADIUS, half - TURTLE_RADIUS);

    // Snap turtle's y to the road surface so it follows the curve.
    transform.translation.y = road_y_at(transform.translation.x);
}

fn check_collision(
    turtle_query: Query<&Transform, With<Turtle>>,
    tea_query: Query<&Transform, With<Tea>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let Ok(turtle_t) = turtle_query.get_single() else { return; };
    let Ok(tea_t) = tea_query.get_single() else { return; };

    let tp = turtle_t.translation.truncate();
    let sp = tea_t.translation.truncate();
    let half = TEA_SIZE / 2.0;

    let cx = tp.x.clamp(sp.x - half, sp.x + half);
    let cy = tp.y.clamp(sp.y - half, sp.y + half);
    let dx = tp.x - cx;
    let dy = tp.y - cy;

    if dx * dx + dy * dy <= TURTLE_RADIUS * TURTLE_RADIUS {
        next_state.set(AppState::Win);
    }
}