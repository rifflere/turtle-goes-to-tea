use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Turtle Goes to Tea".into(),
                resolution: (800., 600.).into(),
                ..default()
            }),
            ..default()
        }))
        // This sets the background color of the window.
        .insert_resource(ClearColor(Color::srgb(0.22, 0.60, 0.22)))
        .add_systems(Startup, setup)
        .add_systems(Update, handle_input)
        .run();
}

// A marker component — we attach this to our text entity so we can
// find it later in the `handle_input` system.
#[derive(Component)]
struct GameText;

fn setup(mut commands: Commands) {
    // Spawn a 2D camera. In Bevy 0.15+, required components mean this
    // single component brings along everything the camera needs (Transform, etc.).
    commands.spawn(Camera2d);

    // Spawn text at the world origin (0, 0), which the camera looks at by default,
    // so the text appears centered on screen.
    commands.spawn((
        Text2d::new("Press any key to continue..."),
        TextFont {
            font_size: 40.0,
            ..default()
        },
        TextColor(Color::WHITE),
        GameText,
    ));
}

fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Text2d, With<GameText>>,
    // `Local<T>` is system-local state — like a field on the system function,
    // persisted between frames but private to this system.
    mut won: Local<bool>,
) {
    // Only react once — once the player has won, ignore further keypresses.
    if *won {
        return;
    }

    // `get_just_pressed()` returns keys pressed THIS frame (not held).
    // `.next().is_some()` checks if there's at least one.
    if keyboard_input.get_just_pressed().next().is_some() {
        *won = true;

        if let Ok(mut text) = query.get_single_mut() {
            *text = Text2d::new("You win!");
        }
    }
}
