use std::collections::HashSet;

use aalo::{
    inspector::{self, EntityData},
    AaloPlugin,
};
use bevy::prelude::*;
use haalka::prelude::*;

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
            AaloPlugin,
        ))
        .register_type::<BoolComponent>()
        .register_type::<BoolComponentHolder>()
        .add_systems(Startup, (camera, ui_root))
        .run();
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component, Reflect, Default)]
struct BoolComponent(bool, bool);

#[derive(Component, Reflect, Default)]
struct BoolComponentHolder {
    bool_1: BoolComponent,
    bool_2: BoolComponent,
    bool_3: Vec<bool>,
    bool_4: (bool, BoolComponent, Vec<bool>),
}

fn ui_root(world: &mut World) {
    inspector::ENTITIES
        .entries_cloned()
        .for_each(|_| {
            let mut lock = inspector::ENTITIES.lock_mut();
            let mut remove = vec![];
            let ui_root_name = Some(Name::new("ui root"));
            for (&entity, inspector::EntityData { name, expanded }) in lock.iter() {
                if name == &ui_root_name {
                    expanded.set_neq(true);
                } else {
                    remove.push(entity);
                }
            }
            for ref entity in remove.into_iter().rev() {
                lock.remove(entity);
            }
            async {}
        })
        .apply(spawn)
        .detach();
    El::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .name("ui root")
        .update_raw_el(|raw_el| {
            raw_el
                .insert(BoolComponent::default())
                // .insert(BoolComponentHolder::default())
                .insert(BoolComponentHolder {
                    bool_3: vec![true, false],
                    bool_4: (false, default(), vec![false, true]),
                    ..default()
                })
        })
        .child(
            Stack::<NodeBundle>::new()
                .width(Val::Percent(100.))
                .height(Val::Percent(100.))
                .name("stuff stack"), // .layer(Checkbox::new(Mutable::new(false)).align(Align::center())),
        )
        .spawn(world);
}
