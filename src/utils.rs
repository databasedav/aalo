use bevy::{ecs::system::SystemId, prelude::*};

// TODO: move to haalka ?
#[macro_export]
macro_rules! impl_syncers {
    { $($field:ident: $field_ty:ty),* $(,)? } => {
        paste::paste! {
            $(
                pub fn $field(self, $field: $field_ty) -> Self where Self: ElementWrapper {
                    self.[<$field _signal>](always($field))
                }

                pub fn [<$field _signal>](self, [<$field _signal>]: impl Signal<Item = $field_ty> + Send + 'static) -> Self where Self: ElementWrapper {
                    let syncer = spawn(sync([<$field _signal>], self.$field.clone()));
                    self.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]))
                }
            )*
        }
    };
}

#[derive(Component)]
pub struct AaloOneShotSystem;

pub fn register_system<I: 'static, O: 'static, M, S: IntoSystem<I, O, M> + 'static>(
    world: &mut World,
    system: S,
) -> SystemId<I, O> {
    let system = world.register_system(system);
    if let Some(mut entity) = world.get_entity_mut(system.entity()) {
        entity.insert(AaloOneShotSystem);
    }
    system
}
