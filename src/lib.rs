use bevy::{
    color::palettes::css::{LIME, MAROON},
    prelude::*,
};
use globals::{GLOBAL_BORDER_COLOR, GLOBAL_HIGHLIGHTED_COLOR, GLOBAL_UNHIGHLIGHTED_COLOR};
use haalka::prelude::*;
use inspector::ENTITIES;
use std::sync::Mutex;

pub mod defaults;
pub mod globals;
pub mod inspector;
pub mod reflect;
pub mod style;
pub mod utils;
pub mod widgets;

use inspector::*;
use style::*;

fn world_inspector(
    with_entity_inspector: Vec<
        Box<dyn FnMut(EntityInspector) -> EntityInspector + Send + Sync + 'static>,
    >,
) -> EntityInspector {
    EntityInspector::new()
        .apply(|mut entity_inspector| {
            for mut f in with_entity_inspector {
                entity_inspector = f(entity_inspector);
            }
            entity_inspector
        })
        .entities(ENTITIES.clone())
}

struct WorldInspectorConfig {
    inspector_transformers:
        Mutex<Vec<Box<dyn FnOnce(EntityInspector) -> EntityInspector + Send + Sync + 'static>>>,
}

// from MoonZoon https://github.com/MoonZoon/MoonZoon/blob/fc73b0d90bf39be72e70fdcab4f319ea5b8e6cfc/crates/zoon/src/lib.rs#L177-L193
pub trait FlagSet {}
pub trait FlagNotSet {}

#[macro_export]
macro_rules! make_flags {
    ($($flag:ident),*) => {
        $($crate::paste!{
            #[derive(Default)]
            pub struct [<$flag FlagSet>];
            #[derive(Default)]
            pub struct [<$flag FlagNotSet>];
            impl $crate::FlagSet for [<$flag FlagSet>] {}
            impl $crate::FlagNotSet for [<$flag FlagNotSet>] {}
        })*
    }
}

make_flags!(World);

pub struct AaloPlugin<WorldFlag> {
    world_inspector_config: Option<WorldInspectorConfig>,
    flags: std::marker::PhantomData<WorldFlag>,
}

impl AaloPlugin<WorldFlagNotSet> {
    pub fn new() -> Self {
        Self {
            world_inspector_config: None,
            flags: std::marker::PhantomData,
        }
    }
}

impl<WorldFlag> AaloPlugin<WorldFlag> {
    pub fn world(mut self) -> AaloPlugin<WorldFlagSet>
    where
        WorldFlag: FlagNotSet,
    {
        self.world_inspector_config = Some(WorldInspectorConfig {
            inspector_transformers: Mutex::new(Vec::new()),
        });
        self.into_type()
    }

    pub fn with_inspector<F>(mut self, f: F) -> Self
    where
        F: FnOnce(EntityInspector) -> EntityInspector + Send + Sync + 'static,
        WorldFlag: FlagSet,
    {
        self.world_inspector_config
            .as_mut()
            .unwrap()
            .inspector_transformers
            .lock()
            .unwrap()
            .push(Box::new(f));
        self
    }

    // TODO: picking powered web-like inspect element
    // pub fn right_click(self) -> Self {

    // }

    fn into_type<NewWorldFlag>(self) -> AaloPlugin<NewWorldFlag> {
        AaloPlugin {
            world_inspector_config: self.world_inspector_config,
            flags: std::marker::PhantomData,
        }
    }
}

#[derive(Resource)]
struct WorldInspectorTransformers {
    transformers: Vec<Box<dyn FnOnce(EntityInspector) -> EntityInspector + Send + Sync + 'static>>,
}

impl<WorldFlag: Send + Sync + 'static> Plugin for AaloPlugin<WorldFlag> {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<HaalkaPlugin>() {
            app.add_plugins(HaalkaPlugin);
        }
        app.add_plugins((inspector::plugin, style::plugin));
        // TODO: is there a better way to do this? couldn't just capture the transformers in a |&mut World| closure
        if let Some(ref world_inspector_config) = self.world_inspector_config {
            app.insert_resource(WorldInspectorTransformers {
                transformers: world_inspector_config
                    .inspector_transformers
                    .lock()
                    .unwrap()
                    .drain(..)
                    .collect(),
            });
            app.add_systems(Startup, move |world: &mut World| {
                El::<NodeBundle>::new()
                    .update_raw_el(|raw_el| {
                        raw_el.insert(Pickable {
                            should_block_lower: false,
                            ..default()
                        })
                    })
                    .width(Val::Percent(100.))
                    .height(Val::Percent(100.))
                    .cursor(CursorIcon::Default)
                    .child({
                        EntityInspector::new()
                            .entities(ENTITIES.clone())
                            .apply(|mut entity_inspector| {
                                if let Some(WorldInspectorTransformers { transformers }) =
                                    world.remove_resource::<WorldInspectorTransformers>()
                                {
                                    for mut f in transformers {
                                        entity_inspector = f(entity_inspector)
                                    }
                                }
                                entity_inspector
                            })
                            // .align(Align::new().top().left())
                            .into_el()
                            .height(Val::Px(400.))
                            .width(Val::Px(600.))
                            .with_style(|mut style| {
                                style.position_type = PositionType::Absolute;
                                // style.left = Val::Px(300.);
                                // style.top = Val::Px(300.);
                            })
                    })
                    .spawn(world);
            });
        }
    }
}
