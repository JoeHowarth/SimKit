use bevy::prelude::*;
use simkit_core::{KitCoreHeadlessPlugin, KitCorePlugin};

use crate::cli::parse_cli;
use stitchlands::{CliOptions, RunMode, StitchlandsCorePlugin};

pub fn camera_setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        bevy_pancam::PanCam {
            move_keys: bevy_pancam::DirectionKeys::wasd(),
            grab_buttons: vec![MouseButton::Right, MouseButton::Left],
            min_scale: 0.25,
            max_scale: 5.0,
            ..default()
        },
    ));
}

fn main() {
    let cli = parse_cli();

    let mut app = App::new();

    // Default plugins, with headless window config if requested
    if cli.mode == RunMode::Headless {
        app.add_plugins(MinimalPlugins);
    } else {
        app.add_plugins(DefaultPlugins);
    }

    app.insert_resource::<CliOptions>(cli.clone());
    if cli.mode == RunMode::Headless {
        app.add_plugins(KitCoreHeadlessPlugin);
    } else {
        app.add_plugins(KitCorePlugin);
    }
    app.add_plugins(StitchlandsCorePlugin);

    // Only spawn the camera in live mode
    if cli.mode == RunMode::Live {
        app.add_systems(Startup, camera_setup);
    } else {
        // Seed RNG directly in headless since state OnEnter hooks are not used
        use rand::SeedableRng;
        app.insert_resource::<CliOptions>(cli.clone());
        app.insert_resource(stitchlands::RngResource(
            rand::rngs::SmallRng::seed_from_u64(cli.seed),
        ));
    }

    app.run();
}

mod cli;
