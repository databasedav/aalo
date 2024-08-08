use bevy::prelude::*;
use haalka::prelude::*;
use inspector::ENTITIES;

pub mod inspector;
pub mod reflect;
pub mod style;
pub mod utils;

fn ui_root(world: &mut World) {
    El::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .child(
            inspector::EntityInspector::new()
                .entities(ENTITIES.clone())
                .align(Align::new().top().left()),
        )
        .spawn(world);
}

pub struct AaloPlugin;

impl Plugin for AaloPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<HaalkaPlugin>() {
            app.add_plugins(HaalkaPlugin);
        }
        app.add_plugins(inspector::plugin)
            .add_systems(Startup, ui_root);
    }
}
