use bevy_ecs::{
    prelude::*,
    system::{RunSystemOnce, SystemId, SystemParam},
};
use bevy_hierarchy::prelude::*;
use bevy_math::prelude::*;
use bevy_picking::prelude::*;
use bevy_ui::prelude::*;
use haalka::prelude::*;

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

pub fn register_system<
    I: SystemInput + 'static,
    O: 'static,
    M,
    S: IntoSystem<I, O, M> + 'static,
>(
    world: &mut World,
    system: S,
) -> SystemId<I, O> {
    let system = world.register_system(system);
    if let Ok(mut entity) = world.get_entity_mut(system.entity()) {
        entity.insert(AaloOneShotSystem);
    }
    system
}

#[macro_export]
macro_rules! signal_or {
    ($signal:expr) => {
        $signal
    };
    ($first:expr, $($rest:expr),+) => {
        signal::or($first, signal_or!($($rest),+))
    };
}

#[macro_export]
macro_rules! signal_and {
    ($signal:expr) => {
        $signal
    };
    ($first:expr, $($rest:expr),+) => {
        signal::and($first, signal_and!($($rest),+))
    };
}

pub fn map_bool_signal<T: Copy + Send + Sync + 'static>(
    bool: impl Signal<Item = bool>,
    t: Mutable<T>,
    f: Mutable<T>,
) -> impl Signal<Item = T> {
    bool.map_bool_signal(move || t.signal(), move || f.signal())
}

#[allow(dead_code)]
pub fn map_bool_signal_cloned<T: Clone + Send + Sync + 'static>(
    bool: Mutable<bool>,
    t: Mutable<T>,
    f: Mutable<T>,
) -> impl Signal<Item = T> {
    bool.signal()
        .map_bool_signal(move || t.signal_cloned(), move || f.signal_cloned())
}

#[derive(Component)]
pub struct TooltipTargetPosition(pub Vec2);

pub fn sync_tooltip_position(
    // TODO: get rid of this (v cringe)
    expected_tooltip_height: f32,
) -> impl FnOnce(RawHaalkaEl) -> RawHaalkaEl {
    move |el| {
        el.observe(
            |event: Trigger<Pointer<Enter>>,
             mut inspector_ancestor: InspectorAncestor,
             mut commands: Commands| {
                if let Some(inspector) = inspector_ancestor.get(event.entity()) {
                    if let Some(mut entity) = commands.get_entity(inspector) {
                        entity.try_insert(TooltipTargetPosition(
                            event.event().pointer_location.position,
                        ));
                    }
                }
            },
        )
        .on_event_with_system::<Pointer<Move>, _>(
            move |In((entity, move_)): In<(Entity, Pointer<Move>)>,
                  mut move_tooltip_to_position: MoveTooltipToPosition,
                  mut inspector_ancestor: InspectorAncestor,
                  mut commands: Commands| {
                move_tooltip_to_position.move_(
                    entity,
                    move_.pointer_location.position,
                    Some(expected_tooltip_height),
                );
                if let Some(inspector) = inspector_ancestor.get(entity) {
                    if let Some(mut entity) = commands.get_entity(inspector) {
                        entity.try_insert(TooltipTargetPosition(move_.pointer_location.position));
                    }
                }
            },
        )
        .on_remove(|world, entity| {
            world.commands().queue(move |world: &mut World| {
                let _ = world.run_system_once(move |tooltips: Query<&TooltipHolder>| {
                    // needed to iterate through all of them since no components are available to target a specific inspector ? TODO
                    for TooltipHolder(tooltip) in tooltips.iter() {
                        let mut lock = tooltip.lock_mut();
                        if lock.as_ref().map(|tooltip| tooltip.owner) == Some(entity) {
                            *lock = None;
                        }
                    }
                });
            })
        })
    }
}

#[derive(Component)]
pub struct InspectorMarker;

#[derive(SystemParam)]
pub struct InspectorAncestor<'w, 's> {
    parents: Query<'w, 's, &'static Parent>,
    entity_inspectors: Query<'w, 's, &'static InspectorMarker>,
    cache: Local<'s, Option<Entity>>,
}

impl<'w, 's> InspectorAncestor<'w, 's> {
    pub fn get(&mut self, entity: Entity) -> Option<Entity> {
        if self.cache.is_none() {
            for ancestor in self.parents.iter_ancestors(entity) {
                if self.entity_inspectors.contains(ancestor) {
                    *self.cache = Some(ancestor);
                    break;
                }
            }
        }
        *self.cache
    }
}

#[derive(Component)]
pub struct Tooltip;

#[derive(SystemParam)]
pub struct MoveTooltipToPosition<'w, 's> {
    childrens: Query<'w, 's, &'static Children>,
    nodes: Query<'w, 's, &'static mut Node>,
    inspector_ancestor: InspectorAncestor<'w, 's>,
    tooltips: Query<'w, 's, &'static Tooltip>,
}

impl<'w, 's> MoveTooltipToPosition<'w, 's> {
    pub fn move_(&mut self, entity: Entity, position: Vec2, expected_tooltip_height: Option<f32>) {
        if let Some(inspector) = self.inspector_ancestor.get(entity) {
            let tooltip = 'block: {
                // TODO: make sure this doesn't spuriously check every descendant
                for descendant in self.childrens.iter_descendants(inspector) {
                    if self.tooltips.contains(descendant) {
                        break 'block descendant;
                    }
                }
                return;
            };
            if let Ok([inspector_node, mut tooltip_node]) =
                self.nodes.get_many_mut([inspector, tooltip])
            {
                let top = if let Val::Px(top) = inspector_node.top {
                    top
                } else {
                    0.
                };
                let left = if let Val::Px(left) = inspector_node.left {
                    left
                } else {
                    0.
                };
                // TODO: the computed node height is actually wrong sometimes ...
                // let modifier = computed_node.size().y.max(expected_tooltip_height.unwrap_or_default());
                tooltip_node.top =
                    Val::Px(position.y - top - expected_tooltip_height.unwrap_or_default());
                tooltip_node.left = Val::Px(position.x - left);
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct TooltipData {
    pub owner: Entity,
    pub text: String,
}

impl TooltipData {
    pub fn new(owner: Entity, text: String) -> Self {
        Self { owner, text }
    }
}

#[derive(Component, Clone)]
pub struct TooltipHolder(pub Mutable<Option<TooltipData>>);

#[derive(SystemParam)]
pub struct TooltipCache<'w, 's> {
    cache: Local<'s, Option<Mutable<Option<TooltipData>>>>,
    inspector_ancestor: InspectorAncestor<'w, 's>,
    tooltips: Query<'w, 's, &'static TooltipHolder>,
}

impl<'w, 's> TooltipCache<'w, 's> {
    pub fn get(&mut self, entity: Entity) -> Option<Mutable<Option<TooltipData>>> {
        if self.cache.is_none() {
            if let Some(inspector) = self.inspector_ancestor.get(entity) {
                if let Ok(TooltipHolder(tooltip)) = self.tooltips.get(inspector).cloned() {
                    *self.cache = Some(tooltip);
                }
            }
        }
        self.cache.clone()
    }
}
