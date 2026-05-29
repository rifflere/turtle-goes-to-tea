use bevy::{prelude::*, sprite::ColorMaterial};

// ── Gameplay constants ────────────────────────────────────────────────────────
const WINDOW_WIDTH: f32 = 800.0;
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
// Bevy's States system lets us switch between distinct modes of the app.
// Systems can opt in to run only in a specific state.
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

// GameEntity marks everything spawned during gameplay so we can clean it all
// up in one sweep when leaving the Playing state.
#[derive(Component)] struct GameEntity;

// These mark the root UI node of each screen so cleanup is trivial.
#[derive(Component)] struct MenuRoot;
#[derive(Component)] struct WinRoot;

// Marks the animated title so the glow system can find it.
#[derive(Component)] struct TitleText;

// Attached to each navigable button. `index` is the button's position in the list.
#[derive(Component)]
struct MenuItem {
    index: usize,
}

// What happens when a button is activated.
#[derive(Component, Clone, Copy)]
enum ButtonAction {
    Play,
    PlayAgain,
    MainMenu,
    Exit,
}

// ── Resources ─────────────────────────────────────────────────────────────────
// Tracks which button is currently highlighted. Reset each time a new screen opens.
#[derive(Resource)]
struct MenuNav {
    index: usize, // which button is selected
    max: usize,   // total number of buttons on this screen
}

// Drives the rhythm-based movement system.
#[derive(Resource)]
struct TurtleMovement {
    velocity: f32,      // current velocity in pixels per second
    step_size: f32,     // pixels this tap will contribute (adjusts with rhythm)
    last_tap_time: f64, // elapsed_secs at the time of the previous tap
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

// Plain function run conditions are more portable than combinator chains.
fn on_menu_or_win(state: Res<State<AppState>>) -> bool {
    matches!(state.get(), AppState::Menu | AppState::Win)
}

// ── Entry point ───────────────────────────────────────────────────────────────
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Turtle Goes to Tea".into(),
                resolution: (WINDOW_WIDTH, 600.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.22, 0.60, 0.22)))
        .init_state::<AppState>()
        .init_resource::<TurtleMovement>()
        // Camera lives for the whole session — not tied to any state.
        .add_systems(Startup, spawn_camera)
        // OnEnter / OnExit run once when transitioning into or out of a state.
        .add_systems(OnEnter(AppState::Menu), spawn_menu)
        .add_systems(OnExit(AppState::Menu), cleanup::<MenuRoot>)
        .add_systems(OnEnter(AppState::Playing), spawn_game)
        .add_systems(OnExit(AppState::Playing), cleanup::<GameEntity>)
        .add_systems(OnEnter(AppState::Win), spawn_win_screen)
        .add_systems(OnExit(AppState::Win), cleanup::<WinRoot>)
        // Update systems scoped to states via run_if.
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

// Generic cleanup: despawns every entity that has component T, including children.
fn cleanup<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

// ── Menu screen ───────────────────────────────────────────────────────────────
fn spawn_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(MenuNav { index: 0, max: 2 });

    // Load the whimsical font from assets/fonts/. Falls back gracefully to the
    // built-in Bevy font if the file isn't present.
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
            // Title — animates between warm gold shades.
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

// Shared helper — spawns one porcelain button with a text label.
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

// Pulses the title through a range of warm gold shades to suggest a glow.
fn animate_title(time: Res<Time>, mut query: Query<&mut TextColor, With<TitleText>>) {
    for mut color in &mut query {
        let t = (time.elapsed_secs() * 1.4).sin() * 0.5 + 0.5; // oscillates 0 → 1
        color.0 = Color::srgb(0.98, 0.72 + t * 0.20, 0.08 + t * 0.14);
    }
}

// Highlights the selected button (brighter porcelain) and dims the rest.
fn update_button_visuals(nav: Res<MenuNav>, mut query: Query<(&MenuItem, &mut BackgroundColor)>) {
    for (item, mut bg) in &mut query {
        bg.0 = if item.index == nav.index {
            Color::srgb(0.98, 0.96, 0.91) // bright porcelain — selected
        } else {
            Color::srgb(0.84, 0.82, 0.77) // dim porcelain — idle
        };
    }
}

// Gently rotates and scales the selected button back and forth.
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
    // Changed<Interaction> means this only yields entities whose interaction
    // state changed this frame — avoids processing every button every frame.
    interaction_query: Query<(&Interaction, &MenuItem), Changed<Interaction>>,
    button_query: Query<(&MenuItem, &ButtonAction)>,
    mut next_state: ResMut<NextState<AppState>>,
    mut app_exit: EventWriter<AppExit>,
) {
    // Keyboard: up/down arrows or W/S move the highlight.
    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        nav.index = nav.index.saturating_sub(1);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        nav.index = (nav.index + 1).min(nav.max - 1);
    }

    // Mouse hover: whichever button the cursor is over becomes selected.
    for (interaction, item) in &interaction_query {
        if matches!(*interaction, Interaction::Hovered | Interaction::Pressed) {
            nav.index = item.index;
        }
    }

    // Activation: Enter or Space on keyboard, or a mouse click on the highlighted button.
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

// ── Gameplay ──────────────────────────────────────────────────────────────────
fn spawn_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut movement: ResMut<TurtleMovement>,
) {
    // Fresh state every time Play / Play Again is chosen.
    *movement = TurtleMovement::default();
    // GameEntity is added to every gameplay object so cleanup<GameEntity> removes them all.
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(TURTLE_RADIUS))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.1, 0.4, 0.1)))),
        Transform::from_xyz(-350.0, 0.0, 0.0),
        Turtle,
        GameEntity,
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(TEA_SIZE, TEA_SIZE))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.55, 0.35, 0.1)))),
        Transform::from_xyz(300.0, 0.0, 0.0),
        Tea,
        GameEntity,
    ));
}

fn move_turtle(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Turtle>>,
    time: Res<Time>,
    mut movement: ResMut<TurtleMovement>,
) {
    // Distinguish a fresh press from a held key.
    let tapped_left  = keyboard.just_pressed(KeyCode::ArrowLeft)  || keyboard.just_pressed(KeyCode::KeyA);
    let tapped_right = keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD);
    let held_left    = keyboard.pressed(KeyCode::ArrowLeft)  || keyboard.pressed(KeyCode::KeyA);
    let held_right   = keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD);

    let tapped = tapped_left || tapped_right;
    // "held" is true when a key is down but wasn't *just* pressed this frame.
    let held   = (held_left || held_right) && !tapped;

    if tapped {
        let direction: f32 = match (tapped_left, tapped_right) {
            (true, false) => -1.0,
            (false, true) =>  1.0,
            _ => 0.0, // both simultaneously → no movement
        };

        if direction != 0.0 {
            let now      = time.elapsed_secs() as f64;
            let interval = now - movement.last_tap_time;
            movement.last_tap_time = now;

            // Adjust step_size based on how rhythmic the tapping is.
            movement.step_size = if interval < TOO_FAST_SECS {
                // Mashing — penalise. Can bottom out at MIN_STEP.
                (movement.step_size - STEP_PENALTY).max(MIN_STEP)
            } else if (SWEET_LO_SECS..=SWEET_HI_SECS).contains(&interval) {
                // Sweet spot — reward. Capped at MAX_STEP.
                (movement.step_size + STEP_REWARD).min(MAX_STEP)
            } else if interval > RHYTHM_RESET_SECS {
                // Long pause → back to neutral, no bonus or penalty.
                BASE_STEP
            } else {
                // Between too-fast and sweet zone: neutral, no change.
                movement.step_size
            };

            // Set velocity so the natural glide covers roughly step_size pixels.
            // Derivation: total_distance = velocity / (60 * (1 - FRICTION_FREE)) = velocity / VELOCITY_SCALE
            // → velocity = step_size * VELOCITY_SCALE
            movement.velocity = direction * movement.step_size * VELOCITY_SCALE;
        }
    }

    // Friction: normalised to 60 fps so speed feels the same at any frame rate.
    let per_frame_decay = if held { FRICTION_HELD } else { FRICTION_FREE };
    movement.velocity *= per_frame_decay.powf(time.delta_secs() * 60.0);

    let Ok(mut transform) = query.get_single_mut() else { return; };
    transform.translation.x += movement.velocity * time.delta_secs();

    let half = WINDOW_WIDTH / 2.0;
    transform.translation.x = transform
        .translation
        .x
        .clamp(-half + TURTLE_RADIUS, half - TURTLE_RADIUS);
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
