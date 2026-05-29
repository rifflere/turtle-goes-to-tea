use bevy::{prelude::*, sprite::ColorMaterial};

// ── Gameplay constants ────────────────────────────────────────────────────────
const WINDOW_WIDTH: f32 = 800.0;
const TURTLE_RADIUS: f32 = 25.0;
const TEA_SIZE: f32 = 40.0;
const TURTLE_SPEED: f32 = 350.0;

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
) {
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
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Turtle>>,
    time: Res<Time>,
) {
    let Ok(mut transform) = query.get_single_mut() else {
        return;
    };

    let mut direction = 0.0_f32;
    if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
        direction -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
        direction += 1.0;
    }

    transform.translation.x += direction * TURTLE_SPEED * time.delta_secs();

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
