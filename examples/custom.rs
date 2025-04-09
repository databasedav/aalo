//! register custom frontends for any type

mod utils;
use utils::*;

use aalo::prelude::*;
use bevy::ecs::{component::ComponentId, world::DeferredWorld};
use bevy::prelude::*;

fn main() {
    register_frontend("bool", custom_bool_frontend);
    register_frontend("custom::CustomBoolComponent", custom_bool_frontend);
    App::new()
        .add_plugins(DefaultPlugins.set(example_window_plugin()))
        .add_plugins(AaloPlugin::new().world().with_inspector(|inspector| {
            inspector.jump_to(("entity", "custom bool field", "boolcomponent", ".0"))
        }))
        .add_systems(Startup, setup)
        .register_type::<BoolComponent>()
        .register_type::<CustomBoolComponent>()
        .run();
}

#[derive(Component, Reflect, Default)]
struct BoolComponent(bool);

#[derive(Component, Reflect, Default)]
struct CustomBoolComponent(bool);

fn init_custom_bool_frontend(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
    let mut commands = world.commands();
    let text = commands.spawn_empty().id();
    let system = commands.register_system(
        move |In(reflect): In<Box<dyn PartialReflect>>, mut commands: Commands| {
            let cur_option = reflect.try_downcast_ref::<bool>().copied().or_else(|| {
                CustomBoolComponent::from_reflect(reflect.as_ref())
                    .map(|CustomBoolComponent(cur)| cur)
            });
            if let Some(cur) = cur_option {
                commands.entity(text).insert(Text(cur.to_string()));
            }
        },
    );
    commands
        .entity(entity)
        .add_child(text)
        .insert(FieldListener::new(system))
        .observe(
            move |click: Trigger<Pointer<Click>>, texts: Query<&Text>, mut field: TargetField| {
                if let Ok(Text(text)) = texts.get(text) {
                    let cur = match text.as_str() {
                        "true" => true,
                        "false" => false,
                        _ => return,
                    };
                    // one of these will silently error depending on if it's the field or component
                    // target, we just do both here for the convenience of using the same frontend
                    field.update(click.entity(), (!cur).clone_value());
                    field.update(click.entity(), CustomBoolComponent(!cur).clone_value());
                }
            },
        );
}

#[derive(Component)]
#[require(Node)]
#[component(on_add = init_custom_bool_frontend)]
struct CustomBoolFrontend;

fn custom_bool_frontend() -> impl Bundle {
    CustomBoolFrontend
}

fn setup(mut commands: Commands) {
    commands.spawn((Name::new("custom bool field"), BoolComponent::default()));
    commands.spawn((
        Name::new("custom bool component"),
        CustomBoolComponent::default(),
    ));
    commands.spawn(Camera2d);
}
