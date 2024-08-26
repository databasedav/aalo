use std::i32;

use aalo::{
    globals::GLOBAL_PRIMARY_BACKGROUND_COLOR,
    inspector::{ComponentData, EntityData},
    style::{self, *},
    widgets::Dropdown,
    AaloPlugin,
};
use bevy::{
    color::palettes::css::{LIME, MAROON},
    prelude::*,
    ui::shader_flags::BORDER,
};
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
            HaalkaPlugin,
            // style::plugin,
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
                                name == "FloatWrapper" || name == "TestEnum"
                                // ||
                                // name == "BoolComponent"
                                // ||
                                // name == "BoolComponentHolder"
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
        .register_type::<FloatWrapper>()
        .add_systems(Startup, (camera, ui_root))
        .run();
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Clone, PartialEq, Component, Reflect, EnumIter, Display)]
enum TestEnum {
    D,
    Y(bool, bool),
    B(f32),
    A(String),
    J { a: f32, b: String },
    C(BoolComponent),
}

impl Default for TestEnum {
    fn default() -> Self {
        Self::B(20.)
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
    Column::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .cursor(CursorIcon::Default)
        // .align_content(Align::center())
        .name("ui root")
        .update_raw_el(|raw_el| {
            raw_el
                .insert(FloatWrapper(20.))
                .insert(BoolComponent::default())
                .insert(TestEnum::default())
                .insert(BoolComponentHolder {
                    bool_3: vec![true, false],
                    bool_4: (false, default(), vec![false, true]),
                    ..default()
                })
                .insert(Pickable {
                    should_block_lower: false,
                    ..default()
                })
        })
        // .item(
        //     Column::<NodeBundle>::new()
        //         .item(
        //             El::<TextBundle>::new()
        //             .text(Text::from_section("test", TextStyle { font_size: 30., ..default() }))
        //         )
        //         .item(
        //             El::<TextBundle>::new()
        //             .text(Text::from_section("test", TextStyle { font_size: 30., ..default() }))
        //         )
        //         .item(
        //             El::<TextBundle>::new()
        //             .text(Text::from_section("test", TextStyle { font_size: 30., ..default() }))
        //         )
        //         .item(
        //             El::<TextBundle>::new()
        //             .text(Text::from_section("test", TextStyle { font_size: 30., ..default() }))
        //         )
        //         .apply(padding_style(BoxEdge::ALL, always(10.)))
        //         .scrollable_on_hover(ScrollabilitySettings {
        //             flex_direction: FlexDirection::Column,
        //             overflow: Overflow::clip(),
        //             scroll_handler: BasicScrollHandler::new()
        //                 .direction(ScrollDirection::Vertical)
        //                 .pixels(20.)
        //                 .into(),
        //         })
        //         .into_raw().into_node_builder()
        //         .apply(RawHaalkaEl::from)
        //         .apply(El::<NodeBundle>::from)
        //         .apply(resize_border(
        //             always(5.),
        //             always(10.),
        //             always(MAROON.into()),
        //             always(LIME.into()),
        //             None,
        //         ))
        //         .apply(background_style(GLOBAL_PRIMARY_BACKGROUND_COLOR.signal()))
        //         .height(Val::Px(100.))
        //         .width(Val::Px(100.))
        //         .with_style(|mut style| {
        //             style.position_type = PositionType::Absolute;
        //             style.left = Val::Px(600.);
        //             style.top = Val::Px(300.);
        //         }),
        // )
        // .layer(
        //     Dropdown::new(TestEnum::iter().map(Into::into).collect::<Vec<_>>().into())
        //         .basic_option_handler(),
        // )
        .spawn(world);
}
