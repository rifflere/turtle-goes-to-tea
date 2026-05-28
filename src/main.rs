use bevy::{prelude::*, sprite::ColorMaterial};

const WINDOW_WIDTH: f32 = 800.0;
const TURTLE_RADIUS: f32 = 25.0;
const TEA_SIZE: f32 = 40.0;
const TURTLE_SPEED: f32 = 350.0;

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
        // init_resource uses Default::default() to create the resource.
        .init_resource::<GameState>()
        .add_systems(Startup, setup)
        // .chain() ensures move_turtle always runs before check_collision,
        // so collision is tested against the position the turtle actually ends up at.
        .add_systems(Update, (move_turtle, check_collision).chain())
        .run();
}

#[derive(Resource, Default)]
struct GameState {
    won: bool,
}

// Marker components — zero-size types we attach to entities so systems can
// find them via Query. They carry no data; the type itself is the label.
#[derive(Component)]
struct Turtle;

#[derive(Component)]
struct Tea;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    // Turtle: a dark green circle, starting near the left edge, vertically centered.
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(TURTLE_RADIUS))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.1, 0.4, 0.1)))),
        Transform::from_xyz(-350.0, 0.0, 0.0),
        Turtle,
    ));

    // Tea: a brown square, sitting on the right side of the screen.
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(TEA_SIZE, TEA_SIZE))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.55, 0.35, 0.1)))),
        Transform::from_xyz(300.0, 0.0, 0.0),
        Tea,
    ));
}

fn move_turtle(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Turtle>>,
    time: Res<Time>,
    game_state: Res<GameState>,
) {
    if game_state.won {
        return;
    }

    let Ok(mut transform) = query.get_single_mut() else {
        return;
    };

    // Build a direction from held keys. If both are held simultaneously, they cancel.
    let mut direction = 0.0_f32;
    if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
        direction -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
        direction += 1.0;
    }

    // Multiply by delta_secs() so speed is consistent regardless of frame rate.
    transform.translation.x += direction * TURTLE_SPEED * time.delta_secs();

    // Keep the turtle fully on-screen by clamping its center so its edge never
    // crosses the window border.
    let half_width = WINDOW_WIDTH / 2.0;
    transform.translation.x = transform
        .translation
        .x
        .clamp(-half_width + TURTLE_RADIUS, half_width - TURTLE_RADIUS);
}

fn check_collision(
    turtle_query: Query<&Transform, With<Turtle>>,
    tea_query: Query<&Transform, With<Tea>>,
    mut game_state: ResMut<GameState>,
    mut commands: Commands,
) {
    if game_state.won {
        return;
    }

    let Ok(turtle_t) = turtle_query.get_single() else {
        return;
    };
    let Ok(tea_t) = tea_query.get_single() else {
        return;
    };

    let turtle_pos = turtle_t.translation.truncate(); // drop Z, work in 2D
    let tea_pos = tea_t.translation.truncate();
    let half = TEA_SIZE / 2.0;

    // Circle vs. axis-aligned rectangle collision:
    // Find the point on the rectangle closest to the circle's center,
    // then check if it's within the circle's radius.
    let closest_x = turtle_pos.x.clamp(tea_pos.x - half, tea_pos.x + half);
    let closest_y = turtle_pos.y.clamp(tea_pos.y - half, tea_pos.y + half);
    let dx = turtle_pos.x - closest_x;
    let dy = turtle_pos.y - closest_y;

    if dx * dx + dy * dy <= TURTLE_RADIUS * TURTLE_RADIUS {
        game_state.won = true;

        // Spawn the win text on top of everything (z = 1 renders in front of z = 0 shapes).
        commands.spawn((
            Text2d::new("Tea time!"),
            TextFont {
                font_size: 64.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(0.0, 0.0, 1.0),
        ));
    }
}
