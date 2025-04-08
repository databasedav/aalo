use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ui::prelude::*;
use bevy_utils::prelude::*;
use haalka::prelude::*;
use std::sync::{Arc, Mutex};

pub mod defaults;
pub mod globals;
pub mod inspector;
pub mod reflect;
pub mod style;
pub mod utils;
pub mod widgets;

use inspector::*;

#[allow(clippy::type_complexity)]
struct WorldInspectorConfig {
    inspector_transformers:
        Mutex<Vec<Box<dyn FnOnce(Inspector) -> Inspector + Send + Sync + 'static>>>,
    unnest_children: bool,
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

#[derive(Default)]
pub struct AaloPlugin<WorldFlag> {
    world_inspector_config: Option<WorldInspectorConfig>,
    flags: std::marker::PhantomData<WorldFlag>,
}

impl AaloPlugin<WorldFlagNotSet> {
    pub fn new() -> Self {
        default()
    }
}

impl<WorldFlag> AaloPlugin<WorldFlag> {
    pub fn world(mut self) -> AaloPlugin<WorldFlagSet>
    where
        WorldFlag: FlagNotSet,
    {
        self.world_inspector_config = Some(WorldInspectorConfig {
            inspector_transformers: Mutex::new(Vec::new()),
            unnest_children: false,
        });
        self.into_type()
    }

    pub fn unnest_children(mut self) -> AaloPlugin<WorldFlagSet>
    where
        WorldFlag: FlagSet,
    {
        self.world_inspector_config
            .as_mut()
            .unwrap()
            .unnest_children = true;
        self.into_type()
    }

    pub fn with_inspector<F>(mut self, f: F) -> Self
    where
        F: FnOnce(Inspector) -> Inspector + Send + Sync + 'static,
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

impl<WorldFlag: Send + Sync + 'static> Plugin for AaloPlugin<WorldFlag> {
    #[allow(clippy::type_complexity)]
    fn build(&self, app: &mut App) {
        app.add_plugins(inspector::plugin);
        if let Some(world_inspector_config) = &self.world_inspector_config {
            let transformers: Mutex<Vec<Box<dyn FnOnce(Inspector) -> Inspector + Send + Sync>>> =
                Mutex::new(
                    world_inspector_config
                        .inspector_transformers
                        .lock()
                        .unwrap()
                        .drain(..)
                        .collect(),
                );
            let unnest_children = world_inspector_config.unnest_children;
            let transformers = Arc::new(transformers);
            app.add_systems(
                PostStartup,
                clone!((transformers) move |mut commands: Commands| {
                    commands.queue(clone!((transformers) move |world: &mut World| {
                        El::<Node>::new()
                            .global_z_index(GlobalZIndex(i32::MIN))
                            .width(Val::Percent(100.))
                            .height(Val::Percent(100.))
                            .cursor(CursorIcon::System(SystemCursorIcon::Default))
                            .child({
                                let mut inspector = Inspector::new();
                                if unnest_children {
                                    inspector =
                                        inspector.entities(ENTITIES.clone()).unnest_children();
                                } else {
                                    inspector = inspector.entities(ORPHAN_ENTITIES.clone());
                                }
                                inspector
                                    .resources(RESOURCES.clone())
                                    .assets(ASSETS.clone())
                                    .apply(|mut entity_inspector| {
                                        for f in transformers.lock().unwrap().drain(..) {
                                            entity_inspector = f(entity_inspector)
                                        }
                                        entity_inspector
                                    })
                                    .into_el()
                                    .with_node(|mut node| {
                                        node.position_type = PositionType::Absolute;
                                        node.top = Val::Px(20.);
                                        node.left = Val::Px(20.);
                                    })
                            })
                            .spawn(world);
                    }))
                }),
            );
        }
    }
}

pub mod prelude {
    pub use super::AaloPlugin;
    pub use crate::inspector::{register_frontend, FieldListener, Inspector, TargetField};
}
