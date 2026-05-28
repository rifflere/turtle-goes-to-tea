# Turtle Goes to Tea

A tiny Bevy game. Green screen. Press any key. You win.

## Prerequisites

### 1. Install Rust

Go to https://rustup.rs and follow the instructions for Windows.  
This installs `rustup` (the toolchain manager), `rustc` (the compiler), and `cargo` (the build tool/package manager).

After installing, open a **new** terminal and verify:

```
cargo --version
```

### 2. Install Visual Studio C++ Build Tools (Windows only)

Rust on Windows needs the MSVC linker. If you don't have Visual Studio installed:

1. Download the **Build Tools for Visual Studio** from Microsoft's website.
2. In the installer, select **"Desktop development with C++"**.
3. Restart your terminal after installing.

## How to Run

```
cd path\to\turtle-goes-to-tea
cargo run
```

> **First compile will take a while** — Bevy is a large crate.  
> Subsequent runs are much faster. The `Cargo.toml` already includes  
> optimization settings that cut dev compile times significantly.

## What You'll See

- A pleasant green window opens.
- "Press any key to continue..." appears centered on screen.
- Press any key → "You win!"

## Project Structure

```
turtle-goes-to-tea/
├── Cargo.toml       # Project metadata and dependencies
└── src/
    └── main.rs      # All the game code (heavily commented for learning)
```

## Key Bevy Concepts Used

| Concept | What it does |
|---|---|
| `App` | The entry point — you register plugins and systems here |
| `DefaultPlugins` | Bevy's batteries-included bundle (window, input, rendering, etc.) |
| `ClearColor` resource | Sets the window background color |
| `Startup` schedule | Systems here run exactly once at launch |
| `Update` schedule | Systems here run every frame |
| `Commands` | Used to spawn/despawn entities and components |
| `Camera2d` | A 2D camera; Bevy 0.15+ auto-adds required companion components |
| `Text2d` | Text rendered in 2D world space |
| `Component` trait | Marks a struct as something that can be attached to entities |
| `Query` | Lets a system read/write specific components on matching entities |
| `Res<T>` / `ResMut<T>` | Read/write access to a global resource |
| `ButtonInput<KeyCode>` | Tracks which keyboard keys are pressed/just-pressed/just-released |
| `Local<T>` | System-local state that persists between frames |
