//! basic world inspector, inspired by <https://github.com/jakobhellermann/bevy-inspector-egui/blob/main/crates/bevy-inspector-egui/examples/quick/world_inspector.rs>

mod utils;
use utils::*;

use aalo::prelude::*;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(example_window_plugin()))
        .add_plugins(AaloPlugin::new().world().with_inspector(|inspector| {
            inspector.jump_to(("entity", "my cube", "transform", ".translation"))
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            toggle_visibility.run_if(
                any_with_component::<InspectorMarker>.and(resource_changed::<ButtonInput<KeyCode>>),
            ),
        )
        .run();
}

#[allow(clippy::eq_op)]
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    commands.spawn((
        Name::new("my cube"),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgba(255. / 255., 181. / 255., 0., 102. / 255.))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn toggle_visibility(
    inspector: Single<Entity, With<InspectorMarker>>,
    visibilities: Query<&Visibility>,
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if input.just_pressed(KeyCode::Backquote) {
        if let Ok(visibility) = visibilities.get(*inspector) {
            commands.entity(*inspector).insert(match visibility {
                Visibility::Hidden => Visibility::Visible,
                _ => Visibility::Hidden,
            });
        }
    }
}
