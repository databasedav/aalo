use bevy::{ecs::component::ComponentId, prelude::*, reflect::ReflectFromPtr};

pub fn reflect(world: &mut World, entity: Entity, component: ComponentId) -> Option<&dyn Reflect> {
    if let Some(((entity, type_id), type_registry)) = world
        .get_entity(entity)
        .zip(
            world
                .components()
                .get_info(component)
                .and_then(|info| info.type_id()),
        )
        .zip(
            world
                .get_resource::<AppTypeRegistry>()
                .map(|type_registry| type_registry.read()),
        )
    {
        if let Some((component_ptr, type_registration)) =
            entity.get_by_id(component).zip(type_registry.get(type_id))
        {
            if let Some(reflect_from_ptr) = type_registration.data::<ReflectFromPtr>() {
                // SAFETY: same `ComponentId` is used to fetch component data and type id
                return Some(unsafe { reflect_from_ptr.as_reflect(component_ptr) });
            }
        }
    }
    None
}

pub fn reflect_mut<'w>(
    entity: &'w mut EntityWorldMut, // need an `EntityWorldMut` here because the mutable component pointer is tied to the lifetime of the `EntityWorldMut`
    component: ComponentId,
) -> Option<&'w mut dyn Reflect> {
    if let Some((type_id, type_registry)) = entity
        .world_scope(|world| {
            world
                .components()
                .get_info(component)
                .and_then(|info| info.type_id())
        })
        .zip(entity.world_scope(|world| {
            world
                .get_resource::<AppTypeRegistry>()
                .map(|type_registry| type_registry.clone())
        }))
    {
        if let Some((component_ptr, type_registration)) = entity
            .get_mut_by_id(component)
            .zip(type_registry.read().get(type_id))
        {
            if let Some(reflect_from_ptr) = type_registration.data::<ReflectFromPtr>() {
                // SAFETY: same `ComponentId` is used to fetch component data and type id
                return Some(unsafe {
                    reflect_from_ptr.as_reflect_mut(component_ptr.into_inner())
                });
            }
        }
    }
    None
}

pub fn with_reflect<T>(
    world: &mut World,
    entity: Entity,
    component: ComponentId,
    f: impl FnOnce(&dyn Reflect) -> T,
) -> Option<T> {
    reflect(world, entity, component).map(f)
}

pub fn with_reflect_mut<T>(
    world: &mut World,
    entity: Entity,
    component: ComponentId,
    f: impl FnOnce(&mut dyn Reflect) -> T,
) -> Option<T> {
    world
        .get_entity_mut(entity)
        .and_then(|ref mut entity| reflect_mut(entity, component).map(f))
}
