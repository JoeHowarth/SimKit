use bevy::{app::AppExit, prelude::*};

use crate::AppState;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Menu), setup_menu)
            .add_systems(OnExit(AppState::Menu), cleanup_menu)
            .add_systems(
                Update,
                (menu_button_system, menu_keyboard_input)
                    .run_if(in_state(AppState::Menu)),
            )
            .add_systems(
                Update,
                game_keyboard_input.run_if(in_state(AppState::InGame)),
            )
            .add_systems(Update, global_exit_handler);
    }
}

#[derive(Component)]
struct MenuUI;

#[derive(Component)]
struct PlayButton;

fn setup_menu(mut commands: Commands) {
    // Root UI container
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
            MenuUI,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("GAME MENU"),
                TextFont {
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Play button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.5, 0.2)),
                    PlayButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Play"),
                        TextFont {
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Instructions
            parent.spawn((
                Text::new("Press ENTER to play | Ctrl+C to exit"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        });
}

fn cleanup_menu(
    mut commands: Commands,
    menu_query: Query<Entity, With<MenuUI>>,
) {
    for entity in menu_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn menu_button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<PlayButton>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                next_state.set(AppState::InGame);
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.3, 0.6, 0.3));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.2, 0.5, 0.2));
            }
        }
    }
}

fn menu_keyboard_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Enter) {
        next_state.set(AppState::InGame);
    }
}

fn game_keyboard_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Menu);
    }
}

fn global_exit_handler(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    if (keyboard_input.pressed(KeyCode::ControlLeft)
        || keyboard_input.pressed(KeyCode::ControlRight))
        && keyboard_input.just_pressed(KeyCode::KeyC)
    {
        app_exit_events.write(AppExit::Success);
    }
}
