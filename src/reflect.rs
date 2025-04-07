use std::any::TypeId;

use bevy_asset::{ReflectAsset, UntypedAssetId, UntypedHandle};
use bevy_ecs::{component::ComponentId, prelude::*};
use bevy_reflect::{prelude::*, ReflectFromPtr};

pub fn reflect_component(
    world: &mut World,
    entity: Entity,
    component: ComponentId,
) -> Option<&dyn Reflect> {
    if let Some(((entity, type_id), type_registry)) = world
        .get_entity(entity)
        .ok()
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
        if let Some((component_ptr, type_registration)) = entity
            .get_by_id(component)
            .ok()
            .zip(type_registry.get(type_id))
        {
            if let Some(reflect_from_ptr) = type_registration.data::<ReflectFromPtr>() {
                // SAFETY: same `ComponentId` is used to fetch component data and type id
                return Some(unsafe { reflect_from_ptr.as_reflect(component_ptr) });
            }
        }
    }
    None
}

pub fn reflect_component_mut<'w>(
    entity: &'w mut EntityWorldMut, // need an `EntityWorldMut` here because the mutable component pointer is tied to the lifetime of the `EntityWorldMut`
    component: ComponentId,
) -> Option<Mut<'w, dyn Reflect>> {
    if let Some((type_id, type_registry)) = entity
        .world_scope(|world| {
            world
                .components()
                .get_info(component)
                .and_then(|info| info.type_id())
        })
        .zip(entity.world_scope(|world| world.get_resource::<AppTypeRegistry>().cloned()))
    {
        if let Some((component_ptr, type_registration)) = entity
            .get_mut_by_id(component)
            .ok()
            .zip(type_registry.read().get(type_id))
        {
            if let Some(reflect_from_ptr) = type_registration.data::<ReflectFromPtr>() {
                return Some(component_ptr.map_unchanged(|ptr| {
                    // SAFETY: same `ComponentId` is used to fetch component data and type id
                    unsafe { reflect_from_ptr.as_reflect_mut(ptr) }
                }));
            }
        }
    }
    None
}

pub fn with_reflect_component<T>(
    world: &mut World,
    entity: Entity,
    component: ComponentId,
    f: impl FnOnce(&dyn Reflect) -> T,
) -> Option<T> {
    reflect_component(world, entity, component).map(f)
}

pub fn with_reflect_component_mut<T>(
    world: &mut World,
    entity: Entity,
    component: ComponentId,
    f: impl FnOnce(&mut dyn Reflect) -> T,
) -> Option<T> {
    world
        .get_entity_mut(entity)
        .ok()
        .and_then(|ref mut entity| {
            reflect_component_mut(entity, component).map(|mut reflect| f(reflect.as_reflect_mut()))
        })
}

pub fn reflect_resource(world: &mut World, component: ComponentId) -> Option<&dyn Reflect> {
    if let Some((type_id, type_registry)) = world
        .components()
        .get_info(component)
        .and_then(|info| info.type_id())
        .zip(
            world
                .get_resource::<AppTypeRegistry>()
                .map(|type_registry| type_registry.read()),
        )
    {
        if let Some((component_ptr, type_registration)) = world
            .get_resource_by_id(component)
            .zip(type_registry.get(type_id))
        {
            if let Some(reflect_from_ptr) = type_registration.data::<ReflectFromPtr>() {
                // SAFETY: same `ComponentId` is used to fetch component data and type id
                return Some(unsafe { reflect_from_ptr.as_reflect(component_ptr) });
            }
        }
    }
    None
}

pub fn reflect_resource_mut<'w>(
    world: &'w mut World,
    component: ComponentId,
) -> Option<Mut<'w, dyn Reflect>> {
    if let Some((type_id, type_registry)) = world
        .components()
        .get_info(component)
        .and_then(|info| info.type_id())
        .zip(world.get_resource::<AppTypeRegistry>().cloned())
    {
        if let Some((resource_ptr, type_registration)) = world
            .get_resource_mut_by_id(component)
            .zip(type_registry.read().get(type_id))
        {
            if let Some(reflect_from_ptr) = type_registration.data::<ReflectFromPtr>() {
                return Some(
                    resource_ptr
                        // SAFETY: same `ComponentId` is used to fetch component data and type id
                        .map_unchanged(|ptr| unsafe { reflect_from_ptr.as_reflect_mut(ptr) }),
                );
            }
        }
    };
    None
}

pub fn with_reflect_resource<T>(
    world: &mut World,
    component: ComponentId,
    f: impl FnOnce(&dyn Reflect) -> T,
) -> Option<T> {
    reflect_resource(world, component).map(f)
}

pub fn with_reflect_resource_mut<T>(
    world: &mut World,
    component: ComponentId,
    f: impl FnOnce(&mut dyn Reflect) -> T,
) -> Option<T> {
    reflect_resource_mut(world, component).map(|mut reflect| f(reflect.as_reflect_mut()))
}

pub fn reflect_asset(
    world: &mut World,
    asset: TypeId,
    handle: UntypedAssetId,
) -> Option<&dyn Reflect> {
    if let Some(type_registry) = world
        .get_resource::<AppTypeRegistry>()
        .map(|type_registry| type_registry.read())
    {
        if let Some(type_registration) = type_registry.get(asset) {
            if let Some(reflect_asset) = type_registration.data::<ReflectAsset>() {
                if let Some(reflect) = reflect_asset.get(world, UntypedHandle::Weak(handle)) {
                    return Some(reflect);
                }
            }
        }
    }
    None
}

pub fn reflect_asset_mut(
    world: &mut World,
    asset: TypeId,
    handle: UntypedAssetId,
) -> Option<&mut dyn Reflect> {
    if let Some(type_registry) = world.get_resource::<AppTypeRegistry>().cloned() {
        if let Some(registration) = type_registry.read().get(asset) {
            if let Some(reflect_asset) = registration.data::<ReflectAsset>() {
                return reflect_asset.get_mut(world, UntypedHandle::Weak(handle));
            }
        }
    }
    None
}

pub fn with_reflect_asset<T>(
    world: &mut World,
    asset: TypeId,
    handle: UntypedAssetId,
    f: impl FnOnce(&dyn Reflect) -> T,
) -> Option<T> {
    reflect_asset(world, asset, handle).map(f)
}

pub fn with_reflect_asset_mut<T>(
    world: &mut World,
    asset: TypeId,
    handle: UntypedAssetId,
    f: impl FnOnce(&mut dyn Reflect) -> T,
) -> Option<T> {
    reflect_asset_mut(world, asset, handle).map(f)
}
