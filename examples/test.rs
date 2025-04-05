use aalo::{
    inspector::{EntityData, InspectionTargetRoot},
    AaloPlugin,
};
use bevy::prelude::*;
use bevy_math::Vec3A;
use haalka::prelude::*;
use strum::{Display, EnumIter};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    position: WindowPosition::At((3000, 360).into()),
                    // position: WindowPosition::Centered(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
            bevy::dev_tools::ui_debug_overlay::DebugUiPlugin,
            HaalkaPlugin,
            // style::plugin,
            AaloPlugin::new()
                .world()
                .unnest_children()
                .with_inspector(|inspector| {
                    inspector
                        // .header(Some("world inspector".to_string()))
                        // .jump_to(("FloatWrapper", "floatwrapper", ".0"))
                        .jump_to((
                            "entity",
                            "0v1",
                            "window",
                            ".internal.physical_cursor_position.0",
                        ))
                    // .jump_to(("entity", "0v1", "window", ".internal.drag_resize_request"))
                    // .jump_to(("resource", "ambientlight", ".color.0.alpha"))
                    // .jump_to(("asset", "textureatlaslayout", "0001", ".textures[5].max"))
                    // .jump_to(("column", "matrixholder", ".0"))
                    // .jump_to(("entity", "testenum", "testenum", ".0"))
                    // .jump_to(("ui root", "", ""))
                    // .jump_to("1v1")
                    //     .jump_to("camera_lmao")
                    // .jump_to(("0v1", "window", ".mode"))
                    // .jump_to(("FloatWrapper", "Name", ".name"))
                    // .jump_to(("0v1", "Window", ""))
                    // .jump_to((
                    //     "test",
                    //     "test::TestStruct",
                    //     ".ent",
                    // ))
                    // .jump_to(("BoolComponent", "test::BoolComponent", ".1"))
                    // .jump_to((
                    //     "0v1",
                    //     "bevy_window::window::Window",
                    //     ".resolution",
                    // ))
                    // .with_entities(|entities| {
                    //     entities
                    //         .filter_signal_cloned(|&(entity, _)| {
                    //             always(entity).map_future(|entity| async move {
                    //                 let result = Mutable::new(None);
                    //                 async_world().apply(clone!((result) move |world: &mut World| {
                    //                     result.set(Some(world.run_system_once(|| true).ok().unwrap_or(false)));
                    //                 })).await;
                    //                 result.signal_ref(Option::is_some).wait_for(true).await;
                    //                 result.get().unwrap_or(false)
                    //             })
                    //             .map(|result| result.unwrap_or(false))
                    //         })
                    //         .boxed()
                    // })
                    // .with_components(|components| {
                    //     components
                    //         .filter(|(_, ComponentData { name, .. })| {
                    //             name == "FloatWrapper" || name == "TestEnum"
                    //             // ||
                    //             // name == "BoolComponent"
                    //             // ||
                    //             // name == "BoolComponentHolder"
                    //         })
                    //         .map(|data| {
                    //             let (_, ComponentData { expanded, .. }) = &data;
                    //             expanded.set(true);
                    //             data
                    //         })
                    //         .boxed()
                    // })
                }),
        ))
        .register_type::<BoolComponent>()
        .register_type::<BoolComponentHolder>()
        .register_type::<TestEnum>()
        .register_type::<FloatWrapper>()
        .register_type::<TestStruct>()
        .register_type::<VecHolder>()
        .register_type::<MatrixHolder>()
        .register_type::<BoolVecHolder>()
        .register_type::<NonZeroHolder>()
        .add_systems(Startup, (camera, ui_root, setup))
        .add_systems(Update, toggle_overlay)
        .run();
}

fn camera(mut commands: Commands) {
    // commands.spawn((Camera2d, IsDefaultUiCamera));
}

#[derive(Clone, PartialEq, Component, Reflect, EnumIter, Display)]
enum TestEnum {
    D,
    Y(bool, bool),
    B(i8),
    A(String),
    J { a: f32, b: String },
    C(BoolComponent),
    T(u32),
}

impl Default for TestEnum {
    fn default() -> Self {
        Self::B(125)
    }
}

#[derive(Component, Reflect)]
struct FloatWrapper(f32);

#[derive(Clone, PartialEq, Component, Reflect, Default)]
#[reflect(Default)]
struct BoolComponent(bool, bool);

#[derive(Component, Reflect, Default)]
struct BoolComponentHolder {
    bool_1: BoolComponent,
    bool_2: BoolComponent,
    bool_3: Vec<bool>,
    bool_4: (bool, BoolComponent, Vec<bool>),
    enum_: TestEnum,
}

#[derive(Component, Reflect)]
struct TestStruct {
    ent: Entity,
}

// fn test_button() -> impl Element {
//     El::<Node>::new()
//         .width(Val::Px(50.))
//         .height(Val::Px(50.))
//         .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
//         .background_color(BackgroundColor(Color::WHITE))
//         .on_click_with_system(|_: In<_>, mut holder: Query<&mut BoolComponentHolder>| {
//             let mut holder = holder.single_mut();
//             holder.bool_3.push(true);
//         })
// }

#[derive(Component, Reflect)]
struct VecHolder(pub Vec3A);
#[derive(Component, Reflect)]
struct MatrixHolder(pub Mat4);
#[derive(Component, Reflect)]
struct BoolVecHolder(pub BVec2);
#[derive(Component, Reflect)]
struct NonZeroHolder(pub std::num::NonZeroI64);

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((FloatWrapper(f32::MAX - 100.), Name::new("FloatWrapper")));
    commands.spawn((BoolComponent::default(), Name::new("BoolComponent")));
    commands.spawn((TestEnum::default(), Name::new("TestEnum")));
    commands.spawn((
        BoolComponentHolder {
            bool_3: vec![true, false],
            bool_4: (false, default(), vec![false, true]),
            ..default()
        },
        Name::new("BoolComponentHolder"),
    ));

    // // plane
    // commands.spawn((
    //     Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
    //     MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    // ));
    // // cube
    // commands.spawn((
    //     Name::new("My Cube"),
    //     Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
    //     MeshMaterial3d(materials.add(Color::srgba(255. / 255., 181. / 255., 0., 102. / 255.))),
    //     Transform::from_xyz(0.0, 0.5, 0.0),
    // ));
    // // light
    // commands.spawn((
    //     PointLight {
    //         intensity: 2_000_000.0,
    //         shadows_enabled: true,
    //         ..default()
    //     },
    //     Transform::from_xyz(4.0, 8.0, 4.0),
    // ));
    // // camera
    commands.spawn((
        Camera2d::default(),
        Name::new("camera_lmao"),
        // Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn ui_root(world: &mut World) {
    Column::<Node>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .cursor(CursorIcon::System(SystemCursorIcon::Default))
        .update_raw_el(|raw_el| {
            raw_el
                .on_spawn(|world, entity| {
                    world.spawn((Name::new("test"), TestStruct { ent: entity }));
                })
                .insert(FloatWrapper(20.))
                .insert(BoolComponent::default())
                .insert(TestEnum::default())
                .insert(BoolComponentHolder {
                    bool_3: vec![true, false],
                    bool_4: (false, default(), vec![false, true]),
                    ..default()
                })
                .insert(PickingBehavior {
                    should_block_lower: false,
                    ..default()
                })
        })
        .item(
            Column::<Node>::new()
                .name("column")
                .update_raw_el(|raw_el| {
                    raw_el.insert((
                        VecHolder(Vec3A::ZERO),
                        MatrixHolder(Mat4::IDENTITY),
                        BoolVecHolder(BVec2::TRUE),
                        NonZeroHolder(std::num::NonZeroI64::new(1).unwrap()),
                    ))
                })
                .item(El::<Node>::new().name("test 0"))
                .item(El::<Node>::new().name("test 1"))
                .item(El::<Node>::new().name("test 2"))
                .item(El::<Node>::new().name("test 3")),
        )
        .spawn(world);
}

fn toggle_overlay(
    input: Res<ButtonInput<KeyCode>>,
    mut options: ResMut<bevy::dev_tools::ui_debug_overlay::UiDebugOptions>,
) {
    if input.just_pressed(KeyCode::F1) {
        options.toggle();
    }
}
