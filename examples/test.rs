use aalo::{
    inspector::{ComponentData, EntityData},
    widgets::Dropdown,
    AaloPlugin,
};
use bevy::prelude::*;
use haalka::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
            AaloPlugin::new().world().with_inspector(|inspector| {
                inspector
                    .with_entities(|entities| {
                        entities
                            .filter(|(_, EntityData { name, .. })| {
                                name.lock_ref().as_ref().map(AsRef::as_ref) == Some("ui root")
                            })
                            .map(|data| {
                                let (_, EntityData { expanded, .. }) = &data;
                                expanded.set(true);
                                data
                            })
                            .boxed()
                    })
                    .with_components(|components| {
                        components
                            .filter(|(_, ComponentData { name, .. })| {
                                // name == "TestEnum" ||
                                // name == "BoolComponent"// ||
                                name == "BoolComponentHolder"
                            })
                            .map(|data| {
                                let (_, ComponentData { expanded, .. }) = &data;
                                expanded.set(true);
                                data
                            })
                            .boxed()
                    })
            }),
        ))
        .register_type::<BoolComponent>()
        .register_type::<BoolComponentHolder>()
        .register_type::<TestEnum>()
        .add_systems(Startup, (camera, ui_root))
        .run();
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Clone, PartialEq, Component, Reflect, Default, EnumIter, Display)]
enum TestEnum {
    #[default]
    D,
    Y(bool, bool),
    B(f32),
    A(String),
    J {
        a: f32,
        b: String,
    },
    C(BoolComponent),
}

#[derive(Clone, PartialEq, Component, Reflect, Default)]
struct BoolComponent(bool, bool);

#[derive(Component, Reflect, Default)]
struct BoolComponentHolder {
    bool_1: BoolComponent,
    bool_2: BoolComponent,
    bool_3: Vec<bool>,
    bool_4: (bool, BoolComponent, Vec<bool>),
    enum_: TestEnum,
}

fn test_button() -> impl Element {
    El::<NodeBundle>::new()
        .width(Val::Px(50.))
        .height(Val::Px(50.))
        .cursor(CursorIcon::Pointer)
        .background_color(BackgroundColor(Color::WHITE))
        .on_click_with_system(|_: In<_>, mut holder: Query<&mut BoolComponentHolder>| {
            let mut holder = holder.single_mut();
            holder.bool_3.push(true);
        })
}

fn ui_root(world: &mut World) {
    El::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .cursor(CursorIcon::Default)
        .align_content(Align::center())
        .name("ui root")
        .update_raw_el(|raw_el| {
            raw_el
                .insert(BoolComponent::default())
                .insert(TestEnum::default())
                .insert(BoolComponentHolder {
                    bool_3: vec![true, false],
                    bool_4: (false, default(), vec![false, true]),
                    ..default()
                })
        })
        .child(
            Stack::<NodeBundle>::new()
                .align(Align::center())
                .width(Val::Px(100.))
                .height(Val::Px(100.))
                .name("stuff stack")
                .layer(Dropdown::new(
                    TestEnum::iter().map(Into::into).collect::<Vec<_>>().into(),
                ).default_option_handler()),
        )
        .spawn(world);
}
