use aalo::{
    inspector::{ComponentData, EntityData},
    style::{border_color_style, border_width_style, BoxEdge},
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
            HaalkaPlugin,
            AaloPlugin::new().world()
            // .with_inspector(|inspector| {
            //     inspector
            //         .with_entities(|entities| {
            //             entities
            //                 .filter(|(_, EntityData { name, .. })| {
            //                     name.lock_ref().as_ref().map(AsRef::as_ref) == Some("ui root")
            //                 })
            //                 .map(|data| {
            //                     let (_, EntityData { expanded, .. }) = &data;
            //                     expanded.set(true);
            //                     data
            //                 })
            //                 .boxed()
            //         })
            //         .with_components(|components| {
            //             components
            //                 .filter(|(_, ComponentData { name, .. })| {
            //                     name == "TestEnum"
            //                     // ||
            //                     // name == "BoolComponent"
            //                     // ||
            //                     // name == "BoolComponentHolder"
            //                 })
            //                 .map(|data| {
            //                     let (_, ComponentData { expanded, .. }) = &data;
            //                     expanded.set(true);
            //                     data
            //                 })
            //                 .boxed()
            //         })
            // }),
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
        .child({
            let hovered = Mutable::new(false);
            Stack::<NodeBundle>::new()
                .align(Align::center())
                .width(Val::Px(100.))
                .height(Val::Px(100.))
                .name("stuff stack")
                // .layer(
                //     El::<NodeBundle>::new()
                //         .height(Val::Px(100.))
                //         .width(Val::Px(100.))
                //         .border_radius(BorderRadius::all(Val::Px(10.)))
                //         .apply(border_width_style(BoxEdge::ALL, always(5.)))
                //         .apply(border_color_style(always(
                //             bevy::color::palettes::basic::MAROON.into(),
                //         ))),
                // )
                // .layer(
                //     El::<NodeBundle>::new()
                //         .height(Val::Px(100.))
                //         .width(Val::Px(100.))
                //         .border_radius(BorderRadius::all(Val::Px(10.)))
                //         .apply(border_width_style(BoxEdge::HORIZONTAL, hovered.signal().map_false(|| 5.).map(Option::unwrap_or_default)))
                //         .apply(border_color_style(always(
                //             bevy::color::palettes::basic::LIME.into(),
                //         ))),
                // )
                // .layer(
                //     El::<NodeBundle>::new()
                //         .with_style(|mut style| style.left = Val::Px(2.))
                //         .hovered_sync(hovered)
                //         .cursor(CursorIcon::EResize)
                //         .height(Val::Px(100.))
                //         .width(Val::Px(9.))
                //         .align(Align::new().right())
                //         .border_radius(BorderRadius::all(Val::Px(10.)))
                //         .apply(border_width_style(BoxEdge::HORIZONTAL, always(5.)))
                //         .apply(border_color_style(always(Color::NONE)))
                // )
                .layer(
                    Dropdown::new(TestEnum::iter().map(Into::into).collect::<Vec<_>>().into())
                        .basic_option_handler(),
                )
        })
        .spawn(world);
}
