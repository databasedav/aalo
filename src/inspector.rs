use std::{
    any::TypeId,
    borrow::Cow,
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    convert::identity,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut, Not},
    str::FromStr,
    sync::{Arc, Mutex, OnceLock, RwLock},
};

use bevy_app::prelude::*;
use bevy_asset::{prelude::*, ReflectAsset, UntypedAssetId};
use bevy_color::{self, prelude::*};
use bevy_core::prelude::*;
use bevy_core_pipeline::prelude::*;
use bevy_cosmic_edit::{
    cosmic_text::{Action, Edit, Family, FamilyOwned, Motion, Selection},
    CosmicBackgroundColor, CosmicEditor, CosmicFontSystem, CosmicTextAlign, CosmicWrap,
    CursorColor, MaxLines, SelectionColor,
};
use bevy_derive::*;
use bevy_ecs::{
    archetype::Archetypes, component::*, entity::Entities, prelude::*, system::*,
    world::DeferredWorld,
};
use bevy_hierarchy::prelude::*;
use bevy_image::Image;
use bevy_input::{mouse::MouseWheel, prelude::*};
use bevy_log::prelude::*;
use bevy_math::prelude::*;
use bevy_picking::prelude::*;
use bevy_reflect::{prelude::*, *};
use bevy_render::{
    camera::{Camera, RenderTarget},
    prelude::*,
    render_resource::{AsBindGroup, ShaderRef},
    view::RenderLayers,
};
use bevy_rich_text3d::{GlyphMeta, Text3d, Text3dPlugin, Text3dStyling, TextAtlas};
use bevy_sprite::{prelude::*, AlphaMode2d, Material2d, Material2dPlugin};
use bevy_text::{cosmic_text::Weight, *};
use bevy_time::Time;
use bevy_transform::prelude::*;
use bevy_ui::prelude::*;
use bevy_utils::prelude::*;
use bevy_window::{PrimaryWindow, Window, WindowRef};
use disqualified::ShortName;
use haalka::{
    align::AlignabilityFacade,
    mouse_wheel_scrollable::{scroll_normalizer, ScrollDisabled},
    pointer_event_aware::UpdateHoverStatesDisabled,
    prelude::*,
    raw::{utils::remove_system_holder_on_remove, HaalkaObserver, HaalkaOneShotSystem},
    text_input::{FocusedTextInput, TextInputFocusOnDownDisabled},
    viewport_mutable::{LogicalRect, MutableViewport, Scene, Viewport},
};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher,
};
use num::Bounded;
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{defaults::*, globals::*, reflect::*, style::*, utils::*, widgets::*};
use crate::{impl_syncers, signal_or};

// TODO: implement frontend for at least all ui node types; how abt char, str, unit ? for unit, see (resources, Time, .context), should just be a tooltip
// TODO: dropdown z index is greater than headers so it appears above them when scrolling up
// TODO: counters for haalka and aalo systems with tooltips saying they can't be expanded because that would cause infinite recursion
// TODO: api for only showing certain object types, and only running particular syncers if these settings are such
// TODO: toggle inspector (actually spawn/despawn rather than just toggle visibility)
// TODO: docs
// TODO: states reflection
// TODO: show reflect documentation on hover (or right click ?) (see bevy-inspector-egui)
// TODO: use remote justfile from haalka + use new nickel package management to reuse all haalka nickels without copying them

// TODO: unnamed entities should probably just default to unsorted ?
// TODO: typing in the searching or targeting box should disable inputs somehow ? (status quo in bevy-inspector-egui is doing nothing)
// TODO: dragging numeric field doesn't work when number is very big ?
// TODO: text input growing is a bit too much / looks kinda cringe (y does text end align to center ??)
// TODO: parsed failed does not clear error on escape but does on enter
// TODO: dropdown max width should be max width of options ?
// TODO: (asset, image, 0005) 0005 is not in the typeregistry; actually pretty sure this is caused by desyncing, all such assets should be registered ?
// TODO: inconsistent left border hovering highlighting between reflect items and search/target items
// TODO: close other roots when auto scrolling roots to search/target (tried but failed)
// TODO: clipboard for copying names (especially asset handles)
// TODO: should type_path be mutable ?
// TODO: visible fields not syncing after uncollapsing (when globaltransform is changed)
// TODO: `Duration` reflection ?
// TODO: viewability (e.g.) should not be a mutable, since it is not dynamic, it should be a component, change all such things
// TODO: rapidly entering an expected tooltip area may not trigger its visibility (but only on debug builds ?), see (0v1, window, .mode)
// TODO: tooltip does not cover aalo text due to camera shenanigans
// TODO: consider limiting tooltips and the inspector to the area of the window
// TODO: runtime unnest children
// TODO: aalo text doesn't get clipped when the inspector width is less than its width
// TODO: looks like big numbers in numeric fields don't immediately resize correctly on spawn (flakey)
// TODO: non zero number impls
// TODO: numeric field text does not center align despite using CosmicTextAlign::Center (might be related to https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145)
// TODO: nested entity targeting
// TODO: text input font appears to be slightly smaller than normal text (bevy_cosmic_edit bug ?)
// TODO: ease scrollbar disappear and double click collapsing
// TODO: consolidate entity element and field element (a lot of stuff is the same)
// TODO: make text selectable
// TODO: inspect the inspector itself
// TODO: runtime font size (+/- hotkey) (need some dropdown fixes because of this too)
// TODO: separate font for code blocks
// TODO: inter inspector z conflicts
// TODO: open dropdowns z index does not respect header pinning
// TODO: https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145 prevents much of the expected text input styling to react as expected
// TODO: search/targeting input placeholders don't clip to the input (should be addressed by https://github.com/Dimchikkk/bevy_cosmic_edit/issues/171)
// TODO: pressing escape on errored text input doesn't clear the error (this only happens on debug build)
// TODO: dropdown scrollbars
// TODO: horizontal scrolling/scrollbar
// TODO: should the hash field update when modifying the name field ? doesn't happen in bevy-inspector-egui so
// TODO: touch scrolling
// TODO: string field text input centers cursor to center as text grows
// TODO: string field text input selection only flakily highlights entire text
// TODO: scroll snapping when scroll to exceeds the element height
// TODO: when input is focused, hovering it's field path has flakey cursor
// TODO: live editing parse failures don't surface until input is unfocused https://github.com/Dimchikkk/bevy_cosmic_edit/issues/145
// TODO: document how to make custom type views
// TODO: multiline text input
// TODO: popout windows
// TODO: asset based hot reloadable config
// TODO: optional limited components viewport within entity
// TODO: list modification abilities, add, remove, reorder
// TODO: tab and keyboard navigation
// TODO: inspector entities appear above resize borders, just wait for https://github.com/bevyengine/bevy/issues/14773
// TODO: dropdowns cannot extend past bounds of inspector

#[allow(clippy::type_complexity)]
#[derive(Clone, Default)]
pub struct EntityData {
    pub name: Mutable<Option<String>>,
    pub expanded: Mutable<bool>,
    pub filtered: Mutable<bool>,
    pub components: MutableBTreeMap<ComponentId, FieldData>,
    components_transformers:
        Arc<Mutex<Vec<Box<dyn FnMut(ComponentsSignalVec) -> ComponentsSignalVec + Send>>>>,
}

/// Entities without a `Parent`.
pub static ORPHAN_ENTITIES: Lazy<MutableBTreeMap<Entity, EntityData>> = Lazy::new(default);

/// Entities with a `Parent`.
pub static ENTITIES: Lazy<MutableBTreeMap<Entity, EntityData>> = Lazy::new(default);

pub type EntitySignalVec = std::pin::Pin<Box<dyn SignalVec<Item = (Entity, EntityData)> + Send>>;
pub type ComponentsSignalVec =
    std::pin::Pin<Box<dyn SignalVec<Item = (ComponentId, FieldData)> + Send>>;

#[allow(clippy::type_complexity)]
/// Configuration frontend for entity inspecting elements.
#[derive(Default)]
pub struct Inspector {
    el: Column<Node>,
    wrapper_stack: Stack<Node>,
    entities: MutableBTreeMap<Entity, EntityData>,
    entities_transformers:
        Arc<Mutex<Vec<Box<dyn FnMut(EntitySignalVec) -> EntitySignalVec + Send>>>>,
    components_transformers:
        Arc<Mutex<Vec<Box<dyn FnMut(ComponentsSignalVec) -> ComponentsSignalVec + Send>>>>,
    resources: MutableBTreeMap<ComponentId, FieldData>,
    assets: MutableBTreeMap<TypeId, AssetData>,
    search: Mutable<String>,
    first_target: Mutable<String>,
    second_target: Mutable<String>,
    third_target: Mutable<String>,
    // TODO: style stuff should be more packaged ? and maybe not here ?
    height: Mutable<f32>,
    width: Mutable<f32>,
    font_size: Mutable<f32>,
    row_gap: Mutable<f32>,
    column_gap: Mutable<f32>,
    padding: Mutable<f32>,
    border_radius: Mutable<f32>,
    border_width: Mutable<f32>,
    primary_background_color: Mutable<Color>,
    secondary_background_color: Mutable<Color>,
    tertiary_background_color: Mutable<Color>,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    border_color: Mutable<Color>,
    scroll_pixels: Mutable<f32>,
    header: Mutable<Option<String>>,
    flatten_descendants: bool,
}

#[derive(Component)]
pub struct InspectorColumn;

#[derive(Component)]
pub struct ScrollbarHeight(f32);

#[derive(Resource, Deref)]
pub struct SelectedInspector(Entity);

#[allow(clippy::too_many_arguments)]
fn search_input_shared_properties(
    hovered: Mutable<bool>,
    focused: Mutable<bool>,
    highlighted_color: Mutable<Color>,
    border_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    padding: Mutable<f32>,
    font_size: Mutable<f32>,
    text: Mutable<String>,
    tertiary_background_color: Mutable<Color>,
    placeholder: impl Signal<Item = &'static str> + Send + Sync + 'static,
) -> impl FnOnce(TextInput) -> El<Node> {
    move |input: TextInput| {
        El::<Node>::new()
            .child(
                input
                .apply(
                    border_color_style(
                        clone!((highlighted_color, unhighlighted_color, border_color) map_ref! {
                            let &hovered = hovered.signal(),
                            let &focused = focused.signal() => move {
                                if focused {
                                    highlighted_color.clone()
                                } else if hovered {
                                    unhighlighted_color.clone()
                                } else {
                                    border_color.clone()
                                }
                            }
                        })
                        .switch(|color| color.signal())
                    )
                )
                .update_raw_el(|raw_el| {
                    raw_el
                    .observe(|event: Trigger<OnInsert, CosmicEditor>, mut cosmic_editors: Query<&mut CosmicEditor>, mut font_system: ResMut<CosmicFontSystem>| {
                        if let Ok(mut cosmic_editor) = cosmic_editors.get_mut(event.entity()) {
                            let current_cursor = cosmic_editor.cursor();
                            if let Some(len) = cosmic_editor.with_buffer(|buffer| {
                                buffer.lines.first().map(|line| line.text().len())
                            }) {
                                if len > 0 {
                                    cosmic_editor.action(&mut font_system.0, Action::Motion(Motion::BufferEnd));
                                    cosmic_editor.set_selection(Selection::Line(
                                        bevy_cosmic_edit::cosmic_text::Cursor {
                                            line: 0,
                                            index: len - 1,
                                            affinity: current_cursor.affinity,
                                        },
                                    ));
                                }
                            }
                        }
                    })
                })
                .cursor(CursorIcon::System(SystemCursorIcon::Text))
                .attrs(
                    base_text_attrs()
                        .color_signal(
                            map_bool_signal(focused.signal(), highlighted_color, unhighlighted_color)
                            .map(Some)
                        ),
                )
                .max_lines(MaxLines(1))
                .scroll_disabled()
                // TODO: https://github.com/Dimchikkk/bevy_cosmic_edit/issues/171
                // .placeholder(Placeholder::new().text("search").attrs(TextAttrs::new().color_signal(tertiary_background_color.signal().map(Some))))
                .text_position_signal(padding.signal().map(|padding| CosmicTextAlign::Left { padding: padding.round() as i32 }))
            )
            .child(
                El::<Node>::new()
                .visibility_signal(text.signal_ref(String::is_empty).dedupe().map(|visible| if visible { Visibility::Visible } else { Visibility::Hidden }))
                // TODO: Stack would make more sense but it's being super annoying ...
                .with_node(|mut node| node.position_type = PositionType::Absolute)
                .apply(padding_style(BoxEdge::ALL, padding.signal()))
                .child(
                    El::<Text>::new()
                    .text_font(TextFont::from_font_size(font_size.get()))
                    .text_font_signal(font_size.signal().map(TextFont::from_font_size))
                    .text_color_signal(tertiary_background_color.signal().map(TextColor))
                    .text_signal(placeholder.map(Text::new))
                    .apply(text_no_wrap)
                )
            )
    }
}

pub fn inspector_column(
    entity: Entity,
    childrens: &Query<&Children>,
    inspector_columns: &Query<&InspectorColumn>,
) -> Option<Entity> {
    childrens
        .iter_descendants(entity)
        .find(|&child| inspector_columns.contains(child))
}

const DOUBLE_CLICK_TIMER: f32 = 0.5;

#[derive(Event, Clone)]
pub struct DoubleClick;

// TODO: replace with bevy picking native double click
pub fn trigger_double_click<Disabled: Component>(raw_el: RawHaalkaEl) -> RawHaalkaEl {
    raw_el.on_event_with_system_disableable::<Pointer<Click>, Disabled, _>(
        |In((entity, click)): In<(Entity, Pointer<Click>)>,
         time: Res<Time>,
         mut last_click_time_option: Local<Option<f32>>,
         mut commands: Commands| {
            if matches!(click.button, PointerButton::Primary) {
                let now = time.elapsed_secs();
                if let Some(last_click_time) = *last_click_time_option {
                    if now - last_click_time <= DOUBLE_CLICK_TIMER {
                        *last_click_time_option = None;
                        commands.trigger_targets(DoubleClick, entity);
                        return;
                    }
                }
                *last_click_time_option = Some(now);
            }
        },
    )
}

pub fn scroll_to_header_on_birth(el: RawHaalkaEl) -> RawHaalkaEl {
    el.observe(
        |event: Trigger<Born>,
         childrens: Query<&Children>,
         mut maybe_scroll_to_header_root: MaybeScrollToHeaderRoot,
         mut commands: Commands| {
            let entity = event.entity();
            // TODO: use relations to safely get the header element
            if let Some(&header_entity) = i_born(entity, &childrens, 0) {
                if maybe_scroll_to_header_root
                    .scrolled(header_entity, false)
                    .not()
                {
                    if let Some(mut entity) = commands.get_entity(entity) {
                        entity.remove::<WaitForBirth>();
                    }
                }
                commands.trigger(RemoveTarget { from: entity });
            }
        },
    )
}

#[derive(Component)]
pub struct WaitForSize(Option<Vec2>);

#[derive(Component)]
struct PreviousScrollPosition(f32);

fn children_insert_inspector_bloodline(children: &Children, commands: &mut Commands) {
    for &child in children.iter() {
        if let Some(mut child) = commands.get_entity(child) {
            child.try_insert(InspectorBloodline);
        }
    }
}

fn continue_bloodline(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
    world.commands().queue(move |world: &mut World| {
        let _ =
            world.run_system_once(move |childrens: Query<&Children>, mut commands: Commands| {
                if let Ok(children) = childrens.get(entity) {
                    children_insert_inspector_bloodline(children, &mut commands);
                }
            });
    });
}

/// So inspector entities can be filtered from the entity syncer, preventing infinite recursion.
#[derive(Component)]
#[component(on_add = continue_bloodline)]
pub struct InspectorBloodline;

fn propagate_inspector_bloodline(
    data: Query<&Children, (With<InspectorBloodline>, Changed<Children>)>,
    mut commands: Commands,
) {
    for children in data.iter() {
        children_insert_inspector_bloodline(children, &mut commands);
    }
}

#[derive(Component)]
pub struct Dragging;

fn despawn_aalo_text(world: &mut DeferredWorld, entity: Entity, _: ComponentId) {
    if let Some(&AaloText(entity)) = world.get::<AaloText>(entity) {
        world.commands().queue(move |world: &mut World| {
            world.try_despawn(entity);
        });
    }
}

fn maybe_despawn_aalo_text_camera(world: &mut DeferredWorld, entity: Entity, _: ComponentId) {
    if let Some(&AaloTextCamera) = world.get::<AaloTextCamera>(entity) {
        world.commands().queue(move |world: &mut World| {
            let _ = world.run_system_once(
                |aalo_texts: Query<&AaloText>,
                 aalo_text_camera_option: Option<Single<Entity, With<AaloTextCamera>>>,
                 mut commands: Commands| {
                    if aalo_texts.is_empty() {
                        if let Some(aalo_text_camera) = aalo_text_camera_option {
                            if let Some(entity) = commands.get_entity(*aalo_text_camera) {
                                entity.try_despawn_recursive();
                            }
                        }
                    }
                },
            );
        });
    }
}

// TODO: make issue to allow registering multiple component hooks more ergonomically ?
fn aalo_text_on_remove(mut world: DeferredWorld, entity: Entity, component: ComponentId) {
    despawn_aalo_text(&mut world, entity, component);
    maybe_despawn_aalo_text_camera(&mut world, entity, component);
}

fn maybe_spawn_aalo_text_camera(mut world: DeferredWorld, _: Entity, _: ComponentId) {
    world.commands().queue(move |world: &mut World| {
        let _ = world.run_system_once(
            |aalo_text_camera_option: Option<Single<Entity, With<AaloTextCamera>>>,
             mut commands: Commands| {
                if aalo_text_camera_option.is_none() {
                    commands.spawn(AaloTextCamera);
                }
            },
        );
    });
}

#[derive(Component)]
#[component(
    on_add = maybe_spawn_aalo_text_camera,
    on_remove = aalo_text_on_remove,
)]
struct AaloText(Entity);

#[allow(clippy::type_complexity)]
fn forward_aalo_text_visibility(
    data: Query<(Entity, &Visibility), (With<InspectorMarker>, Changed<Visibility>)>,
    childrens: Query<&Children>,
    aalo_texts: Query<&AaloText>,
    mut commands: Commands,
) {
    for (inspector, &visibility) in data.iter() {
        // TODO: use relations to identify this faster
        for descendant in childrens.iter_descendants(inspector) {
            if let Ok(&AaloText(entity)) = aalo_texts.get(descendant) {
                if let Some(mut entity) = commands.get_entity(entity) {
                    entity.insert(visibility);
                }
            }
        }
    }
}

// TODO: this isn't frame perfect, especially on low opt build
fn sync_aalo_text_position(
    data: Query<(&AaloText, &GlobalTransform), Changed<GlobalTransform>>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
    aalo_camera: Single<&Camera, With<AaloTextCamera>>,
    windows: Query<&Window>,
    mut transforms: Query<&mut Transform>,
) {
    for (&AaloText(entity), transform) in data.iter() {
        if let RenderTarget::Window(window) = aalo_camera.target {
            let window_entity = match window {
                WindowRef::Primary => *primary_window,
                WindowRef::Entity(entity) => entity,
            };
            if let Ok(window) = windows.get(window_entity) {
                if let Ok(mut text_transform) = transforms.get_mut(entity) {
                    let mut translation = transform.translation();
                    translation.y = -translation.y; // flip y axis
                    text_transform.translation = translation
                        - Vec3 {
                            x: window.width() / 2.,
                            y: -window.height() / 2.,
                            z: 0.,
                        };
                }
            }
        }
    }
}

#[derive(Component)]
pub struct WaitUntilNonZeroTransform;

#[allow(clippy::type_complexity)]
fn wait_until_non_zero_transform(
    data: Query<
        (Entity, &GlobalTransform),
        (With<WaitUntilNonZeroTransform>, Changed<GlobalTransform>),
    >,
    mut commands: Commands,
) {
    for (entity, global_transform) in data.iter() {
        if global_transform.translation() != Vec3::ZERO {
            if let Some(mut entity) = commands.get_entity(entity) {
                entity.remove::<WaitUntilNonZeroTransform>();
            }
        }
    }
}

fn iter_focused(focuseds: &[Mutable<bool>], tab: &Tab) {
    let len = focuseds.len();
    match tab {
        Tab::Up => {
            if let Some(i) = focuseds.iter().position(|focused| focused.get()) {
                focuseds[i].set_neq(false);
                if i == len - 1 {
                    focuseds[0].set_neq(true);
                } else {
                    focuseds[i + 1].set_neq(true);
                }
            } else {
                focuseds[0].set_neq(true);
            }
        }
        Tab::Down => {
            if let Some(i) = focuseds.iter().position(|focused| focused.get()) {
                focuseds[i].set_neq(false);
                if i == 0 {
                    focuseds[len - 1].set_neq(true);
                } else {
                    focuseds[i - 1].set_neq(true);
                }
            } else {
                focuseds[len - 1].set_neq(true);
            }
        }
    }
}

fn iter_target_root(
    target_root: &Mutable<InspectionTargetRoot>,
    target_root_move: &TargetRootMove,
) {
    let mut lock = target_root.lock_mut();
    match target_root_move {
        TargetRootMove::Right => {
            let next = match *lock {
                InspectionTargetRoot::Entity => InspectionTargetRoot::Resource,
                InspectionTargetRoot::Resource => InspectionTargetRoot::Asset,
                InspectionTargetRoot::Asset => InspectionTargetRoot::Entity,
            };
            *lock = next;
        }
        TargetRootMove::Left => {
            let next = match *lock {
                InspectionTargetRoot::Entity => InspectionTargetRoot::Asset,
                InspectionTargetRoot::Resource => InspectionTargetRoot::Entity,
                InspectionTargetRoot::Asset => InspectionTargetRoot::Resource,
            };
            *lock = next;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn inspection_target_root_selector(
    target_root: Mutable<InspectionTargetRoot>,
    target_root_focused: Mutable<bool>,
    column_gap: Mutable<f32>,
    border_radius: Mutable<f32>,
    border_color: Mutable<Color>,
    primary_background_color: Mutable<Color>,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    padding: Mutable<f32>,
    font_size: Mutable<f32>,
    border_width: Mutable<f32>,
    tertiary_background_color: Mutable<Color>,
) -> impl Element {
    let hovered = Mutable::new(false);
    Stack::<Node>::new()
    .hovered_sync(hovered.clone())
    .on_click(clone!((target_root_focused) move || target_root_focused.set_neq(true)))
    .on_click_outside(clone!((target_root_focused) move || target_root_focused.set_neq(false)))
    .layer(
        Row::<Node>::new()
        .with_node(|mut node| {
            node.justify_content = JustifyContent::SpaceBetween;
            node.align_items = AlignItems::Stretch;
        })
        .apply(row_style(column_gap.signal()))
        .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
        .apply(border_color_style(map_bool_signal(target_root_focused.signal(), unhighlighted_color.clone(), primary_background_color.clone())))
        .items_signal_vec(
            // TODO: this should depend the subset of object types that are desired
            MutableVec::from(InspectionTargetRoot::iter().collect::<Vec<_>>())
            .signal_vec()
            .map(clone!((padding, border_width, font_size, border_color, border_radius, highlighted_color, unhighlighted_color, border_color, tertiary_background_color, target_root) move |type_| {
                let hovered = Mutable::new(false);
                let focused = Mutable::new(false);
                let color = |base_color: Mutable<Color>| {
                    let selected = target_root.signal().eq(type_);
                    clone!((highlighted_color, unhighlighted_color, base_color) map_ref! {
                        let &hovered = hovered.signal(),
                        let &focused = focused.signal(),
                        let &selected = selected => {
                            if selected {
                                highlighted_color.signal()
                            } else if hovered || focused {
                                unhighlighted_color.signal()
                            } else {
                                base_color.signal()
                            }
                        }
                    })
                }
                .flatten();
                El::<Node>::new()
                .with_node(|mut node| node.flex_grow = 1.0)
                .hovered_sync(hovered.clone())
                .on_click(clone!((target_root) move || target_root.set(type_)))
                .height_signal(text_input_height_signal(font_size.signal(), border_width.signal(), padding.signal()).map(Val::Px))
                .apply(border_width_style(BoxEdge::ALL, border_width.signal()))
                .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
                .apply(border_color_style(color(border_color.clone())))
                .apply(padding_style(BoxEdge::HORIZONTAL, padding.signal()))
                .align_content(Align::center())
                .child(
                    El::<Text>::new()
                    .apply(text_style(font_size.signal(), color(tertiary_background_color.clone())))
                    .text(Text::from(match type_ {
                        InspectionTargetRoot::Entity => "entities",
                        InspectionTargetRoot::Resource => "resources",
                        InspectionTargetRoot::Asset => "assets",
                    }))
                )
                .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
            }))
        )
    )
    .layer(
        El::<Node>::new()
        // .align(Align::new().bottom())
        .visibility_signal(signal_or!(target_root_focused.signal(), hovered.signal()).dedupe().map(|show| if show { Visibility::Visible } else { Visibility::Hidden }))
        .on_signal_with_node(border_width.signal(), |mut node, border_width| node.top = Val::Px(border_width))
        .apply(border_color_style(unhighlighted_color.signal()))
        .apply(border_width_style([BoxEdge::Bottom], border_width.signal()))
        .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
    )
}

#[derive(Component)]
struct WaitForRootsCollapsed(HashSet<InspectionTargetRoot>);

#[derive(Component, Debug)]
struct RootCollapsed(InspectionTargetRoot);

impl Event for RootCollapsed {
    type Traversal = &'static Parent;

    const AUTO_PROPAGATE: bool = true;
}

#[derive(Event, Clone, Copy)]
pub struct ScrollToRoot(InspectionTargetRoot);

#[derive(Component)]
struct ResetHeaders(Vec<Entity>);

const PARSED_PATH_PLACEHOLDER: &str = "`ParsedPath` string e.g. \".bar#0.1[2].0\"";

fn listen_to_expanded_component(
    expanded: Mutable<bool>,
) -> impl FnOnce(RawHaalkaEl) -> RawHaalkaEl {
    move |raw_el: RawHaalkaEl| {
        raw_el
            .observe(
                clone!((expanded) move |_: Trigger<OnInsert, Expanded>| expanded.set_neq(true)),
            )
            .observe(move |_: Trigger<OnRemove, Expanded>| expanded.set_neq(false))
    }
}

fn make_matcher_and_atom(search: &str) -> (Matcher, Pattern) {
    let matcher = Matcher::new(Config::DEFAULT);
    let atom = Pattern::new(
        search,
        CaseMatching::Ignore,
        Normalization::Smart,
        nucleo_matcher::pattern::AtomKind::Fuzzy,
    );
    (matcher, atom)
}

fn atom_score(matcher: &mut Matcher, atom: &Pattern, name: &str) -> Option<u32> {
    atom.score(nucleo_matcher::Utf32String::from(name).slice(..), matcher)
}

fn make_target(
    root: InspectionTargetRoot,
    first_target: &str,
    second_target: &str,
    third_target: &str,
) -> Option<InspectionTarget> {
    if first_target.is_empty().not() {
        match root {
            InspectionTargetRoot::Entity | InspectionTargetRoot::Asset => {
                if second_target.is_empty().not() {
                    if third_target.is_empty().not() {
                        if ParsedPath::parse(third_target).is_err() {
                            None
                        } else {
                            Some(InspectionTarget::from((
                                root,
                                first_target,
                                second_target,
                                third_target,
                            )))
                        }
                    } else {
                        Some(InspectionTarget::from((root, first_target, second_target)))
                    }
                } else {
                    Some(InspectionTarget::from((root, first_target)))
                }
            }
            InspectionTargetRoot::Resource => {
                if second_target.is_empty().not() {
                    if ParsedPath::parse(second_target).is_err() {
                        None
                    } else {
                        Some(InspectionTarget::from((root, first_target, second_target)))
                    }
                } else {
                    Some(InspectionTarget::from((root, first_target)))
                }
            }
        }
    } else {
        Some(InspectionTarget::from(root))
    }
}

impl ElementWrapper for Inspector {
    type EL = Stack<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.wrapper_stack
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el,
            entities,
            entities_transformers,
            components_transformers,
            resources,
            assets,
            search,
            first_target,
            second_target,
            third_target,
            font_size,
            row_gap,
            column_gap,
            primary_background_color,
            secondary_background_color,
            tertiary_background_color,
            border_color,
            border_width,
            padding,
            border_radius,
            height,
            width,
            highlighted_color,
            unhighlighted_color,
            header,
            flatten_descendants,
            ..
        } = self;
        let flatten_descendants = Mutable::new(flatten_descendants);
        let viewport_height = Mutable::new(0.);
        let inspector_hovered = Mutable::new(false);
        let scrollbar_height_option: Mutable<Option<f32>> = Mutable::new(None);
        let show_search = Mutable::new(false);
        let search_focused = Mutable::new(false);
        let show_targeting = Mutable::new(false);
        let first_target_focused = Mutable::new(false);
        let second_target_focused = Mutable::new(false);
        let third_target_focused = Mutable::new(false);
        let collapsed = Mutable::new(false);
        let tooltip = Mutable::new(None);
        let search_target_root = Mutable::new(InspectionTargetRoot::Entity);
        let search_target_root_focused = Mutable::new(false);
        let targeting_target_root = Mutable::new(InspectionTargetRoot::Entity);
        let targeting_target_root_focused = Mutable::new(false);
        let search_task = {
            clone!((entities, resources, assets) map_ref! {
                let &show = show_search.signal(),
                let root = search_target_root.signal(),
                let search = search.signal_cloned() => {
                    let unfilter_entities = || {
                        for (_, EntityData { filtered, .. }) in entities.lock_ref().iter() {
                            filtered.set_neq(false);
                        }
                    };
                    let unfilter_resources = || {
                        for (_, FieldData { filtered, .. }) in resources.lock_ref().iter() {
                            filtered.set_neq(false);
                        }
                    };
                    let unfilter_assets = || {
                        for (_, AssetData { filtered, .. }) in assets.lock_ref().iter() {
                            filtered.set_neq(false);
                        }
                    };
                    if show {
                        match root {
                            InspectionTargetRoot::Entity => {
                                unfilter_resources();
                                unfilter_assets();
                                if search.is_empty() {
                                    unfilter_entities();
                                } else {
                                    let (mut matcher, atom) = make_matcher_and_atom(search);
                                    for (_, EntityData { name: name_option, filtered, .. }) in entities.lock_ref().iter() {
                                        filtered.set_neq(
                                            if let Some(name) = &*name_option.lock_ref() {
                                                atom_score(&mut matcher, &atom, name).is_none()
                                            } else {
                                                true
                                            }
                                        )
                                    }
                                }
                            },
                            InspectionTargetRoot::Resource => {
                                unfilter_assets();
                                unfilter_entities();
                                if search.is_empty() {
                                    unfilter_resources();
                                } else {
                                    let (mut matcher, atom) = make_matcher_and_atom(search);
                                    for (_, FieldData { name, filtered, .. }) in resources.lock_ref().iter() {
                                        filtered.set_neq(
                                            atom_score(&mut matcher, &atom, name).is_none()
                                        )
                                    }
                                }
                            },
                            InspectionTargetRoot::Asset => {
                                unfilter_entities();
                                unfilter_resources();
                                if search.is_empty() {
                                    unfilter_assets();
                                } else {
                                    let (mut matcher, atom) = make_matcher_and_atom(search);
                                    for (_, AssetData { name, filtered, .. }) in assets.lock_ref().iter() {
                                        filtered.set_neq(
                                            atom_score(&mut matcher, &atom, name).is_none()
                                        )
                                    }
                                }
                            },
                        }
                    } else {
                        unfilter_entities();
                        unfilter_resources();
                        unfilter_assets();
                    }
                }
            })
            .to_future()
            .apply(spawn)
        };
        let active_filterer = Mutable::new(None);
        let on_insert_search_filterer_task = {
            clone!((entities, resources, assets) map_ref! {
                let &show = show_search.signal(),
                let root = search_target_root.signal(),
                let search = search.signal_cloned() => {
                    if show && search.is_empty().not() {
                        let search = search.clone();
                        let task = match root {
                            InspectionTargetRoot::Entity => {
                                entities.signal_map_cloned().for_each(move |map_diff| {
                                    if let MapDiff::Insert { value: EntityData { name: name_option, filtered, .. }, .. } = map_diff {
                                        let (mut matcher, atom) = make_matcher_and_atom(&search);
                                        filtered.set_neq(
                                            if let Some(name) = &*name_option.lock_ref() {
                                                atom_score(&mut matcher, &atom, name).is_none()
                                            } else {
                                                true
                                            }
                                        )
                                    }
                                    async {}
                                })
                                .apply(spawn)
                            },
                            InspectionTargetRoot::Resource => {
                                resources.signal_map_cloned().for_each(move |map_diff| {
                                    if let MapDiff::Insert { value: FieldData { name, filtered, .. }, .. } = map_diff {
                                        let (mut matcher, atom) = make_matcher_and_atom(&search);
                                        filtered.set_neq(
                                            atom_score(&mut matcher, &atom, &name).is_none()
                                        )
                                    }
                                    async {}
                                })
                                .apply(spawn)
                            },
                            InspectionTargetRoot::Asset => {
                                assets.signal_map_cloned().for_each(move |map_diff| {
                                    if let MapDiff::Insert { value: AssetData { name, filtered, .. }, .. } = map_diff {
                                        let (mut matcher, atom) = make_matcher_and_atom(&search);
                                        filtered.set_neq(
                                            atom_score(&mut matcher, &atom, name).is_none()
                                        )
                                    }
                                    async {}
                                })
                                .apply(spawn)
                            },
                        };
                        active_filterer.set(Some(task));
                    } else {
                        active_filterer.take();
                    }
                }
            })
            .to_future()
            .apply(spawn)
        };
        let filtered_count = search_target_root.signal().switch(clone!((entities, resources, assets) move |root| match root {
            InspectionTargetRoot::Entity => {
                entities
                .entries_cloned()
                .filter_signal_cloned(|(_, EntityData { filtered, .. })| signal::not(filtered.signal()))
                .len()
                .apply(boxed_sync)
            },
            InspectionTargetRoot::Resource => {
                resources
                .entries_cloned()
                .filter_signal_cloned(|(_, FieldData { filtered, .. })| signal::not(filtered.signal()))
                .len()
                .apply(boxed_sync)
            },
            InspectionTargetRoot::Asset => {
                assets
                .entries_cloned()
                .filter_signal_cloned(|(_, AssetData { filtered, .. })| signal::not(filtered.signal()))
                .len()
                .apply(boxed_sync)
            },
        })).broadcast();
        el
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .hovered_sync(inspector_hovered.clone())
        .update_raw_el(clone!((show_search, show_targeting, first_target, second_target, third_target, search_target_root, targeting_target_root, search) move |raw_el| {
            raw_el
            .hold_tasks([search_task, on_insert_search_filterer_task])
            .on_signal_with_system(
                clone!((search, search_target_root, targeting_target_root) map_ref! {
                    let &show_search = show_search.signal(),
                    let &show_targeting = show_targeting.signal() => {
                        if show_search {
                            search.signal_ref(|_|()).switch(clone!((search_target_root) move |_| search_target_root.signal())).apply(boxed_sync).apply(Some)
                        } else if show_targeting {
                            targeting_target_root.signal().apply(boxed_sync).apply(Some)
                        } else {
                            None
                        }
                    }
                })
                .map(signal::option)
                .flatten(),
                |
                    In((entity, root_option)),
                    entities_headers: Query<&EntitiesHeader>,
                    resources_headers: Query<&ResourcesHeader>,
                    assets_headers: Query<&AssetsHeader>,
                    header_datas: Query<&HeaderData>,
                    childrens: Query<&Children>,
                    mut commands: Commands,
                | {
                    if let Some(root) = root_option {
                        let mut roots = vec![];
                        let mut reset_headers = vec![];
                        let mut collapse_and_push_root_if_expanded = |entity: Entity, header_datas: &Query<&HeaderData>, root: InspectionTargetRoot| {
                            if let Ok(HeaderData { expanded, .. }) = header_datas.get(entity) {
                                let mut lock = expanded.lock_mut();
                                if *lock {
                                    roots.push(root);
                                    *lock = false;
                                }
                            }
                        };
                        for descendant in childrens.iter_descendants(entity) {
                            match root {
                                InspectionTargetRoot::Entity => {
                                    if resources_headers.contains(descendant) {
                                        collapse_and_push_root_if_expanded(
                                            descendant,
                                            &header_datas,
                                            InspectionTargetRoot::Resource
                                        );
                                        reset_headers.push(descendant);
                                    }
                                    if assets_headers.contains(descendant) {
                                        collapse_and_push_root_if_expanded(
                                            descendant,
                                            &header_datas,
                                            InspectionTargetRoot::Asset
                                        );
                                        reset_headers.push(descendant);
                                    }
                                },
                                InspectionTargetRoot::Resource => {
                                    if assets_headers.contains(descendant) {
                                        collapse_and_push_root_if_expanded(
                                            descendant,
                                            &header_datas,
                                            InspectionTargetRoot::Asset
                                        );
                                        reset_headers.push(descendant);
                                    }
                                    if entities_headers.contains(descendant) {
                                        collapse_and_push_root_if_expanded(
                                            descendant,
                                            &header_datas,
                                            InspectionTargetRoot::Entity
                                        );
                                        reset_headers.push(descendant);
                                    }
                                },
                                InspectionTargetRoot::Asset => {
                                    if entities_headers.contains(descendant) {
                                        collapse_and_push_root_if_expanded(
                                            descendant,
                                            &header_datas,
                                            InspectionTargetRoot::Entity
                                        );
                                        reset_headers.push(descendant);
                                    }
                                    if resources_headers.contains(descendant) {
                                        collapse_and_push_root_if_expanded(
                                            descendant,
                                            &header_datas,
                                            InspectionTargetRoot::Resource
                                        );
                                        reset_headers.push(descendant);
                                    }
                                }
                            }
                        }
                        if let Some(mut entity) = commands.get_entity(entity) {
                            if roots.is_empty().not() {
                                entity.try_insert((
                                    WaitForRootsCollapsed(HashSet::from_iter(roots)),
                                    ScrollToRoot(root),
                                    ResetHeaders(reset_headers),
                                ));
                            } else {
                                entity.trigger(ScrollToRoot(root));
                            }
                        }
                    }
                }
            )
            .observe(|mut event: Trigger<RootCollapsed>, mut wait_for_roots_collapsed: Query<&mut WaitForRootsCollapsed>, mut commands: Commands| {
                event.propagate(false);
                let entity = event.entity();
                if let Ok(mut wait_for_roots_collapsed) = wait_for_roots_collapsed.get_mut(entity) {
                    wait_for_roots_collapsed.0.remove(&event.event().0);
                    if wait_for_roots_collapsed.0.is_empty() {
                        if let Some(mut entity) = commands.get_entity(entity) {
                            entity.remove::<WaitForRootsCollapsed>();
                        }
                    }
                }
            })
            .observe(|event: Trigger<OnRemove, WaitForRootsCollapsed>, scroll_to_roots: Query<&ScrollToRoot>, mut commands: Commands| {
                let entity = event.entity();
                if let Ok(scroll_to_root) = scroll_to_roots.get(entity).copied() {
                    if let Some(mut entity) = commands.get_entity(entity) {
                        entity.trigger(scroll_to_root);
                        entity.remove::<ScrollToRoot>();
                    }
                }
            })
            .observe(clone!((first_target, second_target, third_target) move |event: Trigger<ScrollToRoot>, reset_headers: Query<&ResetHeaders>, childrens: Query<&Children>, mut nodes: Query<&mut Node>, mut commands: Commands| {
                let entity = event.entity();
                if let Some(mut entity) = commands.get_entity(entity) {
                    let root = event.event().0;
                    if show_targeting.get() {
                        if let Some(target) = make_target(root, &first_target.lock_ref(), &second_target.lock_ref(), &third_target.lock_ref()) {
                            entity.try_insert(target);
                        }
                    } else if show_search.get() {
                        entity.try_insert(InspectionTarget::from(root));
                    }
                    if let Ok(reset_headers) = reset_headers.get(entity.id()) {
                        for &entity in &reset_headers.0 {
                            if let Some(&header) = i_born(entity, &childrens, 0) {
                                if let Ok(mut node) = nodes.get_mut(header) {
                                    node.top = Val::Px(0.);
                                }
                            }
                        }
                    }
                }
            }))
            .on_signal_with_entity(
                map_ref! {
                    let first_target = first_target.signal_cloned().dedupe_cloned(),
                    let second_target = second_target.signal_cloned().dedupe_cloned(),
                    let third_target = third_target.signal_cloned().dedupe_cloned() => {
                        if first_target.is_empty().not() {
                            make_target(targeting_target_root.get(), first_target, second_target, third_target)
                        } else {
                            // this taken care of by the root collapsing signal above, which is sensitive to viewport shifting as other roots are collapsed; doing this here would not be
                            // Some(InspectionTarget::from(root))
                            None
                        }
                    }
                },
                |mut entity, target_option| {
                    if let Some(target) = target_option {
                        entity.insert(target);
                    }
                }
            )
            .observe(|event: Trigger<OnInsert, InspectionTarget>, childrens: Query<&Children>, inspector_columns: Query<&InspectorColumn>, mut commands: Commands| {
                // TODO: use relations to identify inspector column
                if let Some(inspector_column) = inspector_column(event.entity(), &childrens, &inspector_columns) {
                    if let Ok(children) = childrens.get(inspector_column) {
                        // skip last child, which is a scrolling spacer
                        commands.trigger_targets(CheckInspectionTargets, children[..children.len() - 1].to_vec());
                    }
                }
            })
        }))
        // need this outside the column to allow mouse wheel scrolling when hovered above scrollbar track
        .on_hovered_change_with_system(|In((entity, hovered)), childrens: Query<&Children>, inspector_columns: Query<&InspectorColumn>, mut commands: Commands| {
            if let Some(inspector_column) = inspector_column(entity, &childrens, &inspector_columns) {
                if let Some(mut entity) = commands.get_entity(inspector_column) {
                    if hovered {
                        entity.remove::<ScrollDisabled>();
                    } else {
                        entity.try_insert(ScrollDisabled);
                    }
                }
            }
        })
        // header
        .item(
            El::<Node>::new()
            .update_raw_el(|raw_el| {
                raw_el
                .insert(PickingBehavior::default())
                .apply(manage_dragging_component)
                .on_event_with_system_stop_propagation::<Pointer<DragStart>, _>(|In((_, drag_start)): In<(Entity, Pointer<DragStart>)>, mut commands: Commands| {
                    if matches!(drag_start.button, PointerButton::Primary) {
                        commands.insert_resource(CursorOnHoverDisabled);
                        commands.insert_resource(UpdateHoverStatesDisabled);
                    }
                })
                .on_event_with_system_stop_propagation::<Pointer<DragEnd>, _>(|In((_, drag_end)): In<(Entity, Pointer<DragEnd>)>, mut commands: Commands| {
                    if matches!(drag_end.button, PointerButton::Primary) {
                        commands.remove_resource::<CursorOnHoverDisabled>();
                        commands.remove_resource::<UpdateHoverStatesDisabled>();
                    }
                })
                .on_event_with_system::<Pointer<Drag>, _>(|
                    In((entity, drag)): In<(Entity, Pointer<Drag>)>,
                    resize_parent_cache: ResizeParentCache,
                    mut nodes: Query<&mut Node>,
                | {
                    if matches!(drag.button, PointerButton::Primary) {
                        if let Some(resize_parent) = resize_parent_cache.get(entity) {
                            if let Ok(mut node) = nodes.get_mut(resize_parent) {
                                let cur = if let Val::Px(cur) = node.top { cur } else { 0. };
                                node.top = Val::Px(cur + drag.delta.y);
                                let cur = if let Val::Px(cur) = node.left { cur } else { 0. };
                                node.left = Val::Px(cur + drag.delta.x);
                            }
                        }
                    }
                })
                .with_entity(|mut entity| { entity.remove::<Dragging>(); })
                .apply(trigger_double_click::<Dragging>)
                .on_event_with_system::<DoubleClick, _>(clone!((collapsed, border_width) move |
                    In((entity, _)),
                    // inspector_ancestor: InspectorAncestor,
                    childrens: Query<&Children>,
                    computed_nodes: Query<&ComputedNode>,
                    inspector_columns: Query<&InspectorColumn>,
                    resize_parent_cache: ResizeParentCache,
                    scroll_positions: Query<&ScrollPosition>,
                    mut previous_size: Local<Option<(f32, f32)>>,
                    mut nodes: Query<&mut Node>,
                    mut commands: Commands,
                | {
                    if let Some(resize_parent) = resize_parent_cache.get(entity) {
                        if collapsed.get() {
                            if let Some((x, y)) = previous_size.take() {
                                if let Ok(mut node) = nodes.get_mut(resize_parent) {
                                    node.width = Val::Px(x);
                                    node.height = Val::Px(y);
                                    // TODO: replace with inspector ancestor once https://github.com/bevyengine/bevy/issues/14773
                                    if let Some(mut entity) = commands.get_entity(resize_parent) {
                                        entity.try_insert(WaitForSize(Some((x, y).into())));
                                    }
                                }
                            }
                        } else if let Some(&child) = i_born(entity, &childrens, 0) {
                            if let Some((child_computed_node, resize_parent_computed_node)) = computed_nodes.get(child).ok().zip(computed_nodes.get(resize_parent).ok()) {
                                let inspector_column_option = 'block: {
                                    for child in childrens.iter_descendants(resize_parent) {
                                        if inspector_columns.contains(child) {
                                            break 'block Some(child);
                                        }
                                    }
                                    None
                                };
                                if let Some(scroll_position) = inspector_column_option.and_then(|inspector_column| scroll_positions.get(inspector_column).ok()) {
                                    let Vec2 { x, y } = resize_parent_computed_node.size();
                                    // TODO: replace with inspector ancestor once https://github.com/bevyengine/bevy/issues/14773
                                    if let Some(mut entity) = commands.get_entity(resize_parent) {
                                        entity.try_insert(PreviousScrollPosition(scroll_position.offset_y));
                                    }
                                    *previous_size = Some((x, y));
                                    if let Ok(mut node) = nodes.get_mut(resize_parent) {
                                        let Vec2 { x, y } = child_computed_node.size();
                                        // TODO: get this from the root inspector itself after https://github.com/bevyengine/bevy/issues/14773
                                        let border_width = border_width.get() * 2.;
                                        node.width = Val::Px(x + border_width);
                                        node.height = Val::Px(y + border_width);
                                    }
                                }
                            }
                        }
                    }
                    flip(&collapsed);
                }))
            })
            .apply(background_style(primary_background_color.signal()))
            .child(
                El::<Node>::new()
                .align(Align::new().right())
                .apply(padding_style(BoxEdge::ALL, row_gap.signal()))
                .child_signal({
                    let font_size = font_size.signal().map(add(2.)).dedupe().broadcast();
                    header.signal_ref(Option::is_some).dedupe().map_bool(
                        clone!((font_size, unhighlighted_color) move || {
                            El::<Text>::new()
                            .text_color_signal(unhighlighted_color.signal().map(TextColor))
                            .apply(font_size_style(font_size.signal()))
                            .text_signal(header.signal_cloned().map(Option::unwrap_or_default).map(Text))
                        }),
                        clone!((font_size) move || {
                            El::<Text>::new()
                                .text_color(TextColor(Color::NONE))
                                .apply(font_size_style(font_size.signal()))
                                .text(Text::from("aalo"))
                                .update_raw_el(clone!((font_size) move |raw_el| {
                                    raw_el
                                    .insert(WaitUntilNonZeroTransform)
                                    .observe(clone!((font_size) move |event: Trigger<OnRemove, WaitUntilNonZeroTransform>, mut commands: Commands| {
                                        let entity = event.entity();
                                        commands.queue(clone!((font_size) move |world: &mut World| {
                                            let asset_id = Mutable::new(None);
                                            let el = {
                                                RawHaalkaEl::from((
                                                    Text3d::new("aalo"),
                                                    Text3dStyling {
                                                        font: "FiraMono".into(),
                                                        weight: Weight::MEDIUM,
                                                        size: DEFAULT_FONT_SIZE + 2.,
                                                        uv1: (GlyphMeta::RowX, GlyphMeta::ColY),
                                                        ..Default::default()
                                                    },
                                                    Mesh2d::default(),
                                                    AALO_TEXT_CAMERA_RENDER_LAYERS.clone(),
                                                    LightRays,
                                                ))
                                                .on_signal_with_component::<_, Text3dStyling>(font_size.signal(), |mut text_3d_styling, font_size| {
                                                    text_3d_styling.size = font_size;
                                                })
                                                .on_signal_with_system(
                                                    map_ref! {
                                                        let &asset_id = asset_id.signal(),
                                                        let &font_size = font_size.signal() => {
                                                            asset_id.map(|asset_id| (asset_id, font_size))
                                                        }
                                                    },
                                                    |In((_, asset_id_font_size_option)), mut materials: ResMut<Assets<LightRaysMaterial>>| {
                                                        if let Some((asset_id, font_size)) = asset_id_font_size_option {
                                                            if let Some(light_rays) = materials.get_mut(asset_id) {
                                                                light_rays.size.x = font_size;
                                                            }
                                                        }
                                                    }
                                                )
                                            };
                                            let text_entity = el.spawn(world);
                                            let mat = if let Some(mut materials) = world.get_resource_mut::<Assets<LightRaysMaterial>>() {
                                                materials.add(LightRaysMaterial::new(text_entity))
                                            } else { return };
                                            asset_id.set(Some(mat.id()));
                                            if let Ok(mut entity) = world.get_entity_mut(text_entity) {
                                                entity.insert(MeshMaterial2d(mat.clone()));
                                            }
                                            if let Ok(mut entity) = world.get_entity_mut(entity) {
                                                entity.insert(AaloText(text_entity));
                                            }
                                        }));
                                    }))
                                }))
                        })
                    )
                })
            )
        )
        .item(
            Stack::<Node>::new()
                .width(Val::Percent(100.))
                .height(Val::Percent(100.))
                // inspector column
                .layer(
                    Column::<Node>::new()
                    .update_raw_el(clone!((scrollbar_height_option) move |raw_el| {
                        raw_el
                            .insert(InspectorColumn)
                            .component_signal::<ScrollbarHeight, _>(scrollbar_height_option.signal().map_some(ScrollbarHeight))
                            .observe(on_scroll_header_pinner)
                            .observe(|event: Trigger<OnInsert, PinnedHeaders>, mut inspector_ancestor: InspectorAncestor, mut commands: Commands| {
                                let entity = event.entity();
                                if let Some(inspector) = inspector_ancestor.get(entity) {
                                    if let Some(mut entity) = commands.get_entity(inspector) {
                                        entity.try_insert(GlobalZIndex(z_order("inspector") - 100)); // TODO: this depends on the number of expanded headers, be more precise
                                    }
                                }
                            })
                            // TODO: on pinned headers remove should be handled
                    }))
                    .apply(border_radius_style(BoxCorner::TOP, border_radius.signal()))
                    .apply(border_color_style(border_color.signal()))
                    .mutable_viewport(haalka::prelude::Axis::Vertical)
                    .on_scroll_with_system_disableable::<ScrollDisabled, _>(
                        BasicScrollHandler::new()
                            .direction(ScrollDirection::Vertical)
                            .pixels_signal(self.scroll_pixels.signal().dedupe())
                            .into_system(),
                    )
                    .update_raw_el(|raw_el| raw_el.insert(ScrollDisabled))
                    .on_viewport_location_change_with_system(move |In((entity, (_, viewport))): In<(Entity, (Scene, Viewport))>, mut header_pinner: HeaderPinner| {
                        // TODO: is there a way to make this frame perfect ?
                        header_pinner.sync(entity, viewport.offset_y);
                    })
                    .on_viewport_location_change(clone!((viewport_height, scrollbar_height_option) move |scene, viewport| {
                        viewport_height.set_neq(viewport.height);
                        scrollbar_height_option.set_neq(
                            if viewport.height < scene.height {
                                Some(viewport.height / scene.height * viewport.height)
                            } else {
                                None
                            }
                        )
                    }))
                    .on_viewport_location_change_with_system(|
                        In((entity, (scene, viewport))): In<(Entity, (Scene, Viewport))>,
                        parents: Query<&Parent>,
                        childrens: Query<&Children>,
                        mut nodes: Query<&mut Node>,
                        scrollbar_heights: Query<&ScrollbarHeight>,
                    | {
                        // TODO: use relations
                        if let Ok(parent) = parents.get(entity) {
                            if let Some(&child) = i_born(parent.get(), &childrens, 1)  {
                                if let Some(&scrollbar_thumb) = i_born(child, &childrens, 0) {
                                    if let Some((mut node, &ScrollbarHeight(scrollbar_height))) = nodes.get_mut(scrollbar_thumb).ok().zip(scrollbar_heights.get(entity).ok()) {
                                        node.top = Val::Px((viewport.offset_y / scene.height * viewport.height).min(viewport.height - scrollbar_height));
                                    }
                                }
                            }
                        }
                    })
                    .item({
                        let hovered = Mutable::new(false);
                        let pinned = Mutable::new(false);
                        let expanded = Mutable::new(false);
                        object_type_header_with_count(
                            InspectionTargetRoot::Entity,
                            hovered.clone(),
                            font_size.clone(),
                            highlighted_color.clone(),
                            unhighlighted_color.clone(),
                            entities.entries_cloned().len(),
                            row_gap.clone(),
                            primary_background_color.clone(),
                            secondary_background_color.clone(),
                            padding.clone(),
                            pinned,
                            expanded.clone()
                        )
                        .update_raw_el(move |raw_el| {
                            raw_el
                            .insert(EntitiesHeader)
                            .component_signal::<SyncEntities, _>(flatten_descendants.signal().map_true(default))
                            .component_signal::<SyncOrphanEntities, _>(flatten_descendants.signal().map_false(default))
                        })
                        .item_signal(
                            expanded.signal().dedupe().map_true(clone!((padding, border_width, hovered, tertiary_background_color, border_color, font_size, primary_background_color, highlighted_color, unhighlighted_color, row_gap, secondary_background_color, column_gap) move || {
                                Column::<Node>::new()
                                .width(Val::Percent(100.))
                                .apply(move_style(Move_::Right, padding.signal()))
                                .apply(left_bordered_style(border_width.signal(), map_bool_signal(hovered.signal(), tertiary_background_color.clone(), border_color.clone()), padding.signal()))
                                .items_signal_vec({
                                    let mut signal_vec = entities.entries_cloned().boxed();
                                    signal_vec = signal_vec
                                        .map(clone!((components_transformers) move |mut data| {
                                            data.1.components_transformers = components_transformers.clone();
                                            data
                                        }))
                                        .boxed();
                                    for f in entities_transformers.lock().unwrap().iter_mut() {
                                        signal_vec = f(signal_vec);
                                    }
                                    signal_vec
                                        .filter_signal_cloned(|(_, EntityData { filtered, .. })| {
                                            signal::not(filtered.signal())
                                        })
                                        .map(clone!(
                                            (
                                                font_size,
                                                row_gap,
                                                column_gap,
                                                primary_background_color,
                                                secondary_background_color,
                                                border_color,
                                                border_width,
                                                padding,
                                                highlighted_color,
                                                unhighlighted_color
                                            ) move |(id, data)| {
                                            MultiFieldElement::new(MultiFieldData::Entity { id, data })
                                            .update_raw_el(|raw_el| {
                                                raw_el
                                                .on_spawn_with_system(|In(entity), childrens: Query<&Children>, mut commands: Commands| {
                                                    // TODO: use relations to safely fetch the header
                                                    if let Some(&child) = i_born(entity, &childrens, 0) {
                                                        if let Some(mut entity) = commands.get_entity(child) {
                                                            entity.try_insert(GlobalZIndex(z_order("header")));
                                                        }
                                                    }
                                                })
                                            })
                                            .show_name()
                                            .font_size_signal(font_size.signal())
                                            .row_gap_signal(row_gap.signal())
                                            .column_gap_signal(column_gap.signal())
                                            .primary_background_color_signal(primary_background_color.signal())
                                            .secondary_background_color_signal(secondary_background_color.signal())
                                            .border_color_signal(border_color.signal())
                                            .border_width_signal(border_width.signal())
                                            .padding_signal(padding.signal())
                                            .highlighted_color_signal(highlighted_color.signal())
                                            .unhighlighted_color_signal(unhighlighted_color.signal())
                                            .into_el()
                                            .width(Val::Percent(100.))
                                        }))
                                })
                            }))
                        )
                    })
                    .item({
                        let hovered = Mutable::new(false);
                        let pinned = Mutable::new(false);
                        let expanded = Mutable::new(false);
                        object_type_header_with_count(
                            InspectionTargetRoot::Resource,
                            hovered.clone(),
                            font_size.clone(),
                            highlighted_color.clone(),
                            unhighlighted_color.clone(),
                            resources.entries_cloned().len(),
                            row_gap.clone(),
                            primary_background_color.clone(),
                            secondary_background_color.clone(),
                            padding.clone(),
                            pinned,
                            expanded.clone()
                        )
                        .update_raw_el(|raw_el| {
                            raw_el
                            .insert(SyncResources)
                            .insert(ResourcesHeader)
                        })
                        .item_signal(
                            expanded.signal().dedupe().map_true(clone!((padding, border_width, hovered, tertiary_background_color, border_color, highlighted_color, unhighlighted_color, row_gap, column_gap) move || {
                                Column::<Node>::new()
                                .width(Val::Percent(100.))
                                .apply(move_style(Move_::Right, padding.signal()))
                                .apply(left_bordered_style(border_width.signal(), map_bool_signal(hovered.signal(), tertiary_background_color.clone(), border_color.clone()), padding.signal()))
                                .items_signal_vec({
                                    resources.entries_cloned()
                                    .filter_signal_cloned(|(_, FieldData { filtered, .. })| signal::not(filtered.signal()))
                                    .sort_by_cloned(|(_, FieldData { name: left_name, .. }), (_, FieldData { name: right_name, .. })| type_path_ord(left_name, right_name))
                                    .map(clone!((row_gap, column_gap, tertiary_background_color, border_width, border_color, padding, highlighted_color, unhighlighted_color) move |(component, FieldData { name, expanded, viewability, .. })| {
                                        FieldElement::new(FieldElementInput::Component { owner: ComponentOwnerType::Resource, component }, FieldType::Field(name), viewability)
                                        .row_gap_signal(row_gap.signal())
                                        .column_gap_signal(column_gap.signal())
                                        .type_path_color_signal(tertiary_background_color.signal())
                                        .border_width_signal(border_width.signal())
                                        .border_color_signal(border_color.signal())
                                        .padding_signal(padding.signal())
                                        .highlighted_color_signal(highlighted_color.signal())
                                        .unhighlighted_color_signal(unhighlighted_color.signal())
                                        .expanded_signal(expanded.signal().dedupe())
                                    }))
                                })
                            }))
                        )
                    })
                    .item({
                        let hovered = Mutable::new(false);
                        let pinned = Mutable::new(false);
                        let expanded = Mutable::new(false);
                        object_type_header_with_count(
                            InspectionTargetRoot::Asset,
                            hovered.clone(),
                            font_size.clone(),
                            highlighted_color.clone(),
                            unhighlighted_color.clone(),
                            assets.entries_cloned().len(),
                            row_gap.clone(),
                            primary_background_color.clone(),
                            secondary_background_color.clone(),
                            padding.clone(),
                            pinned,
                            expanded.clone()
                        )
                        .update_raw_el(|raw_el| {
                            raw_el
                            .insert(SyncAssets)
                            .insert(AssetsHeader)
                        })
                        .item_signal(
                            expanded.signal().dedupe().map_true(clone!((padding, border_width, hovered, tertiary_background_color, border_color, highlighted_color, unhighlighted_color, row_gap, font_size, primary_background_color, column_gap, padding) move || {
                                Column::<Node>::new()
                                .width(Val::Percent(100.))
                                .apply(move_style(Move_::Right, padding.signal()))
                                .apply(left_bordered_style(border_width.signal(), map_bool_signal(hovered.signal(), tertiary_background_color.clone(), border_color.clone()), padding.signal()))
                                .items_signal_vec({
                                    assets.entries_cloned()
                                    .filter_signal_cloned(|(_, AssetData { filtered, .. })| signal::not(filtered.signal()))
                                    .sort_by_cloned(|(_, AssetData { name: left_name, .. }), (_, AssetData { name: right_name, .. })| type_path_ord(left_name, right_name))
                                    .map(clone!(
                                        (
                                            font_size,
                                            row_gap,
                                            column_gap,
                                            primary_background_color,
                                            secondary_background_color,
                                            border_color,
                                            border_width,
                                            padding,
                                            highlighted_color,
                                            unhighlighted_color
                                        ) move |(id, data)| {
                                        MultiFieldElement::new(MultiFieldData::Asset { id, data })
                                        .update_raw_el(|raw_el| {
                                            raw_el
                                            .on_spawn_with_system(|In(entity), childrens: Query<&Children>, mut commands: Commands| {
                                                // TODO: use relations to safely fetch the header
                                                if let Some(&child) = i_born(entity, &childrens, 0) {
                                                    if let Some(mut entity) = commands.get_entity(child) {
                                                        entity.try_insert(GlobalZIndex(z_order("header")));
                                                    }
                                                }
                                            })
                                        })
                                        .show_name()
                                        .font_size_signal(font_size.signal())
                                        .row_gap_signal(row_gap.signal())
                                        .column_gap_signal(column_gap.signal())
                                        .primary_background_color_signal(primary_background_color.signal())
                                        .secondary_background_color_signal(secondary_background_color.signal())
                                        .border_color_signal(border_color.signal())
                                        .border_width_signal(border_width.signal())
                                        .padding_signal(padding.signal())
                                        .highlighted_color_signal(highlighted_color.signal())
                                        .unhighlighted_color_signal(unhighlighted_color.signal())
                                        .into_el()
                                        .width(Val::Percent(100.))
                                    }))
                                })
                            }))
                        )
                    })
                    .item(
                        // TODO: doesn't work without a wrapper for some reason
                        El::<Node>::new()
                        .child(
                            El::<Node>::new()
                            .height_signal(viewport_height.signal().map(Val::Px))
                        )
                    )
                )
                // scrollbar
                .layer_signal(
                    scrollbar_height_option.signal_ref(Option::is_some).dedupe()
                    .map_true(clone!((tertiary_background_color, scrollbar_height_option, inspector_hovered) move || {
                        let track_hovered = Mutable::new(false);
                        let thumb_hovered = Mutable::new(false);
                        let dragging = Mutable::new(false);
                        let width = signal_or!(track_hovered.signal(), dragging.signal()).map_bool(|| SCROLLBAR_WIDTH_BIG, || SCROLLBAR_WIDTH_SMOL).broadcast();
                        // TODO: while the entities are hovered, listen to move events, if there is no movement after x seconds or it's no longer hovered, start a system that fades out the scrollbar
                        // by increasing the alpha with a quarticin easing function, if the entities are hovered again or there is some movement, instantly bring the color back
                        // TODO: smooth scrolling
                        // track
                        El::<Node>::new()
                        .with_node(|mut node| {
                            node.padding.right = Val::Px(SCROLLBAR_PADDING);
                            node.width = Val::Px(SCROLLBAR_WIDTH_BIG + SCROLLBAR_DEADZONE + SCROLLBAR_PADDING);
                        })
                        .height(Val::Percent(100.))
                        .hovered_sync(track_hovered.clone())
                        .global_z_index(GlobalZIndex(z_order("scrollbar")))
                        .align(Align::new().right())
                        .cursor(CursorIcon::System(SystemCursorIcon::Default))
                        .update_raw_el(|raw_el| {
                            raw_el
                            .on_event_with_system_stop_propagation::<Pointer<Down>, _>(|
                                In((entity, down)): In<(_, Pointer<Down>)>,
                                mut inspector_ancestor: InspectorAncestor,
                                childrens: Query<&Children>,
                                inspector_columns: Query<&InspectorColumn>,
                                mut scroll_positions: Query<&mut ScrollPosition>,
                                scrollbar_heights: Query<&ScrollbarHeight>,
                                mutable_viewports: Query<&MutableViewport>,
                                logical_rect: LogicalRect,
                                mut header_pinner: HeaderPinner,
                            | {
                                if matches!(down.button, PointerButton::Primary) {
                                    // TODO: replace with relations
                                    if let Some(inspector) = inspector_ancestor.get(entity) {
                                        if let Some(inspector_column) = inspector_column(inspector, &childrens, &inspector_columns) {
                                            if let Some((((mut scroll_position, &ScrollbarHeight(scrollbar_height)), MutableViewport { scene, viewport }), logical_rect)) = scroll_positions.get_mut(inspector_column).ok().zip(scrollbar_heights.get(inspector_column).ok()).zip(mutable_viewports.get(inspector_column).ok()).zip(logical_rect.get(inspector_column)) {
                                                let thumb_min_y = viewport.offset_y / scene.height * viewport.height;
                                                let down_y = down.pointer_location.position.y - logical_rect.min.y;
                                                // TODO: this seems to be a bit off, the top of the bar seems correct, but the bottom is not including the border radius
                                                if down_y < thumb_min_y || down_y > thumb_min_y + scrollbar_height {
                                                    let new = ((down_y - scrollbar_height / 2.) * scene.height / viewport.height).max(0.).min(scene.height - viewport.height);
                                                    header_pinner.sync(inspector_column, new);
                                                    scroll_position.offset_y = new
                                                }
                                            }
                                        }
                                    }
                                }
                            })
                            .on_event_with_system_stop_propagation::<Pointer<Down>, _>(clone!((dragging) move |
                                In((_, down)): In<(_, Pointer<Down>)>,
                                mut commands: Commands
                            | {
                                if matches!(down.button, PointerButton::Primary) {
                                    dragging.set_neq(true);
                                    commands.insert_resource(CursorOnHoverDisabled);
                                    commands.insert_resource(UpdateHoverStatesDisabled);
                                }
                            }))
                            .on_event_with_system_stop_propagation::<Pointer<Up>, _>(clone!((dragging) move |In((_, up)): In<(_, Pointer<Up>)>, mut commands: Commands| {
                                if matches!(up.button, PointerButton::Primary) {
                                    dragging.set_neq(false);
                                    commands.remove_resource::<CursorOnHoverDisabled>();
                                    commands.remove_resource::<UpdateHoverStatesDisabled>();
                                }
                            }))
                            .on_event_with_system_stop_propagation::<Pointer<DragEnd>, _>(clone!((dragging) move |In((_, drag_end)): In<(_, Pointer<DragEnd>)>, mut commands: Commands| {
                                if matches!(drag_end.button, PointerButton::Primary) {
                                    dragging.set_neq(false);
                                    commands.remove_resource::<CursorOnHoverDisabled>();
                                    commands.remove_resource::<UpdateHoverStatesDisabled>();
                                }
                            }))
                            .on_event_with_system_stop_propagation::<Pointer<Drag>, _>(|
                                In((entity, drag)): In<(Entity, Pointer<Drag>)>,
                                parents: Query<&Parent>,
                                childrens: Query<&Children>,
                                mut scroll_positions: Query<&mut ScrollPosition>,
                                mutable_viewports: Query<&MutableViewport>,
                                mut header_pinner: HeaderPinner,
                            | {
                                if matches!(drag.button, PointerButton::Primary) {
                                    // TODO: replace with relations ?
                                    if let Ok(parent) = parents.get(entity) {
                                        if let Some(&inspector_column) = i_born(parent.get(), &childrens, 0) {
                                            if let Some((mut scroll_position, MutableViewport { scene, viewport })) = scroll_positions.get_mut(inspector_column).ok().zip(mutable_viewports.get(inspector_column).ok()) {
                                                let new = (scroll_position.offset_y + drag.delta.y * scene.height / viewport.height).max(0.);
                                                header_pinner.sync(inspector_column, new);
                                                scroll_position.offset_y = new;
                                            };
                                        }
                                    }
                                }
                            })
                        })
                        .child(
                            // thumb
                            El::<Node>::new()
                            .hovered_sync(thumb_hovered.clone())
                            .align(Align::new().right())
                            .border_radius(BorderRadius::MAX)
                            .height_signal(scrollbar_height_option.signal().map_some(Val::Px))
                            .width_signal(width.signal().map(Val::Px))
                            .background_color_signal(
                                map_ref! {
                                    let &base_color = tertiary_background_color.signal(),
                                    let &inspector_hovered = inspector_hovered.signal(),
                                    let &track_hovered = track_hovered.signal(),
                                    let &thumb_hovered = thumb_hovered.signal(),
                                    let &dragging = dragging.signal() => move {
                                        if inspector_hovered || track_hovered {
                                            base_color.lighter(
                                                if dragging {
                                                    0.4
                                                } else if thumb_hovered {
                                                    0.2
                                                } else if track_hovered {
                                                    0.1
                                                } else {
                                                    0.
                                                }
                                            )
                                        } else {
                                            Color::NONE
                                        }
                                    }
                                }
                                .map(BackgroundColor)
                            )
                        )
                    }))
                )
                // search
                .layer_signal(
                    signal::and(show_search.signal(), signal::not(collapsed.signal()))
                    .map_true(clone!((
                        highlighted_color,
                        border_color,
                        primary_background_color,
                        tertiary_background_color,
                        filtered_count,
                        padding,
                        unhighlighted_color,
                        font_size,
                        border_radius,
                        border_width,
                        border_color,
                        search,
                        search_focused,
                        column_gap,
                        search_target_root,
                        search_target_root_focused
                    ) move || {
                        let hovered = Mutable::new(false);
                        Column::<Node>::new()
                        .apply(padding_style(BoxEdge::ALL, padding.signal()))
                        .global_z_index(GlobalZIndex(z_order("target/search")))
                        .apply(background_style(primary_background_color.signal()))
                        .align(Align::new().bottom())
                        .apply(border_radius_style(BoxCorner::TOP, border_radius.signal()))
                        .apply(border_width_style([BoxEdge::Top], border_width.signal()))
                        .apply(border_color_style(border_color.signal()))
                        .item(
                            El::<Node>::new()
                            .apply(left_bordered_style(border_width.signal(), map_bool_signal(signal_or!(hovered.signal(), search_focused.signal()).dedupe(), tertiary_background_color.clone(), border_color.clone()), padding.signal()))
                            .apply(padding_style([BoxEdge::Left], padding.signal()))
                            .child(
                                base_text_input(search.clone(), identity, hovered.clone(), search_focused.clone(), None)
                                .update_raw_el(|raw_el| raw_el.on_spawn(clone!((search_focused) move |_, _| search_focused.set(true))))
                                .on_change_sync(search.clone())
                                .apply(
                                    search_input_shared_properties(
                                        hovered.clone(),
                                        search_focused.clone(),
                                        highlighted_color.clone(),
                                        border_color.clone(),
                                        unhighlighted_color.clone(),
                                        padding.clone(),
                                        font_size.clone(),
                                        search.clone(),
                                        tertiary_background_color.clone(),
                                        always("search"),
                                    )
                                )
                                .child_signal(search.signal_ref(String::is_empty).dedupe().apply(signal::not).map_true(clone!((padding, font_size, unhighlighted_color, filtered_count) move || {
                                    El::<Node>::new()
                                    // TODO: Stack would make more sense but it's being super annoying ...
                                    .with_node(|mut node| node.position_type = PositionType::Absolute)
                                    .align(Align::new().right())
                                    .apply(padding_style(BoxEdge::ALL, padding.signal()))
                                    .child(
                                        El::<Text>::new()
                                        .text_font_signal(font_size.signal().map(TextFont::from_font_size))
                                        .text_color_signal(unhighlighted_color.signal().map(TextColor))
                                        .text_signal(filtered_count.signal().map(|count| count.to_string()).map(Text))
                                    )
                                })))
                            )
                        )
                        .item(
                            inspection_target_root_selector(
                                search_target_root.clone(),
                                search_target_root_focused.clone(),
                                column_gap.clone(),
                                border_radius.clone(),
                                border_color.clone(),
                                primary_background_color.clone(),
                                highlighted_color.clone(),
                                unhighlighted_color.clone(),
                                padding.clone(),
                                font_size.clone(),
                                border_width.clone(),
                                tertiary_background_color.clone(),
                            )
                        )
                    }))
                )
                // targeting
                .layer_signal(
                    signal::and(show_targeting.signal(), signal::not(collapsed.signal()))
                    .map_true(clone!((highlighted_color,
                        border_color,
                        primary_background_color,
                        tertiary_background_color,
                        padding,
                        unhighlighted_color,
                        font_size,
                        border_width,
                        border_radius,
                        second_target_focused,
                        third_target_focused,
                        third_target,
                        second_target,
                        first_target,
                        first_target_focused,
                        targeting_target_root,
                        targeting_target_root_focused
                    ) move || {
                        let first_target_hovered = Mutable::new(false);
                        let second_target_hovered = Mutable::new(false);
                        let third_target_hovered = Mutable::new(false);
                        Column::<Node>::new()
                        .apply(padding_style(BoxEdge::ALL, padding.signal()))
                        .global_z_index(GlobalZIndex(z_order("target/search")))
                        .apply(background_style(primary_background_color.signal()))
                        .align(Align::new().bottom())
                        .apply(border_radius_style(BoxCorner::TOP, border_radius.signal()))
                        .apply(border_width_style([BoxEdge::Top], border_width.signal()))
                        .apply(border_color_style(border_color.signal()))
                        .width(Val::Percent(100.))
                        .item({
                            let hovered = Mutable::new(false);
                            Column::<Node>::new()
                            .height(Val::Percent(100.))
                            .hovered_sync(hovered.clone())
                            .apply(
                                left_bordered_style(
                                    border_width.signal(),
                                    map_bool_signal(
                                        signal_or!(
                                            hovered.signal(),
                                            first_target_focused.signal(),
                                            second_target_focused.signal(),
                                            third_target_focused.signal()
                                        ).dedupe(),
                                        tertiary_background_color.clone(),
                                        border_color.clone()
                                    ),
                                    padding.signal()
                                )
                            )
                            .apply(padding_style([BoxEdge::Left], padding.signal()))
                            .item({
                                let hovered = Mutable::new(false);
                                Column::<Node>::new()
                                .height(Val::Percent(100.))
                                .hovered_sync(hovered.clone())
                                .apply(
                                    left_bordered_style(
                                        border_width.signal(),
                                        map_bool_signal(
                                            signal_or!(
                                                hovered.signal(),
                                                second_target_focused.signal(),
                                                third_target_focused.signal()
                                            ).dedupe(),
                                            tertiary_background_color.clone(),
                                            border_color.clone()
                                        ),
                                        padding.signal()
                                    )
                                )
                                .apply(padding_style([BoxEdge::Left], padding.signal()))
                                .item_signal(
                                    targeting_target_root.signal().neq(InspectionTargetRoot::Resource)
                                    .map_true(clone!((third_target, third_target_hovered, third_target_focused, highlighted_color, border_color, unhighlighted_color, padding, font_size, tertiary_background_color, border_width) move || {
                                        let hovered = Mutable::new(false);
                                        El::<Node>::new()
                                        .hovered_sync(hovered.clone())
                                        .child(
                                            base_text_input(third_target.clone(), identity, third_target_hovered.clone(), third_target_focused.clone(), None)
                                            .on_change_sync(third_target.clone())
                                            .apply(
                                                search_input_shared_properties(
                                                    third_target_hovered.clone(),
                                                    third_target_focused.clone(),
                                                    highlighted_color.clone(),
                                                    border_color.clone(),
                                                    unhighlighted_color.clone(),
                                                    padding.clone(),
                                                    font_size.clone(),
                                                    third_target.clone(),
                                                    tertiary_background_color.clone(),
                                                    always(PARSED_PATH_PLACEHOLDER),
                                                )
                                            )
                                        )
                                        .apply(left_bordered_style(border_width.signal(), map_bool_signal(signal::or(hovered.signal(), third_target_focused.signal()).dedupe(), tertiary_background_color.clone(), border_color.clone()), padding.signal()))
                                        .apply(padding_style([BoxEdge::Left], padding.signal()))
                                    }))
                                )
                                .item(
                                    base_text_input(second_target.clone(), identity, second_target_hovered.clone(), second_target_focused.clone(), None)
                                    .on_change_sync(second_target.clone())
                                    .apply(
                                        search_input_shared_properties(
                                            second_target_hovered.clone(),
                                            second_target_focused.clone(),
                                            highlighted_color.clone(),
                                            border_color.clone(),
                                            unhighlighted_color.clone(),
                                            padding.clone(),
                                            font_size.clone(),
                                            second_target.clone(),
                                            tertiary_background_color.clone(),
                                            targeting_target_root.signal().map(|root| match root {
                                                InspectionTargetRoot::Entity => "`Component`",
                                                InspectionTargetRoot::Resource => PARSED_PATH_PLACEHOLDER,
                                                InspectionTargetRoot::Asset => "handle name",
                                            }),
                                        )
                                    )
                                )
                            })
                            .item(
                                base_text_input(first_target.clone(), identity, first_target_hovered.clone(), first_target_focused.clone(), None)
                                .update_raw_el(|raw_el| raw_el.on_spawn(clone!((first_target_focused) move |_, _| first_target_focused.set(true))))
                                .on_change_sync(first_target.clone())
                                .apply(
                                    search_input_shared_properties(
                                        first_target_hovered.clone(),
                                        first_target_focused.clone(),
                                        highlighted_color.clone(),
                                        border_color.clone(),
                                        unhighlighted_color.clone(),
                                        padding.clone(),
                                        font_size.clone(),
                                        first_target.clone(),
                                        tertiary_background_color.clone(),
                                        targeting_target_root.signal().map(|root| match root {
                                            InspectionTargetRoot::Entity => "`Entity` or `Name`",
                                            InspectionTargetRoot::Resource => "`Resource`",
                                            InspectionTargetRoot::Asset => "`Asset`",
                                        }),
                                    )
                                )
                            )
                        })
                        .item(
                            inspection_target_root_selector(
                                targeting_target_root.clone(),
                                targeting_target_root_focused.clone(),
                                column_gap.clone(),
                                border_radius.clone(),
                                border_color.clone(),
                                primary_background_color.clone(),
                                highlighted_color.clone(),
                                unhighlighted_color.clone(),
                                padding.clone(),
                                font_size.clone(),
                                border_width.clone(),
                                tertiary_background_color.clone(),
                            )
                        )
                    }))
                )
        )
        .apply(resize_border(
            border_width.signal(),
            border_radius.signal(),
            border_color.clone(),
            tertiary_background_color.clone(),
            highlighted_color.clone(),
            collapsed.clone(),
            Some(self.wrapper_stack),
        ))
        .layer_signal(
            tooltip.signal_cloned().dedupe_cloned().map_some(clone!((primary_background_color, font_size, padding, border_width) move |TooltipData { text, .. }| {
                El::<Node>::new()
                .update_raw_el(clone!((font_size, padding, border_width) move |raw_el| {
                    raw_el
                    .insert(Tooltip)
                    .insert(Visibility::Hidden)
                    // .insert(RenderLayers::layer(x))
                    .on_spawn_with_system(move |
                        In(entity),
                        mut inspector_ancestor: InspectorAncestor,
                        tooltip_target_position: Query<&TooltipTargetPosition>,
                        mut move_tooltip_to_position: MoveTooltipToPosition,
                        mut commands: Commands,
                    | {
                        if let Some(inspector) = inspector_ancestor.get(entity) {
                            if let Ok(&TooltipTargetPosition(position)) = tooltip_target_position.get(inspector) {
                                // TODO: more intelligent way to get this height? waiting for node to reach "full size" is pretty cringe
                                let expected_tooltip_height = font_size.get() + padding.get() + border_width.get() * 2. + 3.;  // TODO: where did this 3. come from ?
                                move_tooltip_to_position.move_(entity, position, Some(expected_tooltip_height));
                                if let Some(mut entity) = commands.get_entity(entity) {
                                    entity.try_insert(Visibility::Visible);
                                }
                            }
                        }
                    })
                }))
                .align_content(Align::center())
                .global_z_index(GlobalZIndex(z_order("tooltip")))
                .with_node(clone!((padding, border_width) move |mut node| {
                    node.position_type = PositionType::Absolute;
                    // TODO: without setting these statically on spawn, the signals cause the text to noticably jump as the tooltip spawns
                    node.padding = UiRect::all(Val::Px(padding.get() / 2.));
                    node.border = UiRect::all(Val::Px(border_width.get()));
                }))
                .border_radius(BorderRadius::all(Val::Px(border_radius.get())))
                .apply(padding_style(BoxEdge::ALL, padding.signal().map(div(2.))))
                .apply(border_width_style(BoxEdge::ALL, border_width.signal()))
                .apply(border_color_style(border_color.signal()))
                .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
                .apply(background_style(primary_background_color.signal()))
                .child({
                    El::<Text>::new()
                    .text_font(TextFont::from_font_size(font_size.get()))
                    .text_font_signal(font_size.signal().map(TextFont::from_font_size))
                    .text(Text(text))
                    .apply(text_no_wrap)
                })
            }))
        )
        // TODO: move these to the inspector once the resize border wrapper is no longer needed
        .on_click_outside_with_system(|In((entity, _)), selected_inspector_option: Option<Res<SelectedInspector>>, mut commands: Commands| {
            if selected_inspector_option.as_deref().map(Deref::deref).copied() == Some(entity) {
                commands.remove_resource::<SelectedInspector>();
            }
        })
        .update_raw_el(clone!((search_focused, first_target_focused, second_target_focused, third_target_focused, search_target_root_focused, targeting_target_root, targeting_target_root_focused) move |raw_el| {
            raw_el
            .insert(InspectorMarker)
            .insert(TooltipHolder(tooltip.clone()))
            .insert(InspectorBloodline)
            .on_spawn_with_system(|
                In(entity): In<Entity>,
                default_ui_camera_option: Option<Single<(Entity, Option<&RenderLayers>), With<IsDefaultUiCamera>>>,
                camera_2ds: Query<(Entity, Option<&RenderLayers>), With<Camera2d>>,
                camera_3ds: Query<(Entity, Option<&RenderLayers>), With<Camera3d>>,
                parents: Query<&Parent>,
                mut commands: Commands,
            | {
                let (camera, render_layers_option) = default_ui_camera_option.as_deref().copied().or_else(|| camera_2ds.iter().next()).or_else(|| camera_3ds.iter().next()).unwrap_or_else(|| (commands.spawn(Camera2d).id(), None));
                let root = parents.iter_ancestors(entity).last().unwrap_or(entity);
                if let Some(mut entity) = commands.get_entity(root) {
                    entity.try_insert((UiRoot, TargetCamera(camera)));
                    if let Some(render_layers) = render_layers_option {
                        entity.try_insert(render_layers.clone());
                    }
                }
            })
            .on_event_with_system::<Pointer<Down>, _>(|In((entity, _)), mut commands: Commands| commands.insert_resource(SelectedInspector(entity)))
            .observe(|event: Trigger<SizeReached>, childrens: Query<&Children>, inspector_columns: Query<&InspectorColumn>, scroll_positions: Query<&ScrollPosition>, previous_scroll_positions: Query<&PreviousScrollPosition>, mut commands: Commands| {
                let entity = event.entity();
                // TODO: use relations
                for descendent in childrens.iter_descendants(entity) {
                    if inspector_columns.contains(descendent) {
                        if let Ok(&PreviousScrollPosition(y)) = previous_scroll_positions.get(entity) {
                            if let Ok(&ScrollPosition { offset_y, .. }) = scroll_positions.get(descendent) {
                                // need to keep setting the scroll position until it's reflected since the layout may be in flight
                                // when the size is first reached
                                if y == offset_y {
                                    if let Some(mut entity) = commands.get_entity(entity) {
                                        entity.remove::<(WaitForSize, PreviousScrollPosition)>();
                                    }
                                    // if let Some(&system) = SYNC_VISIBILITY_SYSTEM.get() {
                                    //     commands.run_system(system);
                                    // }
                                    return;
                                }
                            }
                            if let Some(mut entity) = commands.get_entity(descendent) {
                                entity.try_insert(ScrollPosition { offset_y: y, ..default() });
                            }
                        }
                        break;
                    }
                }
            })
            // TODO: the cross contamination here is pretty cringe, can we avoid it ? granularizing hotkey listeners is just as bad, likely requires relations
            .observe(clone!((first_target_focused, second_target_focused, third_target_focused, search_focused, show_search, show_targeting) move |_: Trigger<ShowSearch>| {
                if first_target_focused.get().not() & second_target_focused.get().not() & third_target_focused.get().not() {
                    show_targeting.set_neq(false);
                    search_focused.set_neq(true);
                    show_search.set_neq(true);
                }
            }))
            .observe(clone!((show_search) move |_: Trigger<HideSearch>| {
                show_search.set_neq(false);
            }))
            .observe(clone!((first_target_focused, show_targeting, second_target_focused, third_target_focused, search_focused, show_search) move |_: Trigger<ShowTargeting>| {
                if search_focused.get().not() && first_target_focused.get().not() && second_target_focused.get().not() && third_target_focused.get().not() {
                    show_search.set_neq(false);
                    first_target_focused.set_neq(true);
                    show_targeting.set_neq(true);
                }
            }))
            .observe(clone!((show_targeting) move |_: Trigger<HideTargeting>| {
                show_targeting.set_neq(false);
            }))
            .observe(clone!((search_focused, show_search, show_targeting, search_target_root_focused, targeting_target_root_focused, targeting_target_root) move |event: Trigger<Tab>| {
                if show_search.get() {
                    let focuseds = [search_target_root_focused.clone(), search_focused.clone()];
                    iter_focused(&focuseds, event.event());
                }
                if show_targeting.get() {
                    let mut focuseds = vec![targeting_target_root_focused.clone(), first_target_focused.clone(), second_target_focused.clone()];
                    if !matches!(targeting_target_root.get(), InspectionTargetRoot::Resource) {
                        focuseds.push(third_target_focused.clone());
                    }
                    iter_focused(&focuseds, event.event());
                }
            }))
            .observe(clone!((search_target_root, targeting_target_root) move |event: Trigger<TargetRootMove>| {
                if show_search.get() && search_target_root_focused.get() {
                    iter_target_root(&search_target_root, event.event());
                }
                if show_targeting.get() && targeting_target_root_focused.get() {
                    iter_target_root(&targeting_target_root, event.event());
                }
            }))
        }))
        .apply(background_style(primary_background_color.signal()))
        .cursor(CursorIcon::System(SystemCursorIcon::Default))
        .height_signal(height.signal().map(Val::Px))
        .width_signal(width.signal().map(Val::Px))
        .global_z_index(GlobalZIndex(z_order("inspector")))
    }
}

const SCROLLBAR_WIDTH_BIG: f32 = 7.;
const SCROLLBAR_WIDTH_SMOL: f32 = 3.;
const SCROLLBAR_PADDING: f32 = 1.;
const SCROLLBAR_DEADZONE: f32 = 4.;

impl Inspector {
    pub fn new() -> Self {
        Self {
            el: Column::<Node>::new(),
            wrapper_stack: Stack::<Node>::new(),
            entities: MutableBTreeMap::new(),
            entities_transformers: Arc::new(Mutex::new(vec![])),
            components_transformers: Arc::new(Mutex::new(vec![])),
            resources: MutableBTreeMap::new(),
            assets: MutableBTreeMap::new(),
            search: Mutable::new(String::new()),
            first_target: Mutable::new(String::new()),
            second_target: Mutable::new(String::new()),
            third_target: Mutable::new(String::new()),
            height: Mutable::new(DEFAULT_HEIGHT),
            width: Mutable::new(DEFAULT_WIDTH),
            font_size: GLOBAL_FONT_SIZE.clone(),
            row_gap: GLOBAL_ROW_GAP.clone(),
            column_gap: GLOBAL_COLUMN_GAP.clone(),
            padding: GLOBAL_PADDING.clone(),
            border_radius: GLOBAL_BORDER_RADIUS.clone(),
            border_width: GLOBAL_BORDER_WIDTH.clone(),
            primary_background_color: GLOBAL_PRIMARY_BACKGROUND_COLOR.clone(),
            secondary_background_color: GLOBAL_SECONDARY_BACKGROUND_COLOR.clone(),
            tertiary_background_color: GLOBAL_TERTIARY_BACKGROUND_COLOR.clone(),
            highlighted_color: GLOBAL_HIGHLIGHTED_COLOR.clone(),
            unhighlighted_color: GLOBAL_UNHIGHLIGHTED_COLOR.clone(),
            border_color: GLOBAL_BORDER_COLOR.clone(),
            scroll_pixels: GLOBAL_SCROLL_PIXELS.clone(),
            header: Mutable::new(None),
            flatten_descendants: false,
        }
    }

    pub fn entities(mut self, mut entities: MutableBTreeMap<Entity, EntityData>) -> Self {
        std::mem::swap(&mut self.entities, &mut entities);
        self
    }

    pub fn resources(mut self, mut resources: MutableBTreeMap<ComponentId, FieldData>) -> Self {
        std::mem::swap(&mut self.resources, &mut resources);
        self
    }

    pub fn assets(mut self, mut assets: MutableBTreeMap<TypeId, AssetData>) -> Self {
        std::mem::swap(&mut self.assets, &mut assets);
        self
    }

    pub fn with_entities(
        self,
        f: impl FnMut(EntitySignalVec) -> EntitySignalVec + Send + 'static,
    ) -> Self {
        self.entities_transformers.lock().unwrap().push(Box::new(f));
        self
    }

    pub fn filter_entities_with_system<Marker>(
        self,
        handler: impl IntoSystem<In<Entity>, bool, Marker> + Send + 'static,
    ) -> Self {
        let system_holder = Arc::new(OnceLock::new());
        self
        .update_raw_el(clone!((system_holder) move |raw_el| {
            raw_el
            .on_spawn(clone!((system_holder) move |world, _| {
                let _ = system_holder.set(register_system(world, handler));
            }))
            .apply(remove_system_holder_on_remove(system_holder))
        }))
        .with_entities(move |entities| {
            entities
                .filter_signal_cloned(clone!((system_holder) move |&(entity, _)| {
                    // TODO: use async closure and from_future here after rust 2024 upgrade
                    always(entity).map_future(clone!((system_holder) move |entity| clone!((system_holder) async move {
                        let result = Mutable::new(None);
                        async_world().apply(clone!((result) move |world: &mut World| {
                            result.set(Some(world.run_system_with_input(system_holder.get().copied().unwrap(), entity).ok().unwrap_or(false)));
                        })).await;
                        result.signal_ref(Option::is_some).wait_for(true).await;
                        result.get().unwrap_or(false)
                    })))
                    .map(|result| result.unwrap_or(false))
                }))
                .boxed()
        })
    }

    pub fn with_components(
        self,
        f: impl FnMut(ComponentsSignalVec) -> ComponentsSignalVec + Send + 'static,
    ) -> Self {
        self.components_transformers
            .lock()
            .unwrap()
            .push(Box::new(f));
        self
    }

    pub fn jump_to(mut self, target: impl Into<InspectionTarget>) -> Self {
        let target = target.into();
        self.el = self.el.update_raw_el(move |raw_el| {
            raw_el.with_entity(|mut entity| {
                entity.insert(target);
            })
        });
        self
    }

    pub fn flatten_descendants(mut self) -> Self {
        self.flatten_descendants = true;
        self
    }

    impl_syncers! {
        height: f32,
        width: f32,
        font_size: f32,
        row_gap: f32,
        column_gap: f32,
        padding: f32,
        border_radius: f32,
        border_width: f32,
        primary_background_color: Color,
        secondary_background_color: Color,
        tertiary_background_color: Color,
        highlighted_color: Color,
        unhighlighted_color: Color,
        border_color: Color,
        scroll_pixels: f32,
        header: Option<String>,
    }
}

#[derive(Clone, Default)]
pub struct FieldData {
    pub name: String,
    pub expanded: Mutable<bool>,
    pub filtered: Mutable<bool>,
    viewability: Mutable<Viewability>,
}

#[derive(Component)]
#[require(SyncComponentsOnce)]
pub struct EntityRoot {
    entity: Entity, // target
    components: HashSet<ComponentId>,
    name: Mutable<Option<String>>,
}

#[derive(Component, Default)]
struct SyncAssetHandlesOnce;

#[allow(dead_code)]
#[derive(Component)]
#[require(SyncAssetHandlesOnce)]
pub struct AssetRoot {
    asset: TypeId,
    handles: HashSet<UntypedAssetId>,
    name: &'static str,
}

#[derive(Component)]
struct RootHeader;

#[derive(Event)]
struct ComponentsAdded(Vec<ComponentId>);

#[derive(Event)]
struct ComponentsRemoved(Vec<ComponentId>);

#[derive(Clone)]
enum MultiFieldData {
    Entity { id: Entity, data: EntityData },
    Asset { id: TypeId, data: AssetData },
}

struct MultiFieldElement {
    el: Column<Node>,
    data: MultiFieldData,
    show_name: bool,
    font_size: Mutable<f32>,
    row_gap: Mutable<f32>,
    column_gap: Mutable<f32>,
    primary_background_color: Mutable<Color>,
    secondary_background_color: Mutable<Color>,
    tertiary_background_color: Mutable<Color>,
    border_width: Mutable<f32>,
    border_color: Mutable<Color>,
    padding: Mutable<f32>,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    expanded: Mutable<bool>,
}

fn lax_type_path_match(left: &str, right: &str) -> bool {
    left.to_lowercase() == right.to_lowercase()
        || Some(left.to_lowercase().as_str()) == right.to_lowercase().split("::").last()
}

#[allow(dead_code)]
impl MultiFieldElement {
    impl_syncers! {
        font_size: f32,
        row_gap: f32,
        column_gap: f32,
        primary_background_color: Color,
        secondary_background_color: Color,
        border_width: f32,
        border_color: Color,
        padding: f32,
        highlighted_color: Color,
        unhighlighted_color: Color,
        expanded: bool,
    }

    fn new(data: MultiFieldData) -> Self {
        let font_size = Mutable::new(DEFAULT_FONT_SIZE);
        let row_gap = Mutable::new(DEFAULT_ROW_GAP);
        let column_gap = Mutable::new(DEFAULT_COLUMN_GAP);
        let primary_background_color = Mutable::new(DEFAULT_PRIMARY_BACKGROUND_COLOR);
        let secondary_background_color = Mutable::new(DEFAULT_SECONDARY_BACKGROUND_COLOR);
        let tertiary_background_color = Mutable::new(DEFAULT_TERTIARY_BACKGROUND_COLOR);
        let border_width = Mutable::new(DEFAULT_BORDER_WIDTH);
        let border_color = Mutable::new(DEFAULT_BORDER_COLOR);
        let padding = Mutable::new(DEFAULT_PADDING);
        let highlighted_color = Mutable::new(DEFAULT_HIGHLIGHTED_COLOR);
        let unhighlighted_color = Mutable::new(DEFAULT_UNHIGHLIGHTED_COLOR);
        Self {
            el: Column::<Node>::new(),
            expanded: match &data {
                MultiFieldData::Entity {
                    data: entity_data, ..
                } => entity_data.expanded.clone(),
                MultiFieldData::Asset {
                    data: asset_data, ..
                } => asset_data.expanded.clone(),
            },
            data,
            show_name: false,
            font_size,
            row_gap,
            column_gap,
            primary_background_color,
            secondary_background_color,
            tertiary_background_color,
            border_width,
            border_color,
            padding,
            highlighted_color,
            unhighlighted_color,
        }
    }

    fn show_name(mut self) -> Self {
        self.show_name = true;
        self
    }
}

#[derive(Component, Default)]
pub struct SyncComponents;

#[derive(Component, Default)]
pub struct Expanded;

const SHADOW_HEIGHT: f32 = 5.;

fn header_wrapper<E: Element, Marker>(
    hovered: Mutable<bool>,
    on_click: impl IntoSystem<In<(Entity, Pointer<Click>)>, (), Marker> + Send + 'static,
    row_gap: Mutable<f32>,
    primary_background_color: Mutable<Color>,
    secondary_background_color: Mutable<Color>,
    padding: Mutable<f32>,
    pinned: Mutable<bool>,
) -> impl FnOnce(E) -> El<Node> {
    move |el| {
        // let last_expanded_header = Mutable::new(false);
        El::<Node>::new()
            .apply(padding_style(
                BoxEdge::VERTICAL,
                row_gap.signal().map(div(2.)),
            ))
            .apply(padding_style(BoxEdge::HORIZONTAL, padding.signal()))
            .width(Val::Percent(100.))
            .apply(background_style(map_bool_signal(
                hovered.signal(),
                secondary_background_color,
                primary_background_color,
            )))
            .on_click_with_system(on_click)
            // TODO: all of these box shadow signal strats cause deadlocks ?? maybe something wrong w box shadows but more likely signal skill issue ...
            // .box_shadow_signal(
            //     pinned.signal().map_true(||
            //         BoxShadow {
            //             color: Color::BLACK.with_alpha(0.5),
            //             x_offset: Val::ZERO,
            //             y_offset: Val::Px(4.),  // TODO: this should be relative
            //             spread_radius: Val::ZERO,
            //             blur_radius: Val::ZERO,
            //         }
            //     )
            // )
            // .update_raw_el(|raw_el| {
            //     raw_el
            //     .component_signal::<BoxShadow, _>(
            //         pinned.signal().map_true(||
            //             BoxShadow {
            //                 color: Color::BLACK.with_alpha(0.5),
            //                 x_offset: Val::ZERO,
            //                 y_offset: Val::Px(4.),  // TODO: this should be relative
            //                 spread_radius: Val::ZERO,
            //                 blur_radius: Val::ZERO,
            //             }
            //         )
            //     )
            //     .on_signal_with_entity(pinned.signal(), |mut entity, pinned| {
            //         if pinned {
            //             entity.insert(
            //                 BoxShadow {
            //                     color: Color::BLACK.with_alpha(0.5),
            //                     x_offset: Val::ZERO,
            //                     y_offset: Val::Px(4.),  // TODO: this should be relative
            //                     spread_radius: Val::ZERO,
            //                     blur_radius: Val::ZERO,
            //                 }
            //             );
            //         } else {
            //             entity.remove::<BoxShadow>();
            //         }
            //     })
            // })
            .child(el)
            // TODO: shouldn't use El here but Stack is being weird with heights
            .child_signal(pinned.signal().map_true(|| {
                // faux shadow
                El::<Node>::new()
                    .width(Val::Percent(100.))
                    .height(Val::Px(SHADOW_HEIGHT))
                    .border_radius(BorderRadius::top(Val::Px(f32::MAX)))
                    .with_node(|mut node| {
                        node.position_type = PositionType::Absolute;
                        node.top = Val::Percent(100.);
                        node.left = Val::Px(0.); // TODO: wut in tarnation ? (faux shadow doesn't left align without this for some reason)
                    })
                    .background_color(BackgroundColor(Color::BLACK.with_alpha(0.5)))
                    .update_raw_el(|raw_el| {
                        raw_el.on_spawn_with_system(
                            |In(entity): In<Entity>,
                             parents: Query<&Parent>,
                             global_zindices: Query<&GlobalZIndex>,
                             mut commands: Commands| {
                                if let Ok(z_index) = parents
                                    .get(entity)
                                    .and_then(|parent| global_zindices.get(parent.get()))
                                {
                                    if let Some(mut entity) = commands.get_entity(entity) {
                                        entity.try_insert(*z_index);
                                    }
                                }
                            },
                        )
                    })
            }))
    }
}

fn text_no_wrap<E: Element>(el: E) -> E {
    el.update_raw_el(|raw_el| {
        raw_el.with_component::<TextLayout>(|mut text_layout| {
            text_layout.linebreak = LineBreak::NoWrap
        })
    })
}

fn entity_header(
    entity: Entity,
    name: Mutable<Option<String>>,
    hovered: Mutable<bool>,
    font_size: Mutable<f32>,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
) -> impl Element + PointerEventAware {
    let guessed_name = Mutable::new("Entity".to_string());
    HighlightableText::new()
    .highlighted_signal(hovered.signal())
    .with_text(clone!((font_size, guessed_name) move |text| {
        text
        .text_signal(name.signal_cloned().map_option(|name| always(name).apply(boxed_sync), move || guessed_name.signal_cloned().apply(boxed_sync)).flatten().map(move |prefix| format!("{prefix} ({entity})")))
        .font_size_signal(font_size.signal())
        .apply(text_no_wrap)
    }))
    .highlighted_color_signal(highlighted_color.signal())
    .unhighlighted_color_signal(unhighlighted_color.signal())
    .update_raw_el(move |raw_el| raw_el.on_spawn_with_system(move |In(_), entities: &Entities, archetypes: &Archetypes, components: &Components| {
        if let Some(location) = entities.get(entity) {
            if let Some(archetype) = archetypes.get(location.archetype_id) {
                // from bevy-inspector-egui https://github.com/jakobhellermann/bevy-inspector-egui/blob/b54c53046f6765aa893c975dcea00e28468d922f/crates/bevy-inspector-egui/src/utils.rs#L56-L69
                let associations = &[
                    ("bevy_window::window::PrimaryWindow", "PrimaryWindow"),
                    ("bevy_core_pipeline::core_3d::camera_3d::Camera3d", "Camera3d"),
                    ("bevy_core_pipeline::core_2d::camera_2d::Camera2d", "Camera2d"),
                    ("bevy_pbr::light::point_light::PointLight", "PointLight"),
                    ("bevy_pbr::light::directional_light::DirectionalLight", "DirectionalLight"),
                    ("bevy_text::text::Text", "Text"),
                    ("bevy_ui::ui_node::Node", "Node"),
                    ("bevy_asset::handle::Handle<bevy_pbr::pbr_material::StandardMaterial>", "Pbr Mesh"),
                    ("bevy_window::window::Window", "Window"),
                    ("bevy_ecs::observer::runner::ObserverState", "Observer"),
                    ("bevy_window::monitor::Monitor", "Monitor"),
                    ("bevy_picking::pointer::PointerId", "Pointer"),
                ];
                let type_names = archetype.components().filter_map(|id| {
                    components.get_info(id).map(|info| info.name())
                });
                for component_type in type_names {
                    for &(name, matches) in associations {
                        if component_type == name {
                            guessed_name.set(matches.to_string());
                            return
                        }
                    }
                }
            }
        }
    }))
}

pub fn apply_to_accessory_target(
    world: &mut World,
    target: AccessoryTarget,
    f: impl FnOnce(&mut dyn Reflect),
) {
    match target {
        AccessoryTarget::Component { owner, component } => match owner {
            ComponentOwnerType::Entity(entity) => {
                with_reflect_component_mut(world, entity, component, f);
            }
            ComponentOwnerType::Resource => {
                with_reflect_resource_mut(world, component, f);
            }
        },
        AccessoryTarget::Asset { asset, handle } => {
            with_reflect_asset_mut(world, asset, handle, f);
        }
    }
}

#[derive(Component)]
struct HeaderData {
    pinned: Mutable<bool>,
    expanded: Mutable<bool>,
}

#[derive(Component)]
struct EntitiesHeader;

#[derive(Component)]
struct ResourcesHeader;

#[derive(Component)]
struct AssetsHeader;

fn sync_on_expanded_and_visibility<T: Component + Default>(el: RawHaalkaEl) -> RawHaalkaEl {
    el.observe(
        |event: Trigger<OnAdd, Visible>, expandeds: Query<&Expanded>, mut commands: Commands| {
            let entity = event.entity();
            if expandeds.contains(entity) {
                if let Some(mut entity) = commands.get_entity(entity) {
                    entity.try_insert(T::default());
                }
            }
        },
    )
    .observe(
        |event: Trigger<OnRemove, Visible>, mut commands: Commands| {
            let entity = event.entity();
            if let Some(mut entity) = commands.get_entity(entity) {
                entity.remove::<T>();
            }
        },
    )
    .observe(
        |event: Trigger<OnAdd, Expanded>, visibles: Query<&Visible>, mut commands: Commands| {
            let entity = event.entity();
            if visibles.contains(entity) {
                if let Some(mut entity) = commands.get_entity(entity) {
                    entity.try_insert(T::default());
                }
            }
        },
    )
    .observe(
        |event: Trigger<OnRemove, Expanded>, mut commands: Commands| {
            let entity = event.entity();
            if let Some(mut entity) = commands.get_entity(entity) {
                entity.remove::<T>();
            }
        },
    )
}

impl ElementWrapper for MultiFieldElement {
    type EL = Column<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el,
            data,
            show_name,
            font_size,
            row_gap,
            column_gap,
            primary_background_color,
            secondary_background_color,
            tertiary_background_color,
            border_width,
            border_color,
            padding,
            highlighted_color,
            unhighlighted_color,
            ..
        } = self;
        let hovered = Mutable::new(false);
        let pinned = Mutable::new(false);
        let expanded = match &data {
            MultiFieldData::Entity {
                data: EntityData { expanded, .. },
                ..
            } => expanded.clone(),
            MultiFieldData::Asset {
                data: AssetData { expanded, .. },
                ..
            } => expanded.clone(),
        };
        el
        .update_raw_el(clone!((data, pinned, expanded) move |mut raw_el| {
            raw_el = match &data {
                MultiFieldData::Entity { id: entity, data: EntityData { name, components, .. } } => {
                    raw_el
                    .insert(EntityRoot { entity: *entity, components: HashSet::from_iter(components.lock_ref().iter().map(|(&id, _)| id)), name: name.clone() })
                },
                MultiFieldData::Asset { id: asset, data: AssetData { name, handles, .. } } => {
                    raw_el
                    .insert(AssetRoot { asset: *asset, handles: HashSet::from_iter(handles.lock_ref().iter().map(|(&id, _)| id)), name })
                },
            }
            .insert(HeaderData { pinned: pinned.clone(), expanded: expanded.clone() })
            .component_signal::<Expanded, _>(expanded.signal().dedupe().map_true(default))
            .apply(listen_to_expanded_component(expanded.clone()))
            .observe(clone!((expanded, data) move |
                event: Trigger<CheckInspectionTargets>,
                parents: Query<&Parent>,
                childrens: Query<&Children>,
                inspection_targets: Query<&InspectionTarget>,
                mut commands: Commands
            | {
                let ui_entity = event.entity();
                for parent in parents.iter_ancestors(ui_entity) {
                    if let Ok(target) = inspection_targets.get(parent) {
                        if matches!(target.root, InspectionTargetRoot::Entity | InspectionTargetRoot::Asset) {
                            if let Some(InspectionTargetInner::Multi(target)) = &target.target {
                                let target_string = target.name.to_lowercase();
                                let matches_ = match &data {
                                    MultiFieldData::Entity { id: entity, data: EntityData { name, .. } } => {
                                        target_string == entity.to_string() || Some(target_string) == name.get_cloned().map(|name| name.to_lowercase())
                                    },
                                    MultiFieldData::Asset { data: AssetData { name, .. }, .. } => {
                                        lax_type_path_match(&target_string, name)
                                    }
                                };
                                if matches_ {
                                    for ancestor in parents.iter_ancestors(ui_entity) {
                                        if let Some(mut entity) = commands.get_entity(ancestor) {
                                            entity.remove::<WaitForBirth>();
                                        }
                                    }
                                    if let Some(mut entity) = commands.get_entity(ui_entity) {
                                        let mut pending = VecDeque::new();
                                        if let Some(InspectionTargetField { field, path }) = &target.field {
                                            pending.push_back(ProgressPart::Field(field.clone()));
                                            if let Some(path) = &path {
                                                for OffsetAccess { access, .. } in &path.0 {
                                                    pending.push_back(ProgressPart::Access(access.clone()));
                                                }
                                            }
                                        }
                                        entity.try_insert(InspectionTargetProgress { pending });
                                        entity.try_insert(WaitForBirth { ceiling: None });
                                        // TODO: use relations to safely get the entity's component children
                                        if let Some(&child) = i_born(ui_entity, &childrens, 1) {
                                            if let Ok(children) = childrens.get(child) {
                                                commands.trigger_targets(CheckInspectionTargets, children.iter().copied().collect::<Vec<_>>());
                                            }
                                        }
                                    }
                                    expanded.set_neq(true);
                                    return
                                }
                            }
                        }
                    }
                }
            }))
            .apply(scroll_to_header_on_birth)
            .on_spawn_with_system(|In(entity), mut commands: Commands| commands.trigger_targets(CheckInspectionTargets, entity));
            match &data {
                MultiFieldData::Entity { data: EntityData { components, .. }, .. }  => {
                    raw_el = raw_el
                    .observe(clone!((components => components_map) move |event: Trigger<ComponentsAdded>, components: &Components| {
                        let ComponentsAdded(added) = event.event();
                        let mut lock = components_map.lock_mut();
                        for &component in added {
                            if let Some(info) = components.get_info(component) {
                                lock.insert_cloned(component, FieldData { name: info.name().to_string(), ..default() });
                            }
                        }
                    }))
                    .observe(clone!((components) move |event: Trigger<ComponentsRemoved>| {
                        let ComponentsRemoved(removed) = event.event();
                        let mut lock = components.lock_mut();
                        for id in removed {
                            lock.remove(id);
                        }
                    }))
                    .apply(sync_on_expanded_and_visibility::<SyncComponents>)
                },
                MultiFieldData::Asset { data: AssetData { handles, .. }, .. } => {
                    raw_el = raw_el
                    .observe(clone!((handles) move |event: Trigger<AssetHandlesAdded>, asset_server: Res<AssetServer>| {
                        let AssetHandlesAdded(added) = event.event();
                        let mut lock = handles.lock_mut();
                        for &handle in added {
                            lock.insert_cloned(handle, FieldData { name: handle_name(handle, &asset_server), ..default() });
                        }
                    }))
                    .observe(clone!((handles) move |event: Trigger<AssetHandlesRemoved>| {
                        let AssetHandlesRemoved(removed) = event.event();
                        let mut lock = handles.lock_mut();
                        for id in removed {
                            lock.remove(id);
                        }
                    }))
                    .apply(sync_on_expanded_and_visibility::<SyncAssetHandles>)
                }
            }
            raw_el
        }))
        .hovered_sync(hovered.clone())
        .item(if show_name {
            match &data {
                MultiFieldData::Entity { id: entity, data: EntityData { name, .. } } => {
                    let entity = *entity;
                    entity_header(
                        entity,
                        name.clone(),
                        hovered.clone(),
                        font_size,
                        highlighted_color.clone(),
                        unhighlighted_color.clone(),
                    )
                    .type_erase()
                },
                MultiFieldData::Asset { data: AssetData { name, .. }, .. } => {
                    field_header(
                        name.split("::").last().unwrap_or_default().to_string(),
                        Some(FieldType::Field(name.to_string())),
                        Mutable::new(Some(name.to_string())),
                        Mutable::new(Viewability::Viewable),
                        hovered.clone(),
                        column_gap.clone(),
                        highlighted_color.clone(),
                        unhighlighted_color.clone(),
                        Mutable::new(DEFAULT_ERROR_COLOR),
                        Mutable::new(DEFAULT_TERTIARY_BACKGROUND_COLOR),
                    )
                    .type_erase()
                }
            }
            .apply(
                header_wrapper(
                    hovered.clone(),
                    clone!((expanded) move |
                        In((entity, click)): In<(Entity, Pointer<Click>)>,
                        headers: Query<&HeaderData>,
                        relative_rect: RelativeRect,
                        mut maybe_scroll_to_header_root: MaybeScrollToHeaderRoot,
                        parents: Query<&Parent>
                    | {
                        if matches!(click.button, PointerButton::Primary) {
                            let mut i = -1;  // don't count current header
                            for ancestor in parents.iter_ancestors(entity) {
                                if headers.contains(ancestor) {
                                    i += 1;
                                }
                            }
                            if let Some(rect) = relative_rect.get(entity) {
                                let partially_under = rect.min.y < rect.size().y * i as f32;
                                if !maybe_scroll_to_header_root.scrolled(entity, !partially_under) || partially_under && expanded.get().not()
                                {
                                    flip(&expanded)
                                }
                            }
                        }
                    }),
                    row_gap.clone(),
                    primary_background_color,
                    secondary_background_color.clone(),
                    padding.clone(),
                    pinned.clone(),
                )
            )
            .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
            .apply(Some)
        } else {
            None
        })
        .item_signal(if show_name { expanded.signal().dedupe().boxed() } else { always(true).boxed() }.map_true(clone!((row_gap, column_gap, border_width, border_color, padding, highlighted_color, unhighlighted_color, hovered, data) move || {
            Column::<Node>::new()
                .apply(move_style(Move_::Right, padding.signal()))
                .apply(left_bordered_style(border_width.signal(), map_bool_signal(hovered.signal(), tertiary_background_color.clone(), border_color.clone()), padding.signal()))
                .items_signal_vec({
                    match &data {
                        MultiFieldData::Entity { id: entity, data: EntityData { components, .. } } => {
                            let entity = *entity;
                            components.entries_cloned()
                            // this is an emulation of something like .sort_by_signal_cloned
                            .map_signal(|(component, data)| {
                                data.viewability.signal().map(move |cur| (component, data.clone(), cur))
                            })
                            .sort_by_cloned(|(_, FieldData { name: left_name, .. }, left_viewability), (_, FieldData { name: right_name, .. }, right_viewability)| left_viewability.cmp(right_viewability).reverse().then(type_path_ord(left_name, right_name)))
                            //
                            .map(clone!((row_gap, column_gap, tertiary_background_color, border_width, border_color, padding, highlighted_color, unhighlighted_color) move |(component, FieldData { name, expanded, viewability, .. }, _)| {
                                FieldElement::new(FieldElementInput::Component { owner: ComponentOwnerType::Entity(entity), component }, FieldType::Field(name), viewability)
                                .row_gap_signal(row_gap.signal())
                                .column_gap_signal(column_gap.signal())
                                .type_path_color_signal(tertiary_background_color.signal())
                                .border_width_signal(border_width.signal())
                                .border_color_signal(border_color.signal())
                                .padding_signal(padding.signal())
                                .highlighted_color_signal(highlighted_color.signal())
                                .unhighlighted_color_signal(unhighlighted_color.signal())
                                .expanded_signal(expanded.signal().dedupe())
                            }))
                            .boxed()
                        },
                        MultiFieldData::Asset { id: asset, data: AssetData { handles, .. } } => {
                            let asset = *asset;
                            handles.entries_cloned()
                            // this is an emulation of something like .sort_by_signal_cloned
                            .map_signal(|(handle, data)| {
                                data.viewability.signal().map(move |cur| (handle, data.clone(), cur))
                            })
                            .sort_by_cloned(|(_, FieldData { name: left_name, .. }, left_viewability), (_, FieldData { name: right_name, .. }, right_viewability)| left_viewability.cmp(right_viewability).reverse().then(type_path_ord(left_name, right_name)))
                            //
                            .map(clone!((row_gap, column_gap, tertiary_background_color, border_width, border_color, padding, highlighted_color, unhighlighted_color) move |(handle, FieldData { name, expanded, viewability, .. }, _)| {
                                FieldElement::new(FieldElementInput::Asset { asset, handle }, FieldType::Field(name), viewability)
                                .row_gap_signal(row_gap.signal())
                                .column_gap_signal(column_gap.signal())
                                .type_path_color_signal(tertiary_background_color.signal())
                                .border_width_signal(border_width.signal())
                                .border_color_signal(border_color.signal())
                                .padding_signal(padding.signal())
                                .highlighted_color_signal(highlighted_color.signal())
                                .unhighlighted_color_signal(unhighlighted_color.signal())
                                .expanded_signal(expanded.signal().dedupe())
                            }))
                            .boxed()
                        }
                    }
                })
        })))
    }
}

fn type_path_ord(left: &str, right: &str) -> Ordering {
    left.split("::").last().cmp(&right.split("::").last())
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum FieldType {
    Field(String),
    Access(Access<'static>),
}

#[derive(Clone)]
struct AccessFieldData {
    access: Access<'static>,
    viewability: Mutable<Viewability>,
}

impl AccessFieldData {
    fn new(access: Access<'static>) -> Self {
        Self {
            access,
            viewability: Mutable::new(Viewability::Viewable),
        }
    }
}

struct FieldElement {
    el: Column<Node>,
    row_gap: Mutable<f32>,
    column_gap: Mutable<f32>,
    border_width: Mutable<f32>,
    border_color: Mutable<Color>,
    padding: Mutable<f32>,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    type_path_color: Mutable<Color>,
    expanded: Mutable<bool>,
}

impl ElementWrapper for FieldElement {
    type EL = Column<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

#[derive(Clone)]
enum NodeType {
    Solo(String), // type path
    Multi {
        items: MutableVec<AccessFieldData>,
        size_dynamic: Option<ReflectKind>,
    },
}

#[derive(Clone)]
struct VariantData {
    variant: &'static str,
    has_default: bool,
}

fn get_variant_info(enum_: &dyn Enum, variant: usize) -> Option<&VariantInfo> {
    if let Some(TypeInfo::Enum(enum_info)) = enum_.get_represented_type_info() {
        enum_info.variant_at(variant)
    } else {
        None
    }
}

fn populate_enum_with_variant(
    enum_: &dyn Enum,
    variant: usize,
    node_type: &Mutable<Option<NodeType>>,
) {
    if let Some(variant_info) = get_variant_info(enum_, variant) {
        match variant_info {
            VariantInfo::Struct(struct_info) => {
                let mut fields = vec![];
                for i in 0..struct_info.field_len() {
                    if let Some(name) = struct_info.field_at(i).map(|field| field.name()) {
                        let access = Access::Field(name.to_string().into());
                        fields.push(AccessFieldData::new(access));
                    }
                }
                node_type.set(Some(NodeType::Multi {
                    items: fields.into(),
                    size_dynamic: None,
                }));
            }
            VariantInfo::Tuple(tuple_info) => {
                let mut fields = vec![];
                for i in 0..tuple_info.field_len() {
                    let access = Access::TupleIndex(i);
                    fields.push(AccessFieldData::new(access));
                }
                node_type.set(Some(NodeType::Multi {
                    items: fields.into(),
                    size_dynamic: None,
                }));
            }
            VariantInfo::Unit(_) => {
                // TODO: unit enum indicator
                node_type.take();
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn field_header(
    name: String,
    field_type: Option<FieldType>,
    type_path: Mutable<Option<String>>,
    viewability: Mutable<Viewability>,
    hovered: Mutable<bool>,
    // styles TODO: higher level abstraction for managing styles
    column_gap: Mutable<f32>,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    error_color: Mutable<Color>,
    type_path_color: Mutable<Color>,
) -> impl Element + CursorOnHoverable {
    Row::<Node>::new()
    .apply(row_style(column_gap.signal()))
    .item_signal(
        viewability.signal().map(|viewability| !matches!(viewability, Viewability::NotInRegistry)).map_bool(
        clone!((name, highlighted_color, unhighlighted_color, hovered) move || HighlightableText::new().with_text(|text| text.text(name.clone()))
            .highlighted_color_signal(highlighted_color.signal())
            .unhighlighted_color_signal(unhighlighted_color.signal())
            .highlighted_signal(hovered.signal())
            .type_erase()),
        move || {
            DynamicText::new()
            .text(name.clone())
            .color_signal(error_color.signal())
            .into_el()
            .type_erase()
        })
        .map(|el| el.align(Align::new().top()).apply(text_no_wrap))
    )
    .item_signal(
        if let Some(FieldType::Field(type_path)) = field_type {
            hovered.signal()
            .map_true(clone!((type_path_color, type_path) move || {
                DynamicText::new()
                .text(type_path.clone())
                .color_signal(type_path_color.signal())
            }))
            .boxed()
        } else {
            type_path.signal_cloned().map_some(clone!((hovered, type_path_color) move |type_path| {
                DynamicText::new()
                .text_signal(hovered.signal().map_bool(clone!((type_path) move || type_path.clone()), move || ShortName(&type_path).to_string()))
                .color_signal(type_path_color.signal())
            }))
            .boxed()
        }
        .map(|el_option| el_option.map(text_no_wrap))
    )
}

#[derive(Event)]
struct CheckInspectionTargets;

#[allow(clippy::too_many_arguments)]
fn object_type_header_with_count(
    root: InspectionTargetRoot,
    hovered: Mutable<bool>,
    font_size: Mutable<f32>,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    count: impl Signal<Item = usize> + Send + Sync + 'static,
    row_gap: Mutable<f32>,
    primary_background_color: Mutable<Color>,
    secondary_background_color: Mutable<Color>,
    padding: Mutable<f32>,
    pinned: Mutable<bool>,
    expanded: Mutable<bool>,
) -> Column<Node> {
    Column::<Node>::new()
    .width(Val::Percent(100.))
    .hovered_sync(hovered.clone())
    .update_raw_el(|raw_el| {
        raw_el
        .insert(RootHeader)
        .insert(PickingBehavior::default())
        .insert(HeaderData { pinned: pinned.clone(), expanded: expanded.clone() })
        .component_signal::<Expanded, _>(expanded.signal().dedupe().map_true(default))
        .apply(listen_to_expanded_component(expanded.clone()))
        .observe(move |event: Trigger<OnRemove, Expanded>, mut commands: Commands| {
            commands.trigger_targets(RootCollapsed(root), event.entity());
        })
        .observe(clone!((expanded) move |
            event: Trigger<CheckInspectionTargets>,
            parents: Query<&Parent>,
            inspection_targets: Query<&InspectionTarget>,
            childrens: Query<&Children>,
            mut commands: Commands
        | {
            let ui_entity = event.entity();
            for parent in parents.iter_ancestors(ui_entity) {
                if let Ok(target) = inspection_targets.get(parent) {
                    if target.root == root {
                        if let Some(mut entity) = commands.get_entity(parent) {
                            entity.remove::<InspectionTarget>();
                        }
                        if let Some(mut entity) = commands.get_entity(ui_entity) {
                            entity.try_insert((target.clone(), WaitForBirth { ceiling: None }));
                        }
                        if let Some(&child) = i_born(ui_entity, &childrens, 1) {
                            if let Ok(children) = childrens.get(child) {
                                commands.trigger_targets(CheckInspectionTargets, children.iter().copied().collect::<Vec<_>>());
                            }
                        }
                        expanded.set_neq(true);
                        return
                    }
                }
            }
        }))
        .apply(scroll_to_header_on_birth)
        .on_spawn_with_system(|In(entity), mut commands: Commands| commands.trigger_targets(CheckInspectionTargets, entity))
    })
    .item(
        // TODO: use text spans for this
        Row::<Node>::new()
        .item(
            HighlightableText::new()
            .highlighted_signal(hovered.signal())
            .with_text(clone!((font_size) move |text| {
                text
                .text(format!("{} (", match root {
                    InspectionTargetRoot::Entity => "entities",
                    InspectionTargetRoot::Asset => "assets",
                    InspectionTargetRoot::Resource => "resources",
                }))
                .font_size_signal(font_size.signal())
                .apply(text_no_wrap)
            }))
            .highlighted_color_signal(highlighted_color.signal())
            .unhighlighted_color_signal(unhighlighted_color.signal())
        )
        .item(
            HighlightableText::new()
            .highlighted_signal(hovered.signal())
            .with_text(clone!((font_size) move |text| {
                text
                .text_signal(count.map(|len| len.to_string()))
                .font_size_signal(font_size.signal())
                .apply(text_no_wrap)
            }))
            .highlighted_color_signal(highlighted_color.signal())
            .unhighlighted_color_signal(unhighlighted_color.signal())
        )
        .item(
            HighlightableText::new()
            .highlighted_signal(hovered.signal())
            .with_text(clone!((font_size) move |text| {
                text
                .text(")".to_string())
                .font_size_signal(font_size.signal())
                .apply(text_no_wrap)
            }))
            .highlighted_color_signal(highlighted_color.signal())
            .unhighlighted_color_signal(unhighlighted_color.signal())
        )
        .apply(
            header_wrapper(
                hovered.clone(),
                clone!((expanded) move |
                    In((entity, click)): In<(Entity, Pointer<Click>)>,
                    headers: Query<&HeaderData>,
                    relative_rect: RelativeRect,
                    mut maybe_scroll_to_header_root: MaybeScrollToHeaderRoot,
                    parents: Query<&Parent>
                | {
                    if matches!(click.button, PointerButton::Primary) {
                        let mut i = -1;  // don't count current header
                        for ancestor in parents.iter_ancestors(entity) {
                            if headers.contains(ancestor) {
                                i += 1;
                            }
                        }
                        if let Some(rect) = relative_rect.get(entity) {
                            let partially_under = rect.min.y < rect.size().y * i as f32;
                            if !maybe_scroll_to_header_root.scrolled(entity, !partially_under) || partially_under && expanded.get().not()
                            {
                                flip(&expanded)
                            }
                        }
                    }
                }),
                row_gap.clone(),
                primary_background_color.clone(),
                secondary_background_color.clone(),
                padding.clone(),
                pinned
            )
        )
        .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
    )
}

#[derive(Component)]
struct FieldsColumn;

#[derive(Clone, Copy, Debug)]
pub enum ComponentOwnerType {
    Entity(Entity),
    Resource,
}

#[derive(Clone, Copy)]
enum FieldElementInput {
    Component {
        owner: ComponentOwnerType,
        component: ComponentId,
    },
    Asset {
        asset: TypeId,
        handle: UntypedAssetId,
    },
}

impl From<FieldElementInput> for AccessoryTarget {
    fn from(input: FieldElementInput) -> Self {
        match input {
            FieldElementInput::Component { owner, component } => {
                AccessoryTarget::Component { owner, component }
            }
            FieldElementInput::Asset { asset, handle } => AccessoryTarget::Asset { asset, handle },
        }
    }
}

#[allow(dead_code)]
impl FieldElement {
    fn new(
        input: FieldElementInput,
        field_type: FieldType,
        viewability: Mutable<Viewability>,
    ) -> Self {
        let row_gap = Mutable::new(DEFAULT_ROW_GAP);
        let column_gap = Mutable::new(DEFAULT_COLUMN_GAP);
        let border_width = Mutable::new(DEFAULT_BORDER_WIDTH);
        let border_color = Mutable::new(DEFAULT_BORDER_COLOR);
        let padding = Mutable::new(DEFAULT_PADDING);
        let highlighted_color = Mutable::new(DEFAULT_HIGHLIGHTED_COLOR);
        let unhighlighted_color = Mutable::new(DEFAULT_UNHIGHLIGHTED_COLOR);
        let error_color = Mutable::new(DEFAULT_ERROR_COLOR);
        let type_path_color = Mutable::new(DEFAULT_TERTIARY_BACKGROUND_COLOR);
        let primary_background_color = Mutable::new(DEFAULT_PRIMARY_BACKGROUND_COLOR);
        let secondary_background_color = Mutable::new(DEFAULT_SECONDARY_BACKGROUND_COLOR);
        let tertiary_background_color: Mutable<Color> =
            Mutable::new(DEFAULT_TERTIARY_BACKGROUND_COLOR);
        let font_size = GLOBAL_FONT_SIZE.clone();
        let expanded = Mutable::new(false);
        let pinned = Mutable::new(false);
        let (name, access_option) = match field_type.clone() {
            FieldType::Field(type_path) => (ShortName(&type_path).to_string(), None),
            FieldType::Access(access) => (access.to_string(), Some(access.clone())),
        };
        let type_path = Mutable::new(None);
        let node_type = Mutable::new(None);
        let enum_data_option = Mutable::new(None);
        let hovered = Mutable::new(false);
        // TODO: more intelligent way to get this height? waiting for node to reach "full size" is pretty cringe
        let expected_tooltip_height =
            font_size.get() + padding.get() + border_width.get() * 2. + 3.; // TODO: where did this 3. come from ?
        if let FieldType::Field(type_path) = &field_type {
            if has_frontend(type_path) {
                viewability.set_neq(Viewability::Viewable);
            }
        }
        let el = Column::<Node>::new()
            .hovered_sync(hovered.clone())
            .width(Val::Percent(100.))
            .update_raw_el(|raw_el| {
                raw_el
                .insert(HeaderData { pinned: pinned.clone(), expanded: expanded.clone() })
                .insert(PickingBehavior::default())
                .component_signal::<Expanded, _>(expanded.signal().dedupe().map_true(default))
                .apply(listen_to_expanded_component(expanded.clone()))
                .observe(clone!((expanded, field_type) move |
                    event: Trigger<CheckInspectionTargets>,
                    parents: Query<&Parent>,
                    childrens: Query<&Children>,
                    progresses: Query<&InspectionTargetProgress>,
                    inspection_targets: Query<&InspectionTarget>,
                    fields_columns: Query<&FieldsColumn>,
                    mut commands: Commands
                | {
                    let ui_entity = event.entity();
                    for parent in parents.iter_ancestors(ui_entity) {
                        // TODO: this should just be generalized for every header (a lot of logic repeated for multi fields + fields)
                        let mut pending_option = None;
                        if let Ok(target) = inspection_targets.get(parent) {
                            if matches!(target.root, InspectionTargetRoot::Resource) {
                                if let Some(InspectionTargetInner::Solo(target)) = &target.target {
                                    if let FieldType::Field(field) = &field_type {
                                        if lax_type_path_match(&target.field, field) {
                                            let mut pending = VecDeque::new();
                                            if let Some(path) = &target.path {
                                                for OffsetAccess { access, .. } in &path.0 {
                                                    pending.push_back(ProgressPart::Access(access.clone()));
                                                }
                                            }
                                            pending_option = Some(pending);
                                        }
                                    }
                                }
                            }
                        }
                        if let Ok(InspectionTargetProgress { pending }) = progresses.get(parent) {
                            if let Some(first) = pending.front() {
                                if match (first, field_type.clone()) {
                                    (ProgressPart::Field(target_field), FieldType::Field(field)) => {
                                        lax_type_path_match(target_field, &field)
                                    },
                                    (ProgressPart::Access(target_access), FieldType::Access(access)) => {
                                        target_access == &access
                                    },
                                    _ => false
                                } {
                                    // TODO: we can't remove the parent progress as we go for some reason ...
                                    // if let Some(mut entity) = commands.get_entity(parent) {
                                    //     entity.remove::<InspectionTargetProgress>();
                                    // }
                                    let mut pending = pending.clone();
                                    pending.pop_front();
                                    pending_option = Some(pending);
                                }
                            }
                        }
                        if let Some(pending) = pending_option {
                            for ancestor in parents.iter_ancestors(ui_entity) {
                                if let Some(mut entity) = commands.get_entity(ancestor) {
                                    entity.remove::<WaitForBirth>();
                                }
                            }
                            if let Some(mut entity) = commands.get_entity(ui_entity) {
                                if pending.is_empty() {
                                    entity.try_insert(WaitForBirth { ceiling: None });
                                } else {
                                    entity.try_insert(InspectionTargetProgress { pending });
                                    // TODO: use relations to safely get the field's fields column's children, this is excessive
                                    for child in childrens.iter_descendants(ui_entity) {
                                        if fields_columns.contains(child) {
                                            if let Ok(children) = childrens.get(child) {
                                                commands.trigger_targets(CheckInspectionTargets, children.iter().copied().collect::<Vec<_>>());
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                            expanded.set_neq(true);
                            return
                        }
                    }
                }))
                .apply(scroll_to_header_on_birth)
                .on_spawn_with_system(|In(entity), mut commands: Commands| commands.trigger_targets(CheckInspectionTargets, entity))
                .on_spawn(clone!((viewability, node_type, type_path, enum_data_option, field_type) move |world, ui_entity| {
                    // TODO: more intelligent way to get this height? waiting for node to reach "full size" is pretty cringe
                    let mut field_path_option = None;
                    let type_registry = world.resource::<AppTypeRegistry>().clone();
                    match field_type {
                        FieldType::Field(_) => {
                            if let FieldElementInput::Component { component, .. } = input {
                                let mut system_state = SystemState::<&Components>::new(world);
                                let components = system_state.get(world);
                                let type_registry = type_registry.read();
                                if let Some(info) = components.get_info(component) {
                                    if info.type_id().and_then(|type_id| type_registry.get(type_id)).is_none() {
                                        viewability.set_neq(Viewability::NotInRegistry);
                                    }
                                }
                            }
                        },
                        FieldType::Access(access) => {
                            let mut system_state = SystemState::<FieldPathCached>::new(world);
                            let mut field_path_cached = system_state.get_mut(world);
                            let mut field_path = field_path_cached.get(ui_entity);
                            field_path.0.push(access.into());
                            field_path_option = Some(field_path);
                        },
                    }
                    if let Some(mut reflect) = match input {
                        FieldElementInput::Component { owner, component } => {
                            match owner {
                                ComponentOwnerType::Entity(entity) => reflect_component(world, entity, component),
                                ComponentOwnerType::Resource => reflect_resource(world, component),
                            }
                        },
                        FieldElementInput::Asset { asset, handle } => {
                            reflect_asset(world, asset, handle)
                        }
                    } {
                        if let Some(path) = field_path_option {
                            if let Some(result) = reflect.reflect_path(&path).ok().and_then(PartialReflect::try_as_reflect) {
                                reflect = result;
                            }
                        }
                        let type_path_string = reflect.reflect_type_path().to_string();
                        type_path.set(Some(type_path_string.clone()));
                        if has_frontend(&type_path_string) {
                            node_type.set(Some(NodeType::Solo(type_path_string)));
                            // TODO: expose this as a setting ?
                            // expanded.set_neq(true);
                        } else {
                            let mut set_viewability = None;
                            match reflect.reflect_ref() {
                                ReflectRef::Struct(struct_) => {
                                    let mut fields = vec![];
                                    for i in 0..struct_.field_len() {
                                        if let Some(name) = struct_.name_at(i) {
                                            let access = Access::Field(name.to_string().into());
                                            fields.push(AccessFieldData::new(access));
                                        }
                                    }
                                    node_type.set(Some(NodeType::Multi { items: fields.into(), size_dynamic: None }));
                                    set_viewability = if struct_.field_len() == 0 { Viewability::Unit } else { Viewability::Viewable }.apply(Some);
                                },
                                ReflectRef::TupleStruct(tuple_struct) => {
                                    let mut fields = vec![];
                                    for i in 0..tuple_struct.field_len() {
                                        let access = Access::TupleIndex(i);
                                        fields.push(AccessFieldData::new(access));
                                    }
                                    node_type.set(Some(NodeType::Multi { items: fields.into(), size_dynamic: None }));
                                    set_viewability = if tuple_struct.field_len() == 0 { Viewability::Unit } else { Viewability::Viewable }.apply(Some);
                                },
                                ReflectRef::Tuple(tuple) => {
                                    let mut fields = vec![];
                                    for i in 0..tuple.field_len() {
                                        let access = Access::TupleIndex(i);
                                        fields.push(AccessFieldData::new(access));
                                    }
                                    node_type.set(Some(NodeType::Multi { items: fields.into(), size_dynamic: None }));
                                    set_viewability = Some(Viewability::Viewable);
                                },
                                ReflectRef::List(list) => {
                                    let mut fields = vec![];
                                    for i in 0..list.len() {
                                        let access = Access::ListIndex(i);
                                        fields.push(AccessFieldData::new(access));
                                    }
                                    node_type.set(Some(NodeType::Multi { items: fields.into(), size_dynamic: Some(ReflectKind::List) }));
                                    set_viewability = Some(Viewability::Viewable);
                                },
                                ReflectRef::Array(array) => {
                                    let mut fields = vec![];
                                    for i in 0..array.len() {
                                        let access = Access::ListIndex(i);
                                        fields.push(AccessFieldData::new(access));
                                    }
                                    node_type.set(Some(NodeType::Multi { items: fields.into(), size_dynamic: None }));
                                    set_viewability = Some(Viewability::Viewable);
                                },
                                ReflectRef::Set(_array) => {
                                    warn!("Set not supported yet");
                                },
                                ReflectRef::Map(_map) => {
                                    // TODO: might require adding map support to Access ?
                                    warn!("Map not supported yet");
                                },
                                ReflectRef::Enum(enum_) => {
                                    if let Some(TypeInfo::Enum(enum_info)) = enum_.get_represented_type_info() {
                                        let type_registry = type_registry.read();
                                        let mut enum_data = vec![];
                                        for variant in enum_info.variant_names() {
                                            if let Some(variant_info) = enum_info.variant(variant) {
                                                let has_default = variant_default_value(variant_info, &type_registry).is_some();
                                                enum_data.push(VariantData { variant, has_default });
                                            }
                                        }
                                        enum_data_option.set(Some(enum_data));
                                        set_viewability = Some(Viewability::Viewable);
                                        // TODO: expose this as a setting ?
                                        // expanded.set_neq(true);
                                    }
                                },
                                ReflectRef::Opaque(opaque) => {
                                    let type_path = opaque.reflect_type_path();
                                    node_type.set(Some(NodeType::Solo(type_path.to_string())));
                                    set_viewability = Some(Viewability::Opaque);
                                },
                            }
                            if let Some(v) = set_viewability {
                                viewability.set_neq(v);
                            }
                        }
                    }
                }))
                .apply(sync_tooltip_position(expected_tooltip_height))
                .on_signal_with_system(
                    map_ref! {
                        let &viewability = viewability.signal(),
                        let &hovered = hovered.signal() => {
                            (viewability, hovered)
                        }
                    },
                    clone!((field_type) move |
                        In((entity, (viewability, hovered))),
                        mut tooltip_cache: TooltipCache,
                    | {
                        if let Some(tooltip) = tooltip_cache.get(entity) {
                            if let FieldType::Field(type_path) = &field_type {
                                let text = match viewability {
                                    Viewability::NotInRegistry => {
                                        format!("`{}` is not registered in the `TypeRegistry`", ShortName(type_path))
                                    },
                                    Viewability::Opaque => {
                                        "reflect opaque".to_string()
                                    },
                                    Viewability::Unit => {
                                        "unit struct".to_string()
                                    },
                                    _ => return
                                };
                                let data = Some(TooltipData::new(entity, text));
                                let mut lock = tooltip.lock_mut();
                                if hovered {
                                    if *lock != data {
                                        *lock = data;
                                    }
                                } else if *lock == data {
                                    *lock = None;
                                }
                            }
                        }
                    })
                )
            })
            .item({
                field_header(
                    name,
                    // TODO: make this cleaner with an enum
                    if matches!(input, FieldElementInput::Asset { .. }) && matches!(field_type, FieldType::Field(_)) {
                        Some(FieldType::Field("".to_string()))
                    } else {
                        Some(field_type.clone())
                    },
                    type_path.clone(),
                    viewability.clone(),
                    hovered.clone(),
                    column_gap.clone(),
                    highlighted_color.clone(),
                    unhighlighted_color.clone(),
                    error_color.clone(),
                    type_path_color.clone(),
                )
                .apply(
                    header_wrapper(
                        hovered.clone(),
                        clone!((expanded, viewability) move |
                            In((entity, click)): In<(Entity, Pointer<Click>)>,
                            headers: Query<&HeaderData>,
                            relative_rect: RelativeRect,
                            mut maybe_scroll_to_header_root: MaybeScrollToHeaderRoot,
                            parents: Query<&Parent>
                        | {
                            if matches!(viewability.get(), Viewability::Viewable) && matches!(click.button, PointerButton::Primary) {
                                let mut i = -1;  // don't count current header
                                for ancestor in parents.iter_ancestors(entity) {
                                    if headers.contains(ancestor) {
                                        i += 1;
                                    }
                                }
                                if let Some(rect) = relative_rect.get(entity) {
                                    let partially_under = rect.min.y < rect.size().y * i as f32;
                                    if !maybe_scroll_to_header_root.scrolled(entity, !partially_under) || partially_under && expanded.get().not()
                                    {
                                        flip(&expanded)
                                    }
                                }
                            }
                        }),
                        row_gap.clone(),
                        primary_background_color,
                        secondary_background_color,
                        padding.clone(),
                        pinned.clone(),
                    )
                )
                .cursor_disableable_signal(CursorIcon::System(SystemCursorIcon::Pointer), viewability.signal().map(|viewability| !matches!(viewability, Viewability::Viewable)))
                .z_index(ZIndex(i32::MAX))
            })
            .item_signal(expanded.signal().dedupe().map_true(
                clone!((
                    row_gap,
                    border_width,
                    border_color,
                    padding,
                    highlighted_color,
                    unhighlighted_color,
                    type_path_color,
                    viewability,
                    enum_data_option,
                    hovered,
                    field_type
                ) move || {
                    let mut el = Column::<Node>::new()
                    .width(Val::Percent(100.))
                    .height(Val::Percent(100.))
                    .apply(margin_style(BoxEdge::HORIZONTAL, padding.signal()))
                    .apply(left_bordered_style(border_width.signal(), map_bool_signal(hovered.signal(), tertiary_background_color.clone(), border_color.clone()), padding.signal()));
                    let mut custom_field_option = None;
                    if let FieldType::Field(field_) = &field_type {
                        custom_field_option = frontend(field_);
                    }
                    if let Some(field) = custom_field_option {
                        el = el.item(
                            El::<Node>::new()
                            .apply(padding_style(BoxEdge::VERTICAL, row_gap.signal().map(div(2.))))
                            .apply(padding_style(BoxEdge::HORIZONTAL, padding.signal()))
                            .child(
                                field
                                .update_raw_el(|raw_el| {
                                    raw_el
                                    .insert(Accessory { target: input.into(), access_option: None})
                                })
                            )
                        );
                    } else {
                        el = el
                        .item_signal(
                            enum_data_option.signal_cloned()
                            .map_some(clone!((access_option, node_type, row_gap, padding) move |enum_data| {
                                let options = enum_data.into_iter().map(|VariantData { variant, has_default }| OptionData::new(variant, !has_default)).collect::<Vec<_>>().into();
                                let selected = Mutable::new(None);
                                let show_dropdown = Mutable::new(false);
                                let dropdown_entity = Mutable::new(None);
                                El::<Node>::new()
                                .apply(padding_style(BoxEdge::HORIZONTAL, padding.signal()))
                                .child(
                                    Dropdown::new(options)
                                    .on_click_outside(clone!((show_dropdown) move || show_dropdown.set_neq(false)))
                                    .with_show_dropdown(show_dropdown.clone())
                                    .apply(padding_style(BoxEdge::VERTICAL, row_gap.signal().map(div(2.))))
                                    .blocked_tooltip("variant has no default".to_string())
                                    .update_raw_el(clone!((access_option, selected, dropdown_entity, node_type) move |raw_el| {
                                        raw_el
                                        .insert(Accessory { target: input.into(), access_option })
                                        .with_entity(clone!((selected, node_type) move |mut entity| {
                                            dropdown_entity.set_neq(Some(entity.id()));
                                            let handler = entity.world_scope(move |world| {
                                                register_system(world, clone!((selected, node_type) move |In(reflect): In<Box<dyn PartialReflect>>| {
                                                    if let ReflectRef::Enum(enum_) = reflect.reflect_ref() {
                                                        let variant = enum_.variant_index();
                                                        let new = Some(variant);
                                                        if *selected.lock_ref() != new {
                                                            selected.set(new);
                                                            populate_enum_with_variant(enum_, variant, &node_type);
                                                        }
                                                    }
                                                }))
                                            });
                                            entity.insert(FieldListener { handler });
                                        }))
                                    }))
                                    .width(Val::Percent(60.))
                                    .selected_signal(selected.signal())
                                    .option_handler_system(clone!((node_type) move |
                                        In(i): In<usize>,
                                        accessories: Query<&Accessory>,
                                        mut field_path_cached: FieldPathCached,
                                        type_registry: Res<AppTypeRegistry>,
                                        mut commands: Commands,
                                    | {
                                        let ui_entity = dropdown_entity.get().unwrap();
                                        if let Ok(&Accessory { target, .. }) = accessories.get(ui_entity) {
                                            let field_path = field_path_cached.get(ui_entity);
                                            let type_registry = type_registry.0.clone();
                                            commands.queue(clone!((node_type) move |world: &mut World| {
                                                let f = |reflect: &mut dyn Reflect| {
                                                    if let Ok(target) = reflect.reflect_path_mut(&field_path) {
                                                        if let ReflectMut::Enum(enum_) = target.reflect_mut() {
                                                            if let Some(variant_info) = get_variant_info(enum_, i) {
                                                                if let Some(default) = variant_default_value(variant_info, &type_registry.read()) {
                                                                    populate_enum_with_variant(enum_, i, &node_type);
                                                                    let _ = target.try_apply(&default);
                                                                }
                                                            }
                                                        }
                                                    }
                                                };
                                                apply_to_accessory_target(world, target, f);
                                            }));
                                        }
                                        show_dropdown.set_neq(false);
                                    }))
                                    .into_el()
                                    .z_index(ZIndex(i32::MAX))
                                )
                            }))
                        )
                        .item_signal(
                            node_type.signal_cloned()
                            .map_some(clone!((row_gap, border_width, border_color, padding, highlighted_color, unhighlighted_color, type_path_color, viewability, padding) move |node_type| match node_type {
                                NodeType::Solo(type_path) => {
                                    let el_option = frontend(&type_path).map(TypeEraseable::type_erase);
                                    if el_option.is_some() {
                                        viewability.set_neq(Viewability::Viewable);
                                    }
                                    let is_entity_field = &type_path == "bevy_ecs::entity::Entity";
                                    el_option
                                        .map(|el| {
                                            // TODO: why do we need a wrapper even when we forgo the padding for entity fields ?
                                            let mut el = El::<Node>::new().child(el).type_erase();
                                            if !is_entity_field {
                                                el = el
                                                .apply(padding_style(BoxEdge::VERTICAL, row_gap.signal().map(div(2.))))
                                                .apply(padding_style(BoxEdge::HORIZONTAL, padding.signal()));
                                            }
                                            el
                                        })
                                },
                                NodeType::Multi { items, size_dynamic } => {
                                    Column::<Node>::new()
                                    .update_raw_el(clone!((items) move |mut raw_el| {
                                        raw_el = raw_el.insert(FieldsColumn);
                                        if let Some(reflect_kind) = size_dynamic {
                                            raw_el = raw_el.with_entity(move |mut entity| {
                                                let handler = entity.world_scope(|world| {
                                                    register_system(world, move |In(reflect): In<Box<dyn PartialReflect>>| {
                                                        if let ReflectRef::List(list) = reflect.reflect_ref() {
                                                            let cur = list.len();
                                                            let len = items.lock_ref().len();
                                                            let mut lock = items.lock_mut();
                                                            if cur > len {
                                                                for i in len..cur {
                                                                    if let Some(access) = match reflect_kind {
                                                                        ReflectKind::List => Some(Access::ListIndex(i)),
                                                                        // TODO
                                                                        // ReflectKind::Map => Some(...),
                                                                        _ => None,
                                                                    } {
                                                                        lock.push_cloned(AccessFieldData::new(access));
                                                                    }
                                                                }
                                                            } else if cur < len {
                                                                for _ in 0..(len - cur) {
                                                                    lock.pop();
                                                                }
                                                            }
                                                        }
                                                    })
                                                });
                                                entity.insert(FieldListener { handler });
                                            });
                                        }
                                        raw_el
                                    }))
                                    .items_signal_vec(
                                        items.signal_vec_cloned()
                                        .map(clone!((row_gap, border_width, border_color, padding, highlighted_color, unhighlighted_color, type_path_color) move |AccessFieldData { access, viewability }| {
                                            FieldElement::new(input, FieldType::Access(access.clone()), viewability)
                                            .row_gap_signal(row_gap.signal())
                                            .border_width_signal(border_width.signal())
                                            .border_color_signal(border_color.signal())
                                            .padding_signal(padding.signal())
                                            .highlighted_color_signal(highlighted_color.signal())
                                            .unhighlighted_color_signal(unhighlighted_color.signal())
                                            .type_path_color_signal(type_path_color.signal())
                                        }))
                                    )
                                    .type_erase()
                                    .apply(Some)
                                },
                            }))
                            .map(Option::flatten)
                            .map(clone!((access_option, node_type) move |el_option| {
                                el_option
                                .map(clone!((access_option, node_type) move |el| {
                                    el.update_raw_el(move |mut raw_el| {
                                        match node_type.get_cloned() {
                                            Some(NodeType::Solo(_)) => {
                                                raw_el = raw_el.on_spawn_with_system(clone!((access_option) move |In(field_parent_entity), childrens: Query<&Children>, mut commands: Commands| {
                                                    let Some(&field_entity) = i_born(field_parent_entity, &childrens, 0) else { return };
                                                    if let Some(mut entity_commands) = commands.get_entity(field_entity) {
                                                        entity_commands.try_insert(Accessory { target: input.into(), access_option: access_option.clone() });
                                                    }
                                                }));
                                            },
                                            Some(NodeType::Multi { .. }) => {
                                                raw_el = raw_el.insert(Accessory { target: input.into(), access_option: access_option.clone() });
                                            },
                                            _ => ()
                                        }
                                        raw_el
                                    })
                                }))
                            }))
                        );
                    }
                    el
                })
            ))
            ;
        Self {
            el,
            row_gap,
            column_gap,
            border_width,
            border_color,
            padding,
            highlighted_color,
            unhighlighted_color,
            type_path_color,
            expanded,
        }
    }

    impl_syncers! {
        row_gap: f32,
        column_gap: f32,
        border_width: f32,
        border_color: Color,
        padding: f32,
        highlighted_color: Color,
        unhighlighted_color: Color,
        type_path_color: Color,
        expanded: bool,
    }
}

#[allow(clippy::type_complexity)]
#[rustfmt::skip]
static FRONTENDS: Lazy<
    RwLock<HashMap<&'static str, Box<dyn Fn() -> AlignabilityFacade + Send + Sync + 'static>>>,
> = Lazy::new(|| {
    HashMap::from([
        ("bool", Box::new(|| bool_field().type_erase()) as Box<_>),
        ("isize", Box::new(|| numeric_field::<isize>().apply(basic_numeric_field_width::<isize>).type_erase()) as Box<_>),
        ("i8", Box::new(|| numeric_field::<i8>().apply(basic_numeric_field_width::<i8>).type_erase()) as Box<_>),
        ("i16", Box::new(|| numeric_field::<i16>().apply(basic_numeric_field_width::<i16>).type_erase()) as Box<_>),
        ("i32", Box::new(|| numeric_field::<i32>().apply(basic_numeric_field_width::<i32>).type_erase()) as Box<_>),
        ("i64", Box::new(|| numeric_field::<i64>().apply(basic_numeric_field_width::<i64>).type_erase()) as Box<_>),
        ("i128", Box::new(|| numeric_field::<i128>().apply(basic_numeric_field_width::<i128>).type_erase()) as Box<_>),
        ("usize", Box::new(|| numeric_field::<usize>().apply(basic_numeric_field_width::<usize>).type_erase()) as Box<_>),
        ("u8", Box::new(|| numeric_field::<u8>().apply(basic_numeric_field_width::<u8>).type_erase()) as Box<_>),
        ("u16", Box::new(|| numeric_field::<u16>().apply(basic_numeric_field_width::<u16>).type_erase()) as Box<_>),
        ("u32", Box::new(|| numeric_field::<u32>().apply(basic_numeric_field_width::<u32>).type_erase()) as Box<_>),
        ("u64", Box::new(|| numeric_field::<u64>().apply(basic_numeric_field_width::<u64>).type_erase()) as Box<_>),
        ("u128", Box::new(|| numeric_field::<u128>().apply(basic_numeric_field_width::<u128>).type_erase()) as Box<_>),
        ("f32", Box::new(|| numeric_field::<f32>().apply(basic_numeric_field_width::<f32>).type_erase()) as Box<_>),
        ("f64", Box::new(|| numeric_field::<f64>().apply(basic_numeric_field_width::<f64>).type_erase()) as Box<_>),
        ("glam::Vec2", Box::new(|| numeric_vec_field::<f32>(&["x", "y"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::Vec3", Box::new(|| numeric_vec_field::<f32>(&["x", "y", "z"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::Vec3A", Box::new(|| numeric_vec_field::<f32>(&["x", "y", "z"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::Vec4", Box::new(|| numeric_vec_field::<f32>(&["x", "y", "z", "w"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::UVec2", Box::new(|| numeric_vec_field::<u32>(&["x", "y"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::UVec3", Box::new(|| numeric_vec_field::<u32>(&["x", "y", "z"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::UVec4", Box::new(|| numeric_vec_field::<u32>(&["x", "y", "z", "w"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::IVec2", Box::new(|| numeric_vec_field::<i32>(&["x", "y"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::IVec3", Box::new(|| numeric_vec_field::<i32>(&["x", "y", "z"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::IVec4", Box::new(|| numeric_vec_field::<i32>(&["x", "y", "z", "w"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::DVec2", Box::new(|| numeric_vec_field::<f64>(&["x", "y"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::DVec3", Box::new(|| numeric_vec_field::<f64>(&["x", "y", "z"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::DVec4", Box::new(|| numeric_vec_field::<f64>(&["x", "y", "z", "w"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("glam::Mat2", Box::new(|| numeric_mat_field::<f32>(&["x", "y"]).type_erase()) as Box<_>),
        ("glam::Mat3", Box::new(|| numeric_mat_field::<f32>(&["x", "y", "z"]).type_erase()) as Box<_>),
        ("glam::Mat3A", Box::new(|| numeric_mat_field::<f32>(&["x", "y", "z"]).type_erase()) as Box<_>),
        ("glam::Mat4", Box::new(|| numeric_mat_field::<f32>(&["x", "y", "z", "w"]).type_erase()) as Box<_>),
        ("glam::DMat2", Box::new(|| numeric_mat_field::<f64>(&["x", "y"]).type_erase()) as Box<_>),
        ("glam::DMat3A", Box::new(|| numeric_mat_field::<f64>(&["x", "y", "z"]).type_erase()) as Box<_>),
        ("glam::DMat4", Box::new(|| numeric_mat_field::<f64>(&["x", "y", "z", "w"]).type_erase()) as Box<_>),
        ("glam::BVec2", Box::new(|| bool_vec_field(&["x", "y"]).type_erase()) as Box<_>),
        ("glam::BVec3", Box::new(|| bool_vec_field(&["x", "y", "z"]).type_erase()) as Box<_>),
        ("glam::BVec4", Box::new(|| bool_vec_field(&["x", "y", "z", "w"]).type_erase()) as Box<_>),
        ("glam::Quat", Box::new(|| numeric_vec_field::<f32>(&["x", "y", "z", "w"], None::<MutableSignal<f32>>, None).type_erase()) as Box<_>),
        ("alloc::string::String", Box::new(|| string_field::<String>().type_erase()) as Box<_>),
        ("alloc::borrow::Cow<str>", Box::new(|| string_field::<Cow<str>>().type_erase()) as Box<_>),
        ("bevy_ecs::entity::Entity", Box::new(|| entity_field().type_erase()) as Box<_>),
    ])
    .apply(RwLock::new)
});

#[allow(clippy::type_complexity)]
static CUSTOM_FRONTENDS: Lazy<
    RwLock<HashMap<&'static str, Box<dyn Fn() -> AlignabilityFacade + Send + Sync + 'static>>>,
> = Lazy::new(default);

pub fn register_frontend<T: Into<AlignabilityFacade> + 'static>(
    type_path: &'static str,
    element_function: impl Fn() -> T + Send + Sync + 'static,
) {
    CUSTOM_FRONTENDS
        .write()
        .unwrap()
        .insert(type_path, Box::new(move || element_function().into()));
}

pub fn append_access<E: RawElWrapper>(access: Access<'static>) -> impl FnOnce(E) -> E {
    move |el| {
        el.update_raw_el(move |raw_el| {
            raw_el.on_spawn_with_system(
                move |In(entity),
                      parents: Query<&Parent>,
                      accessories: Query<&Accessory>,
                      mut commands: Commands| {
                    for ancestor in parents.iter_ancestors(entity) {
                        if let Ok(accessory) = accessories.get(ancestor).cloned() {
                            if let Some(mut entity) = commands.get_entity(entity) {
                                entity.try_insert(Accessory {
                                    access_option: Some(access.clone()),
                                    ..accessory
                                });
                            }
                            return;
                        }
                    }
                },
            )
        })
    }
}

fn max_width_option_signal(
    widths: impl SignalVec<Item = Mutable<f32>>,
) -> impl Signal<Item = Option<f32>> {
    widths
        .map_signal(|width| width.signal())
        .to_signal_map(|widths| widths.iter().max_by(|a, b| a.total_cmp(b)).copied())
}

pub fn numeric_vec_field<T: NumericFieldable>(
    fields: &'static [&str],
    global_width_receiver_option: Option<impl Signal<Item = f32> + Send + Sync + 'static>,
    width_sender_option: Option<Mutable<f32>>,
) -> impl Element + Sizeable
where
    <<T as NumericFieldable>::T as FromStr>::Err: Debug,
{
    let column_gap = GLOBAL_COLUMN_GAP.clone();
    let widths: MutableVec<Mutable<f32>> = fields
        .iter()
        .map(|_| Mutable::new(0.))
        .collect::<Vec<_>>()
        .into();
    let local_width = max_width_option_signal(widths.signal_vec_cloned())
        .map(Option::unwrap_or_default)
        .broadcast();
    let global_width = if let Some(global_width_receiver) = global_width_receiver_option {
        global_width_receiver.apply(boxed_sync)
    } else {
        local_width.signal().apply(boxed_sync)
    }
    .broadcast();
    let mut tasks = vec![];
    if let Some(width_receiver) = width_sender_option {
        tasks.push(sync_neq(local_width.signal(), width_receiver).apply(spawn));
    }
    let widths = widths.lock_ref().iter().cloned().collect::<Vec<_>>();
    Row::<Node>::new()
        .update_raw_el(|raw_el| raw_el.hold_tasks(tasks))
        .apply(row_style(column_gap.signal()))
        .items(fields.iter().zip(widths).map(move |(field, width)| {
            numeric_field::<T>()
                .width_signal(global_width.signal().map(Val::Px))
                .apply(numeric_field_width_reporter::<T>(width))
                .apply(append_access(Access::Field(Cow::from(*field))))
        }))
}

pub fn numeric_mat_field<T: NumericFieldable>(fields: &'static [&str]) -> impl Element
where
    <<T as NumericFieldable>::T as FromStr>::Err: Debug,
{
    let row_gap = GLOBAL_ROW_GAP.clone();
    let widths: MutableVec<Mutable<f32>> = fields
        .iter()
        .map(|_| Mutable::new(0.))
        .collect::<Vec<_>>()
        .into();
    let global_width = max_width_option_signal(widths.signal_vec_cloned())
        .map(|width_option| width_option.unwrap_or(INITIAL_NUMERIC_FIELD_INPUT_WIDTH))
        .broadcast();
    let widths = widths.lock_ref().iter().cloned().collect::<Vec<_>>();
    Column::<Node>::new()
        .apply(column_style(row_gap.signal()))
        .items(
            fields
                .iter()
                .zip(widths)
                .map(move |(field, width_receiver)| {
                    numeric_vec_field::<T>(
                        &["x", "y", "z", "w"][0..fields.len()],
                        Some(global_width.signal()),
                        Some(width_receiver),
                    )
                    .apply(append_access(Access::Field(Cow::from(format!(
                        "{field}_axis"
                    )))))
                }),
        )
}

fn bool_vec_field(fields: &'static [&str]) -> impl Element {
    let column_gap = GLOBAL_COLUMN_GAP.clone();
    Row::<Node>::new()
        .apply(row_style(column_gap.signal()))
        .items(
            fields.iter().map(move |field| {
                bool_field().apply(append_access(Access::Field(Cow::from(*field))))
            }),
        )
}

pub fn has_frontend(type_path: &str) -> bool {
    FRONTENDS.read().unwrap().contains_key(type_path)
        || CUSTOM_FRONTENDS.read().unwrap().contains_key(type_path)
}

pub fn frontend(type_path: &str) -> Option<impl Element> {
    CUSTOM_FRONTENDS
        .read()
        .unwrap()
        .get(type_path)
        .map(|f| f())
        .or_else(|| FRONTENDS.read().unwrap().get(type_path).map(|f| f()))
}

#[derive(SystemParam)]
pub struct FieldPath<'w, 's> {
    accessories: Query<'w, 's, &'static Accessory>,
    parents: Query<'w, 's, &'static Parent>,
    entity_roots: Query<'w, 's, &'static EntityRoot>,
}

impl<'w, 's> FieldPath<'w, 's> {
    pub fn get(&self, entity: Entity) -> ParsedPath {
        let mut path = vec![];
        for ancestor in [entity]
            .into_iter()
            .chain(self.parents.iter_ancestors(entity))
        {
            if let Ok(Accessory {
                access_option: Some(access),
                ..
            }) = self.accessories.get(ancestor)
            {
                path.push(access.clone());
            }
            if self.entity_roots.contains(ancestor) {
                break;
            }
        }
        path.reverse();
        ParsedPath::from(path)
    }
}

#[derive(SystemParam)]
pub struct FieldPathCached<'w, 's> {
    field_path: FieldPath<'w, 's>,
    field_path_cache: ResMut<'w, FieldPathCache>,
}

impl<'w, 's> FieldPathCached<'w, 's> {
    pub fn get(&mut self, entity: Entity) -> ParsedPath {
        if let Some(field_path) = self.field_path_cache.0.get(&entity) {
            field_path.clone()
        } else {
            let field_path = self.field_path.get(entity);
            self.field_path_cache.0.insert(entity, field_path.clone());
            field_path
        }
    }
}

// adapted from Quill https://github.com/viridia/quill/blob/cecbc35426a095f56bad1f12df546f5a79dece32/crates/bevy_quill_obsidian_inspect/src/inspectors/enum.rs#L171
pub fn variant_default_value(
    variant: &VariantInfo,
    registry: &TypeRegistry,
) -> Option<DynamicEnum> {
    match variant {
        VariantInfo::Struct(struct_) => {
            let mut dynamic_struct = DynamicStruct::default();
            for field in struct_.iter() {
                if let Some(reflect_default) =
                    registry.get_type_data::<ReflectDefault>(field.type_id())
                {
                    dynamic_struct.insert_boxed(
                        field.name(),
                        reflect_default.default().into_partial_reflect(),
                    );
                } else {
                    return None;
                }
            }
            Some(DynamicEnum::new(variant.name(), dynamic_struct))
        }
        VariantInfo::Tuple(tuple) => {
            let mut dynamic_tuple = DynamicTuple::default();
            for field in tuple.iter() {
                if let Some(reflect_default) =
                    registry.get_type_data::<ReflectDefault>(field.type_id())
                {
                    dynamic_tuple.insert_boxed(reflect_default.default().into_partial_reflect());
                } else {
                    return None;
                }
            }
            Some(DynamicEnum::new(variant.name(), dynamic_tuple))
        }
        VariantInfo::Unit(_) => Some(DynamicEnum::new(variant.name(), DynamicVariant::Unit)),
    }
}

#[derive(SystemParam)]
pub struct TargetField<'w, 's> {
    accessories: Query<'w, 's, &'static Accessory>,
    field_path_cached: FieldPathCached<'w, 's>,
    commands: Commands<'w, 's>,
}

impl<'w, 's> TargetField<'w, 's> {
    pub fn update(&mut self, entity: Entity, value: Box<dyn PartialReflect>) {
        if let Ok(&Accessory { target, .. }) = self.accessories.get(entity) {
            let field_path = self.field_path_cached.get(entity);
            self.commands.queue(move |world: &mut World| {
                let f = |reflect: &mut dyn Reflect| {
                    if let Ok(target) = reflect.reflect_path_mut(&field_path) {
                        let _ = target.try_apply(&*value);
                    }
                };
                apply_to_accessory_target(world, target, f);
            });
        }
    }
}

fn bool_field() -> impl Element {
    let checked: Mutable<bool> = Mutable::new(false);
    Checkbox::new()
        .checked_signal(checked.signal())
        .on_click_with_system(clone!((checked) move |
            In((ui_entity, click)): In<(Entity, Pointer<Click>)>,
            mut field: TargetField,
        | {
            if matches!(click.button, PointerButton::Primary) {
                field.update(ui_entity, (!checked.get()).clone_value());
            }
        }))
        .update_raw_el(|raw_el| {
            raw_el.with_entity(|mut entity| {
                let handler = entity.world_scope(|world| {
                    register_system(world, move |In(reflect): In<Box<dyn PartialReflect>>| {
                        if let Ok(cur) = reflect.try_downcast::<bool>() {
                            checked.set_neq(*cur);
                        }
                    })
                });
                entity.insert(FieldListener { handler });
            })
        })
}

pub fn entity_field() -> impl Element {
    let font_size = GLOBAL_FONT_SIZE.clone();
    let row_gap = GLOBAL_ROW_GAP.clone();
    let column_gap = GLOBAL_COLUMN_GAP.clone();
    let padding = GLOBAL_PADDING.clone();
    let border_width = GLOBAL_BORDER_WIDTH.clone();
    let primary_background_color = GLOBAL_PRIMARY_BACKGROUND_COLOR.clone();
    let secondary_background_color = GLOBAL_SECONDARY_BACKGROUND_COLOR.clone();
    let highlighted_color = GLOBAL_HIGHLIGHTED_COLOR.clone();
    let unhighlighted_color = GLOBAL_UNHIGHLIGHTED_COLOR.clone();
    let border_color = GLOBAL_BORDER_COLOR.clone();
    let entity_holder = Mutable::new(None);
    let entity_data = EntityData::default();
    let name = entity_data.name.clone();
    El::<Node>::new()
        .update_raw_el(clone!((name, entity_holder) move |raw_el| {
            raw_el
            .on_signal_with_system(
                entity_holder.signal(),
                move |In((_, entity_option)), debug_names: Query<NameOrEntity>| {
                    if let Some(entity) = entity_option {
                        if let Ok(Some(debug_name)) = debug_names.get(entity).map(|name| name.name) {
                            name.set(Some(debug_name.to_string()));
                        }
                    }
                }
            )
            .with_entity(move |mut entity| {
                let handler = entity.world_scope(|world| {
                    register_system(world, move |In(reflect): In<Box<dyn PartialReflect>>| {
                        if let Ok(cur) = reflect.try_downcast::<Entity>() {
                            entity_holder.set_neq(Some(*cur));
                        }
                    })
                });
                entity.insert(FieldListener { handler });
            })
        }))
        .child_signal(entity_holder.signal().map_some(move |entity| {
            MultiFieldElement::new(MultiFieldData::Entity {
                id: entity,
                data: entity_data.clone(),
            })
            .show_name()
            .font_size_signal(font_size.signal())
            .row_gap_signal(row_gap.signal())
            .column_gap_signal(column_gap.signal())
            .primary_background_color_signal(primary_background_color.signal())
            .secondary_background_color_signal(secondary_background_color.signal())
            .border_color_signal(border_color.signal())
            .border_width_signal(border_width.signal())
            .padding_signal(padding.signal())
            .highlighted_color_signal(highlighted_color.signal())
            .unhighlighted_color_signal(unhighlighted_color.signal())
        }))
}

#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct TextInputField<T, F> {
    el: TextInput,
    initial: T,
    formatter: F,
    highlight: Mutable<bool>, // wrap in option when don't want to provide impl_syncer
    border_color_option: Option<SyncBoxSignal<'static, Option<Color>>>,
    text_color_option: Option<SyncBoxSignal<'static, Option<Color>>>,
    focused: Option<Mutable<bool>>,
    value: Option<Mutable<T>>,
    with_text_signal: Vec<Box<dyn FnMut(TextInput, BoxSignal<'static, String>) -> TextInput>>,
}

impl<T, F> TextInputField<T, F> {
    pub fn new(initial: T, formatter: F) -> Self {
        Self {
            el: TextInput::new(),
            initial,
            formatter,
            highlight: Mutable::new(false),
            border_color_option: None,
            text_color_option: None,
            focused: None,
            value: None,
            with_text_signal: vec![],
        }
    }

    pub fn with_value(mut self, value: Mutable<T>) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_focused(mut self, focused: Mutable<bool>) -> Self {
        self.focused = Some(focused);
        self
    }

    pub fn with_highlight(mut self, highlight: Mutable<bool>) -> Self {
        self.highlight = highlight;
        self
    }

    pub fn with_border_color_option(
        mut self,
        border_color_option: impl Signal<Item = Option<Color>> + Send + Sync + 'static,
    ) -> Self {
        self.border_color_option = Some(boxed_sync(border_color_option));
        self
    }

    pub fn with_text_color_option(
        mut self,
        text_color_option: impl Signal<Item = Option<Color>> + Send + Sync + 'static,
    ) -> Self {
        self.text_color_option = Some(boxed_sync(text_color_option));
        self
    }

    pub fn with_text_signal(
        mut self,
        f: Box<dyn FnMut(TextInput, BoxSignal<'static, String>) -> TextInput>,
    ) -> Self {
        self.with_text_signal.push(f);
        self
    }
}

impl<
        T: Send + Sync + PartialEq + Reflect + Clone + Debug,
        F: Fn(T) -> String + Send + Sync + Clone + 'static,
    > GlobalEventAware for TextInputField<T, F>
{
}
impl<
        T: Send + Sync + PartialEq + Reflect + Clone + Debug,
        F: Fn(T) -> String + Send + Sync + Clone + 'static,
    > PointerEventAware for TextInputField<T, F>
{
}
impl<
        T: Send + Sync + PartialEq + Reflect + Clone + Debug,
        F: Fn(T) -> String + Send + Sync + Clone + 'static,
    > CursorOnHoverable for TextInputField<T, F>
{
}
impl<
        T: Send + Sync + PartialEq + Reflect + Clone + Debug,
        F: Fn(T) -> String + Send + Sync + Clone + 'static,
    > Sizeable for TextInputField<T, F>
{
}

pub fn base_text_attrs() -> TextAttrs {
    TextAttrs::new()
        .family(FamilyOwned::new(Family::Name("Fira Mono")))
        .weight(FontWeight::MEDIUM)
}

pub fn text_input_height_signal(
    font_size: impl Signal<Item = f32> + Send + 'static,
    border_width: impl Signal<Item = f32> + Send + 'static,
    padding: impl Signal<Item = f32> + Send + 'static,
) -> impl Signal<Item = f32> + Send + 'static {
    map_ref! {
        let font_size = font_size,
        let border_width = border_width,
        let padding = padding => {
            font_size + border_width * 4. + padding + 3.  // TODO: where did this 3. come from ?
        }
    }
}

pub fn base_text_input<T, F>(
    value: Mutable<T>,
    formatter: F,
    hovered: Mutable<bool>,
    focused: Mutable<bool>,
    text_input_option: Option<TextInput>,
) -> TextInput
where
    T: Send + Sync + PartialEq + Reflect + Clone + Debug,
    F: Fn(T) -> String + Send + Sync + 'static,
{
    let background_color = GLOBAL_PRIMARY_BACKGROUND_COLOR.clone();
    let font_size = GLOBAL_FONT_SIZE.clone();
    let unhighlighted_color = GLOBAL_UNHIGHLIGHTED_COLOR.clone();
    let border_radius = GLOBAL_BORDER_RADIUS.clone();
    let border_width = GLOBAL_BORDER_WIDTH.clone();
    let border_color = GLOBAL_BORDER_COLOR.clone();
    let padding = GLOBAL_PADDING.clone();
    #[allow(clippy::unwrap_or_default)]
    text_input_option
        .unwrap_or_else(TextInput::new)
        // TODO: height_signal alone is not working for some reason ??
        .height(Val::Px(
            font_size.get() + border_width.get() * 4. + padding.get() + 3.,
        ))
        .height_signal(
            text_input_height_signal(font_size.signal(), border_width.signal(), padding.signal())
                .map(Val::Px),
        )
        .hovered_sync(hovered.clone())
        .text_signal(value.signal_cloned().map(formatter))
        .focus_signal(focused.signal().dedupe())
        .focused_sync(focused.clone())
        .text_position(CosmicTextAlign::Center { padding: 0 })
        .on_click_outside_with_system(
            |In((entity, _)),
             focused_option: Option<Res<FocusedTextInput>>,
             mut commands: Commands| {
                if focused_option.as_deref().map(Deref::deref).copied() == Some(entity) {
                    commands.remove_resource::<FocusedTextInput>();
                }
            },
        )
        .font_size_signal(font_size.signal())
        .cursor_color_signal(unhighlighted_color.signal().map(CursorColor))
        .fill_color_signal(background_color.signal().map(CosmicBackgroundColor))
        .selection_color_signal(border_color.signal().map(SelectionColor))
        .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
        .apply(border_width_style(BoxEdge::ALL, border_width.signal()))
}

impl<
        T: Send + Sync + PartialEq + Reflect + Clone + Debug,
        F: Fn(T) -> String + Send + Sync + Clone + 'static,
    > ElementWrapper for TextInputField<T, F>
{
    type EL = TextInput;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let highlighted_color = GLOBAL_HIGHLIGHTED_COLOR.clone();
        let unhighlighted_color = GLOBAL_UNHIGHLIGHTED_COLOR.clone();
        let border_color = GLOBAL_BORDER_COLOR.clone();
        let value = self.value.unwrap_or_else(|| Mutable::new(self.initial));
        let hovered = Mutable::new(false);
        let focused = self.focused.unwrap_or_else(|| Mutable::new(false));
        let highlight = self.highlight;
        let text = value
            .signal_cloned()
            .map(self.formatter.clone())
            .broadcast();
        base_text_input(
            value.clone(),
            self.formatter,
            hovered.clone(),
            focused.clone(),
            Some(self.el),
        )
        .update_raw_el(|raw_el| {
            raw_el.with_entity(move |mut entity| {
                let handler = entity.world_scope(|world| {
                    register_system(world, move |In(reflect): In<Box<dyn PartialReflect>>| {
                        match reflect.try_downcast::<T>() {
                            Ok(cur) => value.set_neq(*cur),
                            Err(e) => error!(
                                "Failed to downcast value to type {:?}: {:?}",
                                std::any::TypeId::of::<T>(),
                                e
                            ),
                        }
                    })
                });
                entity.insert(FieldListener { handler });
            })
        })
        .attrs(base_text_attrs().color_signal({
            let text_color_option = signal::option(self.text_color_option)
                .map(Option::flatten)
                .broadcast();
            clone!((focused, highlighted_color, unhighlighted_color) map_ref! {
                let &text_color_set = text_color_option.signal_ref(Option::is_some),
                let &focused = focused.signal().dedupe(),
                let &highlight = highlight.signal() => {
                    if text_color_set {
                        text_color_option.signal().apply(boxed_sync)
                    } else if focused || highlight {
                        highlighted_color.signal().map(Some).apply(boxed_sync)
                    } else {
                        unhighlighted_color.signal().map(Some).apply(boxed_sync)
                    }
                }
            })
            .flatten()
            .dedupe()
        }))
        .apply(border_color_style({
            let border_color_option = signal::option(self.border_color_option)
                .map(Option::flatten)
                .broadcast();
            clone!((hovered, focused, highlighted_color, unhighlighted_color) map_ref! {
                let &border_color_set = border_color_option.signal_ref(Option::is_some),
                let &hovered = hovered.signal(),
                let &focused = focused.signal(),
                let &highlight = highlight.signal() => {
                    if border_color_set {
                        border_color_option.signal().apply(boxed_sync)
                    } else if focused || highlight {
                        highlighted_color.signal().map(Some).apply(boxed_sync)
                    } else if hovered {
                        unhighlighted_color.signal().map(Some).apply(boxed_sync)
                    } else {
                        border_color.signal().map(Some).apply(boxed_sync)
                    }
                }
            })
            .flatten()
        }))
        .apply(|mut el| {
            for mut f in self.with_text_signal {
                el = f(el, text.signal_cloned().boxed());
            }
            el
        })
    }
}

pub trait NumericFieldable: 'static {
    type T: Default
        + PartialEq
        + Reflect
        + Copy
        + std::ops::Add<Self::T, Output = Self::T>
        + std::ops::Sub<Self::T, Output = Self::T>
        + std::ops::Mul<Self::T, Output = Self::T>
        + Display
        + num::Bounded
        + PartialOrd
        + std::str::FromStr
        + TypePath
        + Debug;
    const IS_INTEGRAL: bool;
    const STEP: Self::T;

    fn from_f32(x: f32) -> Self::T;
}

macro_rules! impl_numeric_fieldable {
    ($type:ty, $step:expr, $is_integral:expr) => {
        impl NumericFieldable for $type {
            type T = $type;
            const IS_INTEGRAL: bool = $is_integral;
            const STEP: $type = $step;

            fn from_f32(x: f32) -> Self::T {
                x as Self::T
            }
        }
    };
}

impl_numeric_fieldable!(isize, 1, true);
impl_numeric_fieldable!(i8, 1, true);
impl_numeric_fieldable!(i16, 1, true);
impl_numeric_fieldable!(i32, 1, true);
impl_numeric_fieldable!(i64, 1, true);
impl_numeric_fieldable!(i128, 1, true);
impl_numeric_fieldable!(usize, 1, true);
impl_numeric_fieldable!(u8, 1, true);
impl_numeric_fieldable!(u16, 1, true);
impl_numeric_fieldable!(u32, 1, true);
impl_numeric_fieldable!(u64, 1, true);
impl_numeric_fieldable!(u128, 1, true);
impl_numeric_fieldable!(f32, 0.1, false);
impl_numeric_fieldable!(f64, 0.1, false);
// impl_numeric_fieldable!(std::num::NonZeroU8, 1);  // TODO

const INITIAL_NUMERIC_FIELD_INPUT_WIDTH: f32 = 35.;

const INPUT_WIDTH_PER_CHAR: f32 = 10.; // TODO: this should be tied to the font size
const NUMERIC_FIELD_GROW_THRESHOLD: usize = 2;

fn numeric_field_width(text_signal: impl Signal<Item = String>) -> impl Signal<Item = f32> {
    text_signal.map(|text| text.len()).map(|len| {
        INITIAL_NUMERIC_FIELD_INPUT_WIDTH
            + if len > NUMERIC_FIELD_GROW_THRESHOLD {
                (len - NUMERIC_FIELD_GROW_THRESHOLD) as f32 * INPUT_WIDTH_PER_CHAR
            } else {
                0.
            }
    })
}

#[allow(clippy::type_complexity)]
fn basic_numeric_field_width<T: NumericFieldable>(
    el: TextInputField<T::T, fn(T::T) -> String>,
) -> TextInputField<T::T, fn(T::T) -> String> {
    el.with_text_signal(Box::new(|self_, text_signal| {
        self_.width_signal(numeric_field_width(text_signal).map(Val::Px))
    }))
}

#[allow(clippy::type_complexity)]
fn numeric_field_width_reporter<T: NumericFieldable>(
    width: Mutable<f32>,
) -> impl FnOnce(TextInputField<T::T, fn(T::T) -> String>) -> TextInputField<T::T, fn(T::T) -> String>
{
    move |el| {
        el.with_text_signal(Box::new(move |self_, text_signal| {
            let task = sync_neq(numeric_field_width(text_signal), width.clone()).apply(spawn);
            self_.update_raw_el(|raw_el| raw_el.hold_tasks([task]))
        }))
    }
}

fn basic_numeric_formatter<T: NumericFieldable>() -> fn(T::T) -> String {
    |x| format!("{:.1}", x)
}

#[derive(Component)]
struct DragInitial<T: NumericFieldable>(T::T);

#[allow(clippy::type_complexity)]
pub fn numeric_field<T: NumericFieldable>() -> TextInputField<T::T, fn(T::T) -> String>
where
    <<T as NumericFieldable>::T as FromStr>::Err: Debug,
{
    let dragging = Mutable::new(false);
    let parse_failed = Mutable::new(None);
    let highlight = Mutable::new(false);
    let focused = Mutable::new(false);
    let value = Mutable::new(T::T::default());
    let error_color = GLOBAL_ERROR_COLOR.clone();
    let padding = GLOBAL_PADDING.clone();
    let parse_failure_color = parse_failed
        .signal_cloned()
        .map_some(move |_| error_color.signal())
        .map(signal::option)
        .flatten()
        .dedupe()
        .broadcast();
    let hovered = Mutable::new(false);
    let font_size = GLOBAL_FONT_SIZE.clone();
    let border_width = GLOBAL_BORDER_WIDTH.clone();
    let expected_tooltip_height = font_size.get() + padding.get() + border_width.get() * 2. + 3.; // TODO: where did this 3. come from ?
                                                                                                  // TODO: float formatting should be configurable
    let mut el = TextInputField::new(T::T::default(), basic_numeric_formatter::<T>())
        .hovered_sync(hovered.clone())
        .with_value(value.clone())
        .with_focused(focused.clone())
        .with_highlight(highlight.clone())
        .with_border_color_option(parse_failure_color.signal())
        .with_text_color_option(parse_failure_color.signal())
        // TODO: without this initial static value, width snaps from 100% due to signal runtime lag
        .width(Val::Px(INITIAL_NUMERIC_FIELD_INPUT_WIDTH))
        .update_raw_el(clone!((value, dragging) move |raw_el| {
            raw_el
            .insert(TextInputFocusOnDownDisabled)
            .on_event_with_system_stop_propagation::<Pointer<DragStart>, _>(clone!((highlight, dragging, value) move |In((entity, drag_start)): In<(Entity, Pointer<DragStart>)>, mut commands: Commands| {
                if matches!(drag_start.button, PointerButton::Primary) {
                    commands.insert_resource(CursorOnHoverDisabled);
                    commands.insert_resource(UpdateHoverStatesDisabled);
                    if let Some(mut entity) = commands.get_entity(entity) {
                        entity.try_insert(DragInitial::<T>(value.get()));
                    }
                    highlight.set_neq(true);
                    // TODO: dragstart on web is triggering on down
                    #[cfg(not(target_arch = "wasm32"))]
                    dragging.set_neq(true);
                }
            }))
            .on_event_with_system_stop_propagation::<Pointer<DragEnd>, _>(clone!((dragging) move |In((entity, drag_end)): In<(Entity, Pointer<DragEnd>)>, mut commands: Commands| {
                if matches!(drag_end.button, PointerButton::Primary) {
                    commands.remove_resource::<CursorOnHoverDisabled>();
                    commands.remove_resource::<UpdateHoverStatesDisabled>();
                    if let Some(mut entity) = commands.get_entity(entity) {
                        entity.remove::<DragInitial<T>>();
                    }
                    highlight.set_neq(false);
                    dragging.set_neq(false);
                }
            }))
            .on_event_with_system_stop_propagation::<Pointer<Drag>, _>(move |
                In((ui_entity, drag)): In<(Entity, Pointer<Drag>)>,
                drag_initials: Query<&DragInitial<T>>,
                mut field: TargetField
            | {
                if matches!(drag.button, PointerButton::Primary) {
                    // TODO: dragstart on web is triggering on down
                    #[cfg(target_arch = "wasm32")]
                    dragging.set_neq(true);
                    if let Ok(&DragInitial(initial)) = drag_initials.get(ui_entity) {
                        let cur = value.get();
                        let new = if !T::IS_INTEGRAL {
                            // TODO: this seems to do nothing around the max values ?
                            initial + T::from_f32(drag.distance.x) * T::STEP
                        } else {
                            // TODO: wasn't able to figure out how to integrate the distance into integral values :'(
                            if drag.delta.x > 0. {
                                if cur <= T::T::max_value() - T::STEP {
                                    cur + T::STEP
                                } else {
                                    return
                                }
                            } else if drag.delta.x < 0. {
                                if cur >= T::T::min_value() + T::STEP {
                                    cur - T::STEP
                                } else {
                                    return
                                }
                            } else {
                                return
                            }
                        };
                        field.update(ui_entity, new.clone_value());
                    }
                }
            })
        }))
        .cursor_signal(focused.signal().map_bool(|| SystemCursorIcon::Text, || SystemCursorIcon::EwResize).map(CursorIcon::System))
        .on_click(move || {
            if !dragging.get() {
                focused.set_neq(true);
            }
        });
    el.el = el
        .el
        .update_raw_el(clone!((parse_failed) move |raw_el| {
            raw_el
            .apply(sync_tooltip_position(expected_tooltip_height))
            .on_signal_with_system(
                map_ref! {
                    let error_option = parse_failed.signal_cloned(),
                    let &hovered = hovered.signal() => {
                        if hovered {
                            error_option.clone()
                        } else {
                            None
                        }
                    }
                },
                |In((entity, tooltip_option)), mut tooltip_cache: TooltipCache| {
                    if let Some(tooltip) = tooltip_cache.get(entity) {
                        if let Some(text) = tooltip_option {
                            let data = Some(TooltipData::new(entity, text));
                            tooltip.set_neq(data);
                        } else {
                            let remove_tooltip = if let Some(tooltip) = &*tooltip.lock_ref() {
                                tooltip.owner == entity
                            } else {
                                false
                            };
                            if remove_tooltip {
                                tooltip.set(None);
                            }
                        }
                    }
                }
            )
        }))
        .mode(CosmicWrap::InfiniteLine)
        .max_lines(MaxLines(1))
        .scroll_disabled()
        // TODO: this does not seem to work ... switch back to ::center once that works
        .text_position_signal(padding.signal().map(|padding| CosmicTextAlign::Left {
            padding: padding.round() as i32,
        }))
        .on_focused_change(clone!((value, parse_failed) move |focused| {
            if !focused {
                let mut lock = parse_failed.lock_mut();
                if lock.is_some() {
                    value.lock_mut().deref_mut();  // resurface valid value
                    *lock = None;
                }
            }
        }))
        .on_change_with_system(clone!((parse_failed) move |
            In((ui_entity, text)): In<(Entity, String)>,
            mut field: TargetField
        | {
            let result = text.parse::<T::T>();
            match result {
                Ok(new) => {
                    parse_failed.set(None);
                    field.update(ui_entity, new.clone_value());
                }
                Err(e) => {
                    parse_failed.set(Some(format!("{:?}", e)));
                }
            }
        }));
    el
}

const INITIAL_STRING_FIELD_INPUT_WIDTH: f32 = 200.;
const STRING_FIELD_GROW_THRESHOLD: usize = 16;

pub fn string_field<
    T: PartialReflect + From<String> + Into<String> + Default + PartialEq + Reflect + Clone + Debug,
>() -> impl Element {
    let padding = GLOBAL_PADDING.clone();
    TextInputField::new(T::default(), Into::into)
        .cursor(CursorIcon::System(SystemCursorIcon::Text))
        // TODO: without this initial static value, width snaps from 100% due to signal runtime lag
        .width(Val::Px(INITIAL_STRING_FIELD_INPUT_WIDTH))
        .with_text_signal(Box::new(|self_, text_signal| {
            self_.width_signal(
                text_signal
                    .map(|text| text.len())
                    .map(|len| {
                        INITIAL_STRING_FIELD_INPUT_WIDTH
                            + if len > STRING_FIELD_GROW_THRESHOLD {
                                (len - STRING_FIELD_GROW_THRESHOLD) as f32 * INPUT_WIDTH_PER_CHAR
                            } else {
                                0.
                            }
                    })
                    .map(Val::Px),
            )
        }))
        .into_el()
        .mode(CosmicWrap::InfiniteLine)
        // TODO: remove for multiline
        .max_lines(MaxLines(1))
        .text_position_signal(padding.signal().map(|padding| CosmicTextAlign::Left {
            padding: padding.round() as i32,
        }))
        .on_change_with_system(
            move |In((ui_entity, text)): In<(Entity, String)>, mut field: TargetField| {
                field.update(ui_entity, T::from(text).clone_value());
            },
        )
}

#[derive(Clone)]
pub struct FieldListener {
    handler: SystemId<In<Box<dyn PartialReflect>>>,
}

impl FieldListener {
    pub fn new(handler: SystemId<In<Box<dyn PartialReflect>>>) -> Self {
        Self { handler }
    }
}

impl Component for FieldListener {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_remove(|mut world, entity, _| {
            if let Some(&Self { handler }) = world.get::<Self>(entity) {
                world.commands().queue(move |world: &mut World| {
                    let _ = world.unregister_system(handler);
                });
            }
        });
    }
}

fn remove_from_field_path_cache(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
    if let Some(mut field_path_cache) = world.get_resource_mut::<FieldPathCache>() {
        field_path_cache.0.remove(&entity);
    }
}

#[derive(Clone, Copy, Debug)]
pub enum AccessoryTarget {
    Component {
        owner: ComponentOwnerType,
        component: ComponentId,
    },
    Asset {
        asset: TypeId,
        handle: UntypedAssetId,
    },
}

#[derive(Component, Clone, Debug)]
// TODO: not sure if this is semantically correct, we do this just as a convenience to insert SyncOnce to all accessories, but it is removed right after
#[require(SyncUiOnce)]
#[component(on_remove = remove_from_field_path_cache)]
pub struct Accessory {
    target: AccessoryTarget,
    access_option: Option<Access<'static>>,
}

pub fn sync_entities_helper(
    entities: &MutableBTreeMap<Entity, EntityData>,
    new: impl IntoIterator<Item = Entity>,
    debug_names: &Query<NameOrEntity>,
    field_path_cache: &mut ResMut<FieldPathCache>,
) {
    let mut entities = entities.lock_mut();
    let new = new.into_iter().collect::<HashSet<_>>();
    let old = entities.keys().copied().collect::<HashSet<_>>();
    for entity in new.difference(&old).copied() {
        let name_option = debug_names
            .get(entity)
            .ok()
            .and_then(|name| name.name)
            .map(|name| name.to_string());
        entities.insert_cloned(
            entity,
            EntityData {
                name: Mutable::new(name_option),
                ..default()
            },
        );
    }
    for entity in old.difference(&new) {
        entities.remove(entity);
        field_path_cache.0.remove(entity);
    }
}

#[allow(clippy::type_complexity)]
fn sync_orphan_entities(
    query: Query<
        Entity,
        (
            Without<Parent>,
            Without<HaalkaOneShotSystem>,
            Without<HaalkaObserver>,
            Without<AaloOneShotSystem>,
        ),
    >,
    debug_names: Query<NameOrEntity>,
    mut field_path_cache: ResMut<FieldPathCache>,
) {
    sync_entities_helper(
        &ORPHAN_ENTITIES,
        &query,
        &debug_names,
        &mut field_path_cache,
    )
}

#[allow(clippy::type_complexity)]
fn sync_entities(
    query: Query<
        Entity,
        (
            Without<HaalkaOneShotSystem>,
            Without<HaalkaObserver>,
            Without<AaloOneShotSystem>,
            Without<InspectorBloodline>,
        ),
    >,
    debug_names: Query<NameOrEntity>,
    mut field_path_cache: ResMut<FieldPathCache>,
) {
    sync_entities_helper(&ENTITIES, &query, &debug_names, &mut field_path_cache)
}

#[allow(clippy::type_complexity)]
fn sync_components(
    mut entity_roots: Query<
        (Entity, &mut EntityRoot),
        Or<(With<SyncComponents>, With<SyncComponentsOnce>)>,
    >,
    entities: &Entities,
    archetypes: &Archetypes,
    mut commands: Commands,
) {
    for (ui_entity, mut entity_root) in entity_roots.iter_mut() {
        if let Some(location) = entities.get(entity_root.entity) {
            if let Some(archetype) = archetypes.get(location.archetype_id) {
                let new = archetype.components().collect::<HashSet<_>>();
                let added = new
                    .difference(&entity_root.components)
                    .copied()
                    .collect::<Vec<_>>();
                let removed = entity_root
                    .components
                    .difference(&new)
                    .copied()
                    .collect::<Vec<_>>();
                entity_root.components = new;
                if let Some(mut entity) = commands.get_entity(ui_entity) {
                    if !added.is_empty() {
                        entity.trigger(ComponentsAdded(added));
                    }
                    if !removed.is_empty() {
                        entity.trigger(ComponentsRemoved(removed));
                    }
                    entity.remove::<SyncComponentsOnce>();
                }
            }
        }
    }
}

// TODO: limit size of the cache
#[derive(Resource, Default)]
pub struct FieldPathCache(HashMap<Entity, ParsedPath>);

#[derive(Component)]
pub struct Visible;

const HEADER_HEIGHT_STABILITY_COUNT: usize = 3;

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn sync_visibility(
    field_listeners: Query<
        Entity,
        (
            Or<((With<Accessory>, With<FieldListener>), With<EntityRoot>)>,
            // Changed<GlobalTransform>,  // TODO: this makes it so that fields do not regain visibility on inspector uncollapse
        ),
    >,
    relative_rect: RelativeRect,
    mutable_viewports: Query<&MutableViewport>,
    inspector_column_ancestor: InspectorColumnAncestor,
    pinned_headers: Query<&PinnedHeaders>,
    headers: Query<Entity, With<HeaderData>>,
    mut header_heights: Local<Vec<f32>>,
    mut header_height: Local<Option<f32>>,
    mut commands: Commands,
) {
    if header_height.is_none() {
        if let Some(header) = headers.iter().next() {
            if let Some(header) = relative_rect.get(header) {
                if header.height() > 0. {
                    let height = header.height().round();
                    header_heights.push(height);
                    let len = header_heights.len();
                    if len >= HEADER_HEIGHT_STABILITY_COUNT {
                        if let Some(last) = header_heights.last() {
                            for other in
                                header_heights[len - HEADER_HEIGHT_STABILITY_COUNT..len - 1].iter()
                            {
                                if (other - last).abs() > 0.001 {
                                    return;
                                }
                            }
                        }
                        header_heights.clear();
                        *header_height = Some(height);
                    }
                }
            }
        }
    }
    let Some(header_height) = *header_height else {
        return;
    };
    for entity in field_listeners.iter() {
        if let Some(((mut entity, rect), inspector_column)) = commands
            .get_entity(entity)
            .zip(relative_rect.get(entity))
            .zip(inspector_column_ancestor.get(entity))
        {
            if let Ok(mutable_viewport) = mutable_viewports.get(inspector_column) {
                let pinned_headers = pinned_headers
                    .get(inspector_column)
                    .map(Deref::deref)
                    .copied()
                    .unwrap_or(0);
                // TODO: this is not totally correct because it does not handle the case where the header has reached the end of its body and is being scrolled "under" its parent (it is no longer "pinned"), but it's good enough for now
                if rect.max.y - header_height * pinned_headers as f32 > 0.
                    && rect.min.y < mutable_viewport.viewport.height
                {
                    entity.try_insert(Visible);
                    continue;
                }
            }
            entity.remove::<Visible>();
        }
    }
}

#[derive(Component, Default)]
struct SyncUiOnce;

#[derive(Component, Default)]
struct SyncComponentsOnce;

#[allow(clippy::type_complexity)]
fn sync_ui(
    field_listeners: Query<
        (Entity, &Accessory, &FieldListener),
        Or<(With<Visible>, With<SyncUiOnce>)>,
    >,
    mut field_path_cached: FieldPathCached,
    mut commands: Commands,
) {
    for (ui_entity, &Accessory { target, .. }, &FieldListener { handler }) in field_listeners.iter()
    {
        let field_path = field_path_cached.get(ui_entity);
        commands.queue(move |world: &mut World| {
            if let Some(cur) = match target {
                AccessoryTarget::Component { owner, component } => match owner {
                    ComponentOwnerType::Entity(entity) => {
                        reflect_component(world, entity, component)
                    }
                    ComponentOwnerType::Resource => reflect_resource(world, component),
                },
                AccessoryTarget::Asset { asset, handle } => reflect_asset(world, asset, handle),
            }
            .and_then(|reflect| {
                reflect
                    .reflect_path(&field_path)
                    .ok()
                    .map(|reflect| reflect.clone_value())
            }) {
                let _ = world.run_system_with_input(handler, cur);
                if let Ok(mut entity) = world.get_entity_mut(ui_entity) {
                    entity.remove::<SyncUiOnce>();
                }
            }
        });
    }
}

#[derive(Clone, Debug, Component)]
struct InspectionTargetMutliField {
    name: String,
    field: Option<InspectionTargetField>,
}

#[derive(Clone, PartialEq, Debug, Component)]
struct InspectionTargetField {
    field: String,
    path: Option<ParsedPath>,
}

#[derive(Clone, Copy, Debug, Display, EnumIter, PartialEq, Eq, Hash)]
pub enum InspectionTargetRoot {
    Entity,
    Resource,
    Asset,
}

// impl TryInto<InspectionTargetRoot> for &str {
//     type Error = ();
//     fn try_into(self) -> Result<InspectionTargetRoot, Self::Error> {
//         match self {
//             "entity" | "entities" => Ok(InspectionTargetRoot::Entity),
//             "resource" | "resources" => Ok(InspectionTargetRoot::Resource),
//             "asset" | "assets" => Ok(InspectionTargetRoot::Asset),
//             _ => Err(()),
//         }
//     }
// }

impl TryFrom<&str> for InspectionTargetRoot {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "entity" | "entities" => Ok(InspectionTargetRoot::Entity),
            "resource" | "resources" => Ok(InspectionTargetRoot::Resource),
            "asset" | "assets" => Ok(InspectionTargetRoot::Asset),
            _ => Err("must be one of 'entity', 'resource', or 'asset'"),
        }
    }
}

#[derive(Clone, Debug)]
enum InspectionTargetInner {
    Multi(InspectionTargetMutliField),
    Solo(InspectionTargetField),
}

#[derive(Clone, Debug, Component)]
pub struct InspectionTarget {
    root: InspectionTargetRoot,
    target: Option<InspectionTargetInner>,
}

impl From<(InspectionTargetRoot, &str, &str, &str)> for InspectionTarget {
    fn from((root, multi_field, field, path): (InspectionTargetRoot, &str, &str, &str)) -> Self {
        if matches!(root, InspectionTargetRoot::Resource) {
            panic!(
                "`Resource` targets cannot be specified with a triple, try ({}, {}, {}) instead",
                root, multi_field, field
            );
        }
        let target = if !multi_field.is_empty() {
            let mut inspection_target_multi_field = InspectionTargetMutliField {
                name: multi_field.to_string(),
                field: None,
            };
            if !field.is_empty() {
                let mut inspection_target_field = InspectionTargetField {
                    field: field.to_string(),
                    path: None,
                };
                if !path.is_empty() {
                    inspection_target_field.path = ParsedPath::parse(path).ok();
                }
                inspection_target_multi_field.field = Some(inspection_target_field);
            }
            Some(inspection_target_multi_field)
        } else {
            None
        };
        Self {
            root,
            target: target.map(InspectionTargetInner::Multi),
        }
    }
}

impl From<(InspectionTargetRoot, &str, &str)> for InspectionTarget {
    fn from((root, first, second): (InspectionTargetRoot, &str, &str)) -> Self {
        match root {
            InspectionTargetRoot::Entity | InspectionTargetRoot::Asset => {
                InspectionTarget::from((root, first, second, ""))
            }
            InspectionTargetRoot::Resource => {
                let target = if !first.is_empty() {
                    let mut inspection_target_field = InspectionTargetField {
                        field: first.to_string(),
                        path: None,
                    };
                    if !second.is_empty() {
                        inspection_target_field.path = ParsedPath::parse(second).ok();
                    }
                    Some(inspection_target_field)
                } else {
                    None
                };
                InspectionTarget {
                    root,
                    target: target.map(InspectionTargetInner::Solo),
                }
            }
        }
    }
}

impl From<(InspectionTargetRoot, &str)> for InspectionTarget {
    fn from((root, first): (InspectionTargetRoot, &str)) -> Self {
        InspectionTarget::from((root, first, ""))
    }
}

impl From<InspectionTargetRoot> for InspectionTarget {
    fn from(root: InspectionTargetRoot) -> Self {
        InspectionTarget::from((root, ""))
    }
}

impl From<(&str, &str, &str, &str)> for InspectionTarget {
    fn from((first, second, third, fourth): (&str, &str, &str, &str)) -> Self {
        let root: InspectionTargetRoot = first.try_into().expect("invalid InspectionTargetRoot");
        InspectionTarget::from((root, second, third, fourth))
    }
}

impl From<(&str, &str, &str)> for InspectionTarget {
    fn from((first, second, third): (&str, &str, &str)) -> Self {
        let root = InspectionTargetRoot::try_from(first).expect("invalid InspectionTargetRoot");
        if matches!(root, InspectionTargetRoot::Resource) {
            InspectionTarget::from((root, second, third))
        } else {
            InspectionTarget::from((first, second, third, ""))
        }
    }
}

impl From<(&str, &str)> for InspectionTarget {
    fn from((first, second): (&str, &str)) -> Self {
        InspectionTarget::from((first, second, ""))
    }
}

impl From<&str> for InspectionTarget {
    fn from(first: &str) -> Self {
        InspectionTarget::from((first, ""))
    }
}

#[derive(Clone, Debug)]
enum ProgressPart {
    Field(String),
    Access(Access<'static>),
}

#[derive(Event, Clone)]
struct InspectionTargetProgress {
    pending: VecDeque<ProgressPart>,
}

#[derive(Event)]
struct RemoveTarget {
    from: Entity,
}

fn unfocus_text_input_on_keys(input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if input.any_just_pressed([KeyCode::Escape, KeyCode::Enter]) {
        commands.remove_resource::<FocusedTextInput>();
    }
}

fn left_align_editors(
    mut cosmic_editors: Query<&mut CosmicEditor>,
    mut font_system: ResMut<CosmicFontSystem>,
) {
    for mut cosmic_editor in cosmic_editors.iter_mut() {
        cosmic_editor.action(&mut font_system.0, Action::Motion(Motion::Home));
    }
}

fn maybe_clear_orphan_entities(mut world: DeferredWorld, _: Entity, _: ComponentId) {
    world.commands().queue(|world: &mut World| {
        let _ = world.run_system_once(|data: Query<&SyncOrphanEntities>| {
            if data.is_empty() {
                ORPHAN_ENTITIES.lock_mut().clear();
            }
        });
    })
}

#[derive(Component, Default)]
#[component(on_remove = maybe_clear_orphan_entities)]
pub struct SyncOrphanEntities;

fn maybe_clear_entities(mut world: DeferredWorld, _: Entity, _: ComponentId) {
    world.commands().queue(|world: &mut World| {
        let _ = world.run_system_once(|data: Query<&SyncEntities>| {
            if data.is_empty() {
                ENTITIES.lock_mut().clear();
            }
        });
    })
}

#[derive(Component, Default)]
#[component(on_remove = maybe_clear_entities)]
pub struct SyncEntities;

#[derive(SystemParam)]
pub struct InspectorColumnAncestor<'w, 's> {
    parents: Query<'w, 's, &'static Parent>,
    entity_inspector_columns: Query<'w, 's, &'static InspectorColumn>,
}

impl<'w, 's> InspectorColumnAncestor<'w, 's> {
    pub fn get(&self, entity: Entity) -> Option<Entity> {
        self.parents
            .iter_ancestors(entity)
            .find(|&ancestor| self.entity_inspector_columns.contains(ancestor))
    }
}

#[derive(SystemParam)]
pub struct RelativeRect<'w, 's> {
    logical_rect: LogicalRect<'w, 's>,
    inspector_column_ancestor: InspectorColumnAncestor<'w, 's>,
}

impl<'w, 's> RelativeRect<'w, 's> {
    pub fn get(&self, entity: Entity) -> Option<Rect> {
        if let Some(inspector_column) = self.inspector_column_ancestor.get(entity) {
            if let Some(inspector_column_rect) = self.logical_rect.get(inspector_column) {
                if let Some(target_rect) = self.logical_rect.get(entity) {
                    return Some(Rect {
                        min: target_rect.min - inspector_column_rect.min,
                        max: target_rect.max - inspector_column_rect.min,
                    });
                }
            }
        }
        None
    }
}

#[derive(SystemParam)]
pub struct Top<'w, 's> {
    logical_rect: LogicalRect<'w, 's>,
    inspector_column_ancestor: InspectorColumnAncestor<'w, 's>,
    childrens: Query<'w, 's, &'static Children>,
}

impl<'w, 's> Top<'w, 's> {
    pub fn get(&self, entity: Entity) -> Option<f32> {
        if let Some(ancestor) = self.inspector_column_ancestor.get(entity) {
            if let Some(&first) = i_born(ancestor, &self.childrens, 0) {
                if let Some((first_rect, target_rect)) = self
                    .logical_rect
                    .get(first)
                    .zip(self.logical_rect.get(entity))
                {
                    return Some(target_rect.min.y - first_rect.min.y);
                }
            }
        }
        None
    }
}

#[derive(SystemParam)]
pub struct HeaderPinner<'w, 's> {
    entity_roots: Query<'w, 's, Entity, (With<RootHeader>, With<Expanded>)>,
    headers: Query<'w, 's, &'static HeaderData>,
    expandeds: Query<'w, 's, &'static Expanded>,
    logical_rect: LogicalRect<'w, 's>,
    childrens: Query<'w, 's, &'static Children>,
    nodes: Query<'w, 's, &'static mut Node>,
    commands: Commands<'w, 's>,
}

impl<'w, 's> HeaderPinner<'w, 's> {
    // TODO: explain wut the fuck is going on here inshallah
    // `y` is the target y offset, e.g. it either is the current offset or will be apparent in a future frame, e.g. the current or future ScrollPosition.offset_y perhaps from mouse wheel scrolling or dragging the scrollbar
    pub fn sync(&mut self, inspector_column: Entity, y: f32) {
        let Some(inspector_rect) = self.logical_rect.get(inspector_column) else {
            return;
        };
        let Some(&first) = i_born(inspector_column, &self.childrens, 0) else {
            return;
        };
        let Some(first_rect) = self.logical_rect.get(first) else {
            return;
        };
        let offset_y = inspector_rect.min.y - first_rect.min.y;
        let mut pinned = 0;
        for entity_root in self.entity_roots.iter() {
            let mut i = 0;
            let mut pin_last = vec![];
            for header_root in
                std::iter::once(entity_root).chain(self.childrens.iter_descendants(entity_root))
            {
                if let Ok(HeaderData { pinned, .. }) = self.headers.get(header_root) {
                    if self.expandeds.contains(header_root) {
                        let Some(header_root_rect) = self.logical_rect.get(header_root) else {
                            continue;
                        };
                        let Some(&header) = i_born(header_root, &self.childrens, 0) else {
                            continue;
                        };
                        let Some(header_rect) = self.logical_rect.get(header) else {
                            continue;
                        };
                        let Ok(mut node) = self.nodes.get_mut(header) else {
                            continue;
                        };
                        let relative_rect = Rect {
                            min: header_root_rect.min - inspector_rect.min,
                            max: header_root_rect.max - inspector_rect.min,
                        };
                        let target_top = offset_y + relative_rect.min.y;
                        let header_offset = header_rect.size().y * i as f32;
                        let in_body = y >= target_top - header_offset
                            && relative_rect.min.y - y - header_offset < 0.
                            && relative_rect.max.y + y + offset_y > 0.;
                        if in_body {
                            let entirely_visible = inspector_rect.min.y - header_root_rect.min.y
                                + relative_rect.min.y
                                + y
                                - target_top;
                            let partially_visible =
                                relative_rect.size().y - (header_rect.size().y + header_offset);
                            let top = entirely_visible.min(partially_visible) + header_offset;
                            let is_pinned = top > 0. && entirely_visible < partially_visible;
                            node.top = Val::Px(top);
                            if !is_pinned {
                                pinned.set_neq(false);
                            } else {
                                pin_last.push(pinned.clone());
                                i += 1;
                                if let Some(mut entity) = self.commands.get_entity(header) {
                                    entity.try_insert(GlobalZIndex(z_order("header") - i));
                                }
                            }
                        } else {
                            node.top = Val::Px(0.);
                            pinned.set_neq(false);
                        }
                    } else {
                        pinned.set_neq(false);
                    }
                }
            }
            let len = pin_last.len();
            for (i, pinned) in pin_last.into_iter().enumerate() {
                pinned.set_neq(i == len - 1);
            }
            pinned = pinned.max(len);
        }
        if let Some(mut entity) = self.commands.get_entity(inspector_column) {
            if pinned > 0 {
                entity.try_insert(PinnedHeaders(pinned));
            } else {
                entity.remove::<PinnedHeaders>();
            }
        }
    }
}

#[derive(Component, Deref)]
pub struct PinnedHeaders(usize);

#[derive(SystemParam)]
pub struct MaybeScrollToHeaderRoot<'w, 's> {
    parents: Query<'w, 's, &'static Parent>,
    inspector_column_ancestor: InspectorColumnAncestor<'w, 's>,
    top: Top<'w, 's>,
    header_pinner: HeaderPinner<'w, 's>,
    scroll_positions: Query<'w, 's, &'static mut ScrollPosition>,
}

impl<'w, 's> MaybeScrollToHeaderRoot<'w, 's> {
    // `entity` must be a header
    pub fn scrolled(&mut self, entity: Entity, require_in_body: bool) -> bool {
        for ancestor in self.parents.iter_ancestors(entity) {
            if self.header_pinner.headers.contains(ancestor) {
                if let Ok(node) = self.header_pinner.nodes.get(entity) {
                    if !require_in_body || ![Val::Auto, Val::Px(0.)].contains(&node.top) {
                        if let Some((inspector_column, top)) = self
                            .inspector_column_ancestor
                            .get(entity)
                            .zip(self.top.get(ancestor))
                        {
                            if let Ok(mut scroll_position) =
                                self.scroll_positions.get_mut(inspector_column)
                            {
                                let mut expanded_ancestors = 0;
                                for ancestor in self.parents.iter_ancestors(ancestor) {
                                    if self.header_pinner.headers.contains(ancestor) {
                                        expanded_ancestors += 1;
                                    }
                                }
                                if let Some(rect) = self.header_pinner.logical_rect.get(entity) {
                                    let offset = expanded_ancestors as f32 * rect.size().y;
                                    self.header_pinner.sync(inspector_column, top - offset);
                                    let scrolled = scroll_position.offset_y != top - offset;
                                    scroll_position.offset_y = top - offset;
                                    return scrolled;
                                }
                            }
                        }
                    }
                }
                break;
            }
        }
        false
    }
}

fn on_scroll_header_pinner(
    // need to use Trigger<MouseWheel> directly because .on_scroll handlers race with each other and this needs to run before ScrollPosition is updated by the basic scroll handler
    event: Trigger<MouseWheel>,
    scroll_positions: Query<&ScrollPosition>,
    mut header_pinner: HeaderPinner,
) {
    let &MouseWheel { unit, y, .. } = event.event();
    // TODO: this should be configurable
    let dy = scroll_normalizer(unit, y, DEFAULT_SCROLL_PIXELS);
    let inspector_column = event.entity();
    if let Ok(ScrollPosition { offset_y, .. }) = scroll_positions.get(inspector_column) {
        header_pinner.sync(inspector_column, (offset_y - dy).max(0.));
    };
}

// fn on_scroll_header_pinner(
//     In((inspector, _)): In<(Entity, MouseWheel)>,
//     scroll_positions: Query<&ScrollPosition>,
//     mut header_pinner: HeaderPinner,
// ) {
//     if let Ok(scroll_position) = scroll_positions.get(inspector) {
//         header_pinner.sync(inspector, scroll_position.offset_y.max(0.));
//     };
// }

#[derive(Clone, Copy, Default, Debug)]
pub enum Viewability {
    Viewable,
    Opaque,
    Unit,
    #[default]
    NotInRegistry,
}

impl Ord for Viewability {
    fn cmp(&self, other: &Self) -> Ordering {
        if matches!(self, Viewability::NotInRegistry) {
            if matches!(other, Viewability::NotInRegistry) {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        } else if !matches!(other, Viewability::NotInRegistry) {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    }
}

impl PartialOrd for Viewability {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Viewability {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Viewability {}

pub fn i_born<'a>(entity: Entity, childrens: &'a Query<&Children>, i: usize) -> Option<&'a Entity> {
    childrens
        .get(entity)
        .ok()
        .and_then(|children| children.get(i))
}

#[derive(Component)]
pub struct WaitForBirth {
    ceiling: Option<Entity>,
}

#[derive(Event)]
pub struct Born;

fn wait_for_birth(
    mut birth_waiters: Query<(Entity, &mut WaitForBirth)>,
    inspector_column_ancestor: InspectorColumnAncestor,
    parents: Query<&Parent>,
    childrens: Query<&Children>,
    computed_nodes: Query<&ComputedNode>,
    mut commands: Commands,
) {
    for (entity, mut wait_for_birth) in birth_waiters.iter_mut() {
        let ceiling = match wait_for_birth.ceiling {
            Some(ceiling) => ceiling,
            None => {
                if let Some(inspector_column) = inspector_column_ancestor.get(entity) {
                    inspector_column
                } else {
                    return;
                }
            }
        };
        let mut path = vec![entity];
        for ancestor in parents.iter_ancestors(entity) {
            path.push(ancestor);
            if ancestor == ceiling {
                break;
            }
        }
        path.reverse();
        'outer: for parent_target in path.windows(2) {
            // TODO: use this instead once stable https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#method.array_chunks
            let parent = parent_target[0];
            let target = parent_target[1];
            if let Ok(children) = childrens.get(parent) {
                for &child in children.iter() {
                    let Ok(computed_node) = computed_nodes.get(child) else {
                        return;
                    };
                    if computed_node.size().y == 0. {
                        wait_for_birth.ceiling = Some(parent);
                        return;
                    }
                    if child == target {
                        continue 'outer;
                    }
                }
            }
        }
        if let Some(mut entity) = commands.get_entity(entity) {
            entity.trigger(Born);
        }
    }
}

fn sync_names(names: Query<(Entity, &Name), Changed<Name>>, entity_roots: Query<&EntityRoot>) {
    for (entity, new_name) in names.iter() {
        if let Some(EntityRoot { name, .. }) = entity_roots
            .iter()
            .find(|EntityRoot { entity: e, .. }| entity == *e)
        {
            name.set_neq(Some(new_name.as_str().to_string()));
        }
    }
}

#[derive(Event)]
struct ShowSearch;

#[derive(Event)]
struct HideSearch;

#[derive(Event)]
struct ShowTargeting;

#[derive(Event)]
struct HideTargeting;

#[derive(Event)]
enum Tab {
    Up,
    Down,
}

// TODO: make hotkeys configurable
fn hotkey_forwarder(
    keys: Res<ButtonInput<KeyCode>>,
    selected_inspector_option: Option<Res<SelectedInspector>>,
    mut commands: Commands,
) {
    if let Some(selected_inspector) = selected_inspector_option {
        // released because pressed causes input to be inserted into the text input on release build (2fast4me)
        if keys.just_released(KeyCode::Slash) {
            commands.trigger_targets(ShowSearch, selected_inspector.0);
        }
        if (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
            && keys.just_released(KeyCode::Semicolon)
        {
            commands.trigger_targets(ShowTargeting, selected_inspector.0);
        }
        if keys.just_pressed(KeyCode::Escape) {
            commands.trigger_targets(HideSearch, selected_inspector.0);
            commands.trigger_targets(HideTargeting, selected_inspector.0);
        }
        if keys.just_pressed(KeyCode::Tab) {
            if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                commands.trigger_targets(Tab::Down, selected_inspector.0);
            } else {
                commands.trigger_targets(Tab::Up, selected_inspector.0);
            }
        }
        if keys.just_pressed(KeyCode::ArrowLeft) {
            commands.trigger_targets(TargetRootMove::Left, selected_inspector.0);
        }
        if keys.just_pressed(KeyCode::ArrowRight) {
            commands.trigger_targets(TargetRootMove::Right, selected_inspector.0);
        }
    }
}

#[derive(Event)]
enum TargetRootMove {
    Left,
    Right,
}

#[derive(Event)]
pub struct SizeReached;

fn wait_for_size(
    data: Query<(Entity, &WaitForSize)>,
    computed_nodes: Query<&ComputedNode>,
    mut commands: Commands,
) {
    for (entity, &WaitForSize(size_option)) in data.iter() {
        if let Ok(computed_node) = computed_nodes.get(entity) {
            let size = computed_node.size();
            let trigger = if let Some(target_size) = size_option {
                size.x >= target_size.x && size.y >= target_size.y
            } else {
                size.x > 0. && size.y > 0.
            };
            if trigger {
                commands.trigger_targets(SizeReached, entity);
            }
        }
    }
}

const RESIZE_BORDER_SLACK_PERCENT: f32 = 100.;

#[derive(Component, Debug)]
pub struct ResizeParent;

#[derive(SystemParam)]
pub struct ResizeParentCache<'w, 's> {
    parents: Query<'w, 's, &'static Parent>,
    resize_parent: Local<'s, Option<Entity>>,
    resize_parents: Query<'w, 's, &'static ResizeParent>,
}

impl<'w, 's> ResizeParentCache<'w, 's> {
    pub fn get(mut self, entity: Entity) -> Option<Entity> {
        if self.resize_parent.is_none() {
            for parent in self.parents.iter_ancestors(entity) {
                if self.resize_parents.contains(parent) {
                    *self.resize_parent = Some(parent);
                }
            }
        }
        *self.resize_parent
    }
}

pub fn manage_dragging_component(el: RawHaalkaEl) -> RawHaalkaEl {
    // TODO: this should be DragStart but looks like on web all Down's are DragStart's ?
    el.on_event_with_system::<Pointer<Drag>, _>(|In((entity, _)), mut commands: Commands| {
        if let Some(mut entity) = commands.get_entity(entity) {
            entity.try_insert(Dragging);
        }
    })
    .on_event_with_system::<Pointer<DragEnd>, _>(|In((entity, _)), mut commands: Commands| {
        if let Some(mut entity) = commands.get_entity(entity) {
            entity.remove::<Dragging>();
        }
    })
}

// this is cursed, do not use this, wait for this https://github.com/bevyengine/bevy/issues/14773
pub fn resize_border<E: Element + Sizeable>(
    border_width: impl Signal<Item = f32> + Send + Sync + 'static,
    border_radius: impl Signal<Item = f32> + Send + Sync + 'static,
    base_color: Mutable<Color>,
    hovered_color: Mutable<Color>,
    pressed_color: Mutable<Color>,
    disabled: Mutable<bool>,
    wrapper_stack_option: Option<Stack<Node>>,
) -> impl FnOnce(E) -> Stack<Node> {
    move |el| {
        let edge_hovereds = BoxEdge::iter()
            .map(|_| Mutable::new(false))
            .collect::<Vec<_>>();
        let corner_hovereds = BoxCorner::iter()
            .map(|_| Mutable::new(false))
            .collect::<Vec<_>>();
        let edge_downs = BoxEdge::iter()
            .map(|_| Mutable::new(false))
            .collect::<Vec<_>>()
            .apply(Arc::new);
        let border_width = border_width.dedupe().broadcast();
        let radius = border_radius.dedupe().broadcast();
        let edge_hovered = |edge| match edge {
            BoxEdge::Top => signal_or!(
                edge_hovereds[0].signal(),
                corner_hovereds[0].signal(),
                corner_hovereds[1].signal(),
                edge_downs[0].signal()
            ),
            BoxEdge::Bottom => signal_or!(
                edge_hovereds[1].signal(),
                corner_hovereds[2].signal(),
                corner_hovereds[3].signal(),
                edge_downs[1].signal()
            ),
            BoxEdge::Left => signal_or!(
                edge_hovereds[2].signal(),
                corner_hovereds[0].signal(),
                corner_hovereds[2].signal(),
                edge_downs[2].signal()
            ),
            BoxEdge::Right => signal_or!(
                edge_hovereds[3].signal(),
                corner_hovereds[1].signal(),
                corner_hovereds[3].signal(),
                edge_downs[3].signal()
            ),
        };
        #[allow(clippy::unwrap_or_default)]
        let mut el = wrapper_stack_option
            .unwrap_or_else(Stack::<Node>::new)
            .update_raw_el(|raw_el| raw_el.insert(ResizeParent))
            .apply(border_radius_style(BoxCorner::ALL, radius.signal()))
            .layer({
                El::<Node>::new()
                    .apply(padding_style(BoxEdge::ALL, border_width.signal()))
                    .with_node(|mut node| node.overflow = Overflow::clip())
                    .child(
                        el.apply(border_radius_style(BoxCorner::ALL, radius.signal()))
                            .height(Val::Percent(100.))
                            .width(Val::Percent(100.)),
                    )
            })
            .layer({
                let mut el = El::<Node>::new()
                    .align(Align::center())
                    .height(Val::Percent(100.))
                    .width(Val::Percent(100.))
                    .apply(border_radius_style(BoxCorner::ALL, radius.signal()))
                    .apply(border_color_style(
                        signal_or!(
                            edge_downs[0].signal(),
                            edge_downs[1].signal(),
                            edge_downs[2].signal(),
                            edge_downs[3].signal()
                        )
                        .map_bool_signal(
                            move || pressed_color.signal(),
                            move || hovered_color.signal(),
                        ),
                    ));
                for edge in BoxEdge::iter() {
                    el = el.apply(border_width_style(
                        [edge],
                        edge_hovered(edge)
                            .map_true_signal(clone!((border_width) move || border_width.signal()))
                            .map(Option::unwrap_or_default),
                    ));
                }
                el
            })
            .layer({
                let mut el = El::<Node>::new()
                    .align(Align::center())
                    .height(Val::Percent(100.))
                    .width(Val::Percent(100.))
                    .apply(border_radius_style(
                        BoxCorner::ALL,
                        radius.signal().map(|radius| radius * 0.8),
                    ))
                    .apply(border_color_style(base_color.signal()))
                    .apply(padding_style(
                        [BoxEdge::Top],
                        signal::and(edge_hovered(BoxEdge::Top), signal::not(disabled.signal()))
                            .map_true_signal(clone!((border_width) move || border_width.signal()))
                            .map(Option::unwrap_or_default),
                    ))
                    .apply(padding_style(
                        [BoxEdge::Left],
                        signal::and(edge_hovered(BoxEdge::Left), signal::not(disabled.signal()))
                            .map_true_signal(clone!((border_width) move || border_width.signal()))
                            .map(Option::unwrap_or_default),
                    ))
                    .apply(padding_style(
                        [BoxEdge::Right],
                        signal::and(edge_hovered(BoxEdge::Right), signal::not(disabled.signal()))
                            .map_true_signal(clone!((border_width) move || border_width.signal()))
                            .map(Option::unwrap_or_default),
                    ));
                for edge in BoxEdge::iter() {
                    el = el.apply(border_width_style(
                        [edge],
                        signal::and(edge_hovered(edge), signal::not(disabled.signal()))
                            .map_false_signal(clone!((border_width) move || border_width.signal()))
                            .map(Option::unwrap_or_default),
                    ));
                }
                el
            });
        let border_width_slack = border_width
            .signal()
            .map(|width| width * RESIZE_BORDER_SLACK_PERCENT / 100.)
            .broadcast();
        let resize_border_width = border_width
            .signal()
            .map(|width| width + width * RESIZE_BORDER_SLACK_PERCENT / 100. * 2.)
            .broadcast();
        let hovereds = MutableVec::from(edge_hovereds);
        let hovered_iter = hovereds.lock_ref().iter().cloned().collect::<Vec<_>>();
        for (edge, hovered) in BoxEdge::iter().zip(hovered_iter) {
            el = el.layer({
                let mut el = El::<Node>::new()
                    .update_raw_el(clone!((edge_downs) |raw_el| {
                        raw_el
                        .apply(manage_dragging_component)
                        .apply(trigger_double_click::<Dragging>)
                        .on_event_with_system_disableable_signal::<DoubleClick, _>(
                            move |
                                In((entity, _)),
                                resize_parent_cache: ResizeParentCache,
                                mut nodes: Query<&mut Node>,
                            | {
                                if let Some(resize_parent) = resize_parent_cache.get(entity) {
                                    if let Ok(mut node) = nodes.get_mut(resize_parent) {
                                        if matches!(edge, BoxEdge::Top | BoxEdge::Bottom) {
                                            node.height = Val::Px(DEFAULT_HEIGHT);
                                        } else {
                                            node.width = Val::Px(DEFAULT_WIDTH);
                                        }
                                    }
                                }
                            },
                            disabled.signal()
                        )
                        .on_event_with_system_disableable_signal::<Pointer<Down>, _>(
                            clone!((edge_downs) move |In((_, down)): In<(_, Pointer<Down>)>, world: &mut World| {
                                let mut new = vec![];
                                if matches!(down.button, PointerButton::Primary) {
                                    match edge {
                                        BoxEdge::Top => {
                                            edge_downs[0].set_neq(true);
                                            new.push(Box::new(clone!((edge_downs) move || {
                                                edge_downs[0].set_neq(false);
                                            })) as Box<_>);
                                        },
                                        BoxEdge::Bottom => {
                                            edge_downs[1].set_neq(true);
                                            new.push(Box::new(clone!((edge_downs) move || {
                                                edge_downs[1].set_neq(false);
                                            })) as Box<_>);
                                        },
                                        BoxEdge::Left => {
                                            edge_downs[2].set_neq(true);
                                            new.push(Box::new(clone!((edge_downs) move || {
                                                edge_downs[2].set_neq(false);
                                            })) as Box<_>);
                                        },
                                        BoxEdge::Right => {
                                            edge_downs[3].set_neq(true);
                                            new.push(Box::new(clone!((edge_downs) move || {
                                                edge_downs[3].set_neq(false);
                                            })) as Box<_>);
                                        },
                                    }
                                }
                                if let Some(mut handlers) = world.get_resource_mut::<OnPointerUpHandlers>() {
                                    handlers.0.extend(new);
                                } else {
                                    world.insert_resource(OnPointerUpHandlers(new));
                                };
                            }),
                            disabled.signal(),
                        )
                        .on_event_with_system_disableable_signal::<Pointer<DragStart>, _>(
                            |In((_, drag_start)): In<(_, Pointer<DragStart>)>, mut commands: Commands| {
                                if matches!(drag_start.button, PointerButton::Primary) {
                                    commands.insert_resource(CursorOnHoverDisabled);
                                    commands.insert_resource(UpdateHoverStatesDisabled);
                                }
                            },
                            disabled.signal(),
                        )
                        .on_event_with_system_disableable_signal::<Pointer<DragEnd>, _>(
                            |In((_, drag_end)): In<(_, Pointer<DragEnd>)>, mut commands: Commands| {
                                if matches!(drag_end.button, PointerButton::Primary) {
                                    commands.remove_resource::<CursorOnHoverDisabled>();
                                    commands.remove_resource::<UpdateHoverStatesDisabled>();
                                }
                            },
                            disabled.signal(),
                        )
                        .on_event_with_system_disableable_signal::<Pointer<Drag>, _>(
                            move |In((entity, drag)): In<(Entity, Pointer<Drag>)>,
                                resize_parent_cache: ResizeParentCache,
                                mut nodes: Query<&mut Node>| {
                            if matches!(drag.button, PointerButton::Primary) {
                                if let Some(resize_parent) = resize_parent_cache.get(entity) {
                                    if let Ok(mut node) = nodes.get_mut(resize_parent) {
                                        match edge {
                                            BoxEdge::Top => {
                                                if let Val::Px(cur) = node.height {
                                                    node.height = Val::Px(cur - drag.delta.y);
                                                }
                                                let cur = if let Val::Px(cur) = node.top { cur } else { 0. };
                                                node.top = Val::Px(cur + drag.delta.y);
                                            }
                                            BoxEdge::Bottom => {
                                                if let Val::Px(cur) = node.height {
                                                    node.height = Val::Px(cur + drag.delta.y);
                                                }
                                            }
                                            BoxEdge::Left => {
                                                if let Val::Px(cur) = node.width {
                                                    node.width = Val::Px(cur - drag.delta.x);
                                                }
                                                let cur = if let Val::Px(cur) = node.left { cur } else { 0. };
                                                node.left = Val::Px(cur + drag.delta.x);
                                            }
                                            BoxEdge::Right => {
                                                if let Val::Px(cur) = node.width {
                                                    node.width = Val::Px(cur + drag.delta.x);
                                                }
                                            }
                                        }
                                    }
                                }
                            }},
                            disabled.signal(),
                        )
                    }))
                    .on_signal_with_node(
                        border_width_slack.signal().map(Val::Px),
                        move |mut node, slack| match edge {
                            BoxEdge::Top => node.top = -slack,
                            BoxEdge::Bottom => node.bottom = -slack,
                            BoxEdge::Left => node.left = -slack,
                            BoxEdge::Right => node.right = -slack,
                        },
                    )
                    .hovered_sync(hovered.clone())
                    .cursor_disableable_signal(
                        CursorIcon::System(match edge {
                            BoxEdge::Top | BoxEdge::Bottom => SystemCursorIcon::NsResize,
                            BoxEdge::Left | BoxEdge::Right => SystemCursorIcon::EwResize,
                        }),
                        disabled.signal(),
                    )
                    .align(match edge {
                        BoxEdge::Top => Align::new().top(),
                        BoxEdge::Bottom => Align::new().bottom(),
                        BoxEdge::Left => Align::new().left(),
                        BoxEdge::Right => Align::new().right(),
                    })
                    .background_color(BackgroundColor(Color::NONE));
                match edge {
                    BoxEdge::Left | BoxEdge::Right => {
                        el = el
                            .height(Val::Percent(100.))
                            .width_signal(resize_border_width.signal().map(Val::Px));
                    }
                    BoxEdge::Top | BoxEdge::Bottom => {
                        el = el
                            .height_signal(resize_border_width.signal().map(Val::Px))
                            .width(Val::Percent(100.));
                    }
                }
                el
            });
        }
        for (corner, hovered) in BoxCorner::iter().zip(corner_hovereds.iter()) {
            el = el.layer(
                El::<Node>::new()
                .update_raw_el(clone!((edge_downs, disabled) move |raw_el| {
                    raw_el
                    .apply(manage_dragging_component)
                    .apply(trigger_double_click::<Dragging>)
                    .on_event_with_system_disableable_signal::<DoubleClick, _>(
                        |
                            In((entity, _)),
                            resize_parent_cache: ResizeParentCache,
                            mut nodes: Query<&mut Node>,
                        | {
                            if let Some(resize_parent) = resize_parent_cache.get(entity) {
                                if let Ok(mut node) = nodes.get_mut(resize_parent) {
                                    node.height = Val::Px(DEFAULT_HEIGHT);
                                    node.width = Val::Px(DEFAULT_WIDTH);
                                }
                            }
                        },
                        disabled.signal(),
                    )
                    .on_event_with_system_disableable_signal::<Pointer<Down>, _>(
                        clone!((edge_downs) move |In((_, down)): In<(_, Pointer<Down>)>, world: &mut World| {
                            let mut new = vec![];
                            if matches!(down.button, PointerButton::Primary) {
                                match corner {
                                    BoxCorner::TopLeft => {
                                        edge_downs[0].set_neq(true);
                                        edge_downs[2].set_neq(true);
                                        new.push(Box::new(clone!((edge_downs) move || {
                                            edge_downs[0].set_neq(false);
                                            edge_downs[2].set_neq(false);
                                        })) as Box<_>);
                                    },
                                    BoxCorner::TopRight => {
                                        edge_downs[0].set_neq(true);
                                        edge_downs[3].set_neq(true);
                                        new.push(Box::new(clone!((edge_downs) move || {
                                            edge_downs[0].set_neq(false);
                                            edge_downs[3].set_neq(false);
                                        })) as Box<_>);
                                    },
                                    BoxCorner::BottomLeft => {
                                        edge_downs[1].set_neq(true);
                                        edge_downs[2].set_neq(true);
                                        new.push(Box::new(clone!((edge_downs) move || {
                                            edge_downs[1].set_neq(false);
                                            edge_downs[2].set_neq(false);
                                        })) as Box<_>);
                                    },
                                    BoxCorner::BottomRight => {
                                        edge_downs[1].set_neq(true);
                                        edge_downs[3].set_neq(true);
                                        new.push(Box::new(clone!((edge_downs) move || {
                                            edge_downs[1].set_neq(false);
                                            edge_downs[3].set_neq(false);
                                        })) as Box<_>);
                                    },
                                }
                            }
                            if let Some(mut handlers) = world.get_resource_mut::<OnPointerUpHandlers>() {
                                handlers.0.extend(new);
                            } else {
                                world.insert_resource(OnPointerUpHandlers(new));
                            };
                        }),
                        disabled.signal(),
                    )
                    .on_event_with_system_disableable_signal::<Pointer<DragStart>, _>(
                        |In((_, drag_start)): In<(_, Pointer<DragStart>)>, mut commands: Commands| {
                            if matches!(drag_start.button, PointerButton::Primary) {
                                commands.insert_resource(CursorOnHoverDisabled);
                                commands.insert_resource(UpdateHoverStatesDisabled);
                            }
                        },
                        disabled.signal(),
                    )
                    .on_event_with_system_disableable_signal::<Pointer<DragEnd>, _>(
                        |In((_, drag_end)): In<(_, Pointer<DragEnd>)>, mut commands: Commands| {
                            if matches!(drag_end.button, PointerButton::Primary) {
                                commands.remove_resource::<CursorOnHoverDisabled>();
                                commands.remove_resource::<UpdateHoverStatesDisabled>();
                            }
                        },
                        disabled.signal(),
                    )
                    .on_event_with_system_disableable_signal::<Pointer<Drag>, _>(
                        move |
                            In((entity, drag)): In<(Entity, Pointer<Drag>)>,
                            resize_parent_cache: ResizeParentCache,
                            mut nodes: Query<&mut Node>
                        | {
                            if matches!(drag.button, PointerButton::Primary) {
                                if let Some(resize_parent) = resize_parent_cache.get(entity) {
                                    if let Ok(mut node) = nodes.get_mut(resize_parent) {
                                        match corner {
                                            BoxCorner::TopLeft => {
                                                if let Val::Px(cur) = node.height {
                                                    node.height = Val::Px(cur - drag.delta.y);
                                                }
                                                let cur = if let Val::Px(cur) = node.top { cur } else { 0. };
                                                node.top = Val::Px(cur + drag.delta.y);
                                                if let Val::Px(cur) = node.width {
                                                    node.width = Val::Px(cur - drag.delta.x);
                                                }
                                                let cur = if let Val::Px(cur) = node.left { cur } else { 0. };
                                                node.left = Val::Px(cur + drag.delta.x);
                                            }
                                            BoxCorner::TopRight => {
                                                if let Val::Px(cur) = node.height {
                                                    node.height = Val::Px(cur - drag.delta.y);
                                                }
                                                let cur = if let Val::Px(cur) = node.top { cur } else { 0. };
                                                node.top = Val::Px(cur + drag.delta.y);
                                                if let Val::Px(cur) = node.width {
                                                    node.width = Val::Px(cur + drag.delta.x);
                                                }
                                            }
                                            BoxCorner::BottomLeft => {
                                                if let Val::Px(cur) = node.height {
                                                    node.height = Val::Px(cur + drag.delta.y);
                                                }
                                                if let Val::Px(cur) = node.width {
                                                    node.width = Val::Px(cur - drag.delta.x);
                                                }
                                                let cur = if let Val::Px(cur) = node.left { cur } else { 0. };
                                                node.left = Val::Px(cur + drag.delta.x);
                                            }
                                            BoxCorner::BottomRight => {
                                                if let Val::Px(cur) = node.height {
                                                    node.height = Val::Px(cur + drag.delta.y);
                                                }
                                                if let Val::Px(cur) = node.width {
                                                    node.width = Val::Px(cur + drag.delta.x);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        disabled.signal(),
                    )
                }))
                .apply(square_style(resize_border_width.signal().map(|width| width * 2.)))
                .on_signal_with_node(border_width_slack.signal(), move |mut node, slack| {
                    match corner {
                        BoxCorner::TopLeft => {
                            node.top = -Val::Px(slack);
                            node.left = -Val::Px(slack);
                        }
                        BoxCorner::TopRight => {
                            node.top = -Val::Px(slack);
                            node.right = -Val::Px(slack);
                        }
                        BoxCorner::BottomLeft => {
                            node.bottom = -Val::Px(slack);
                            node.left = -Val::Px(slack);
                        }
                        BoxCorner::BottomRight => {
                            node.bottom = -Val::Px(slack);
                            node.right = -Val::Px(slack);
                        }
                    }
                })
                .hovered_sync(hovered.clone())
                .cursor_disableable_signal(
                    CursorIcon::System(match corner {
                        BoxCorner::TopLeft | BoxCorner::BottomRight => SystemCursorIcon::NwseResize,
                        BoxCorner::TopRight | BoxCorner::BottomLeft => SystemCursorIcon::NeswResize,
                    }),
                    disabled.signal(),
                )
                .align(match corner {
                    BoxCorner::TopLeft => Align::new().top().left(),
                    BoxCorner::TopRight => Align::new().top().right(),
                    BoxCorner::BottomLeft => Align::new().bottom().left(),
                    BoxCorner::BottomRight => Align::new().bottom().right(),
                })
                .background_color(BackgroundColor(Color::NONE))
            );
        }
        el
    }
}

#[derive(Debug, Clone, TypePath, AsBindGroup, Asset)]
struct LightRaysMaterial {
    #[texture(0)]
    #[sampler(1)]
    texture: Option<Handle<Image>>,
    #[uniform(2)]
    translation: Vec4, // only x/y are used, the rest are padding for webgl2
    #[uniform(3)]
    size: Vec4, // only x is used, the rest are padding for webgl2
    entity: Entity,
}

// TODO: 0.16 migrate to weak_handle!
const LIGHT_RAYS: Handle<Shader> = Handle::weak_from_u128(163308648016094179464119256462205316257);

impl LightRaysMaterial {
    fn new(text_entity: Entity) -> Self {
        Self {
            texture: Some(TextAtlas::DEFAULT_IMAGE.clone_weak()),
            translation: Vec4::ZERO,
            size: Vec4::new(DEFAULT_FONT_SIZE + 2., 0., 0., 0.),
            entity: text_entity,
        }
    }
}

impl Material2d for LightRaysMaterial {
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(LIGHT_RAYS)
    }
}

#[derive(Component)]
struct LightRays;

fn update_light_rays_material(
    mut materials: ResMut<Assets<LightRaysMaterial>>,
    global_transforms: Query<&GlobalTransform, (With<LightRays>, With<GlobalTransform>)>,
) {
    for (_, material) in materials.iter_mut() {
        if let Ok(global_transform) = global_transforms.get(material.entity) {
            material.translation = global_transform.translation().xyzz();
        }
    }
}

pub const AALO_TEXT_CAMERA_ORDER: isize =
    bevy_dev_tools::ui_debug_overlay::LAYOUT_DEBUG_CAMERA_ORDER - 1;
pub static AALO_TEXT_CAMERA_RENDER_LAYERS: Lazy<RenderLayers> = Lazy::new(|| {
    RenderLayers::layer(
        bevy_dev_tools::ui_debug_overlay::LAYOUT_DEBUG_LAYERS
            .iter()
            .next()
            .unwrap()
            - 1,
    )
});

#[derive(Component, Default)]
#[require(
    Camera2d,
    Camera(||Camera { order: AALO_TEXT_CAMERA_ORDER, clear_color: ClearColorConfig::None, ..default() }),
    RenderLayers(|| AALO_TEXT_CAMERA_RENDER_LAYERS.clone()),
)]
struct AaloTextCamera;

pub static RESOURCES: Lazy<MutableBTreeMap<ComponentId, FieldData>> = Lazy::new(default);

#[derive(Component)]
pub struct SyncResources;

fn sync_resources(type_registry: Res<AppTypeRegistry>, components: &Components) {
    let mut new = HashSet::new();
    let old = RESOURCES.lock_ref().keys().copied().collect::<HashSet<_>>();
    let type_registry = type_registry.read();
    for registration in type_registry.iter() {
        if registration.data::<ReflectResource>().is_some() {
            if let Some(component) = components.get_resource_id(registration.type_id()) {
                new.insert(component);
            }
        }
    }
    let mut resources = RESOURCES.lock_mut();
    for component in new.difference(&old).copied() {
        if let Some(info) = components.get_info(component) {
            resources.insert_cloned(
                component,
                FieldData {
                    name: info.name().to_string(),
                    ..default()
                },
            );
        }
    }
    for component in old.difference(&new) {
        resources.remove(component);
    }
}

#[derive(Clone, Default)]
pub struct AssetData {
    pub name: &'static str,
    pub expanded: Mutable<bool>,
    pub filtered: Mutable<bool>,
    pub handles: MutableBTreeMap<UntypedAssetId, FieldData>,
}

pub static ASSETS: Lazy<MutableBTreeMap<TypeId, AssetData>> = Lazy::new(default);

#[derive(Component)]
pub struct SyncAssets;

fn sync_assets(type_registry: Res<AppTypeRegistry>) {
    let mut new = HashSet::new();
    let old = ASSETS.lock_ref().keys().copied().collect::<HashSet<_>>();
    let type_registry = type_registry.read();
    for registration in type_registry.iter() {
        if registration.data::<ReflectAsset>().is_some() {
            new.insert(registration.type_id());
        }
    }
    let mut assets = ASSETS.lock_mut();
    for asset in new.difference(&old).copied() {
        if let Some(info) = type_registry.get_type_info(asset) {
            assets.insert_cloned(
                asset,
                AssetData {
                    name: info.type_path(),
                    ..default()
                },
            );
        }
    }
    for asset in old.difference(&new) {
        assets.remove(asset);
    }
}

#[derive(Component, Default)]
pub struct SyncAssetHandles;

#[derive(Event)]
struct AssetHandlesAdded(Vec<UntypedAssetId>);

#[derive(Event)]
struct AssetHandlesRemoved(Vec<UntypedAssetId>);

#[allow(clippy::type_complexity)]
fn sync_asset_handles(
    mut asset_roots: Query<
        (Entity, &mut AssetRoot),
        Or<(With<SyncAssets>, With<SyncAssetHandlesOnce>)>,
    >,
    type_registry: Res<AppTypeRegistry>,
    mut commands: Commands,
) {
    let type_registry = type_registry.clone();
    for (ui_entity, asset_root) in asset_roots.iter_mut() {
        let asset = asset_root.asset;
        let handles = asset_root.handles.clone();
        commands.queue(clone!((type_registry) move |world: &mut World| {
            let type_registry = type_registry.read();
            if let Some(registration) = type_registry.get(asset) {
                if let Some(reflect_asset) = registration.data::<ReflectAsset>() {
                    let new = reflect_asset.ids(world).collect::<HashSet<_>>();
                    let added = new.difference(&handles).copied().collect::<Vec<_>>();
                    let removed = handles.difference(&new).copied().collect::<Vec<_>>();
                    if !added.is_empty() && !removed.is_empty() {
                        world.trigger(UpdateAssetHandles {
                            entity: ui_entity,
                            handles: new,
                        });
                    }
                    if !added.is_empty() {
                        world.trigger_targets(AssetHandlesAdded(added), ui_entity);
                    }
                    if !removed.is_empty() {
                        world.trigger_targets(AssetHandlesRemoved(removed), ui_entity);
                    }
                    if let Ok(mut entity) = world.get_entity_mut(ui_entity) {
                        entity.remove::<SyncAssetHandlesOnce>();
                    }
                }
            }
        }));
    }
}

// adapted from https://github.com/jakobhellermann/bevy-inspector-egui/blob/8d7580b304c7202054e5d821f6269e1e32b58712/crates/bevy-inspector-egui/src/bevy_inspector/mod.rs#L973
pub fn handle_name(handle: UntypedAssetId, asset_server: &AssetServer) -> String {
    if let Some(path) = asset_server.get_path(handle) {
        return path.to_string();
    }
    match handle {
        UntypedAssetId::Index { index, .. } => {
            format!("{:04X}", index.to_bits())
        }
        UntypedAssetId::Uuid { uuid, .. } => {
            format!("{}", uuid)
        }
    }
}

#[derive(Event)]
struct UpdateAssetHandles {
    entity: Entity,
    handles: HashSet<UntypedAssetId>,
}

#[derive(Event)]
struct OnPointerUpFlush;

#[derive(Resource, Default)]
pub struct OnPointerUpHandlers(pub Vec<Box<dyn FnMut() + Send + Sync + 'static>>);

fn listen_for_pointer_release(mouse_inputs: Res<ButtonInput<MouseButton>>, mut commands: Commands) {
    if mouse_inputs.just_released(MouseButton::Left) {
        commands.trigger(OnPointerUpFlush);
    }
}

// static SYNC_VISIBILITY_SYSTEM: OnceLock<SystemId> = OnceLock::new();

pub(super) fn plugin(app: &mut App) {
    // SYNC_VISIBILITY_SYSTEM
    //     .set(app.register_system(sync_visibility))
    //     .expect("failed to initialize SYNC_VISIBILITY_SYSTEM");
    if !app.is_plugin_added::<HaalkaPlugin>() {
        app.add_plugins(HaalkaPlugin);
    }
    bevy_asset::load_internal_asset!(app, LIGHT_RAYS, "light_rays.wgsl", Shader::from_wgsl);
    app.add_plugins(Material2dPlugin::<LightRaysMaterial>::default())
        .add_plugins(Text3dPlugin {
            load_font_embedded: vec![DEFAULT_FONT_DATA],
            ..default()
        })
        .add_systems(
            PreUpdate,
            propagate_inspector_bloodline.run_if(any_with_component::<InspectorBloodline>),
        )
        .add_systems(
            Update,
            (
                sync_orphan_entities.run_if(any_with_component::<SyncOrphanEntities>),
                sync_entities.run_if(any_with_component::<SyncEntities>),
                sync_components.run_if(any_with_component::<EntityRoot>),
                sync_resources.run_if(any_with_component::<SyncResources>),
                sync_assets.run_if(any_with_component::<SyncAssets>),
                sync_asset_handles.run_if(any_with_component::<AssetRoot>),
                sync_ui.run_if(any_with_component::<FieldListener>),
                (
                    unfocus_text_input_on_keys.run_if(resource_changed::<ButtonInput<KeyCode>>),
                    left_align_editors.run_if(resource_removed::<FocusedTextInput>),
                )
                    .chain(),
                wait_for_birth.run_if(any_with_component::<WaitForBirth>),
                sync_names.run_if(any_with_component::<EntityRoot>),
                sync_visibility.run_if(
                    any_with_component::<FieldListener>.or(any_with_component::<EntityRoot>),
                ),
                hotkey_forwarder.run_if(
                    resource_exists::<SelectedInspector>
                        .and(resource_changed::<ButtonInput<KeyCode>>),
                ),
                wait_for_size.run_if(any_with_component::<WaitForSize>),
                update_light_rays_material.run_if(any_with_component::<LightRays>),
                sync_aalo_text_position.run_if(any_with_component::<AaloTextCamera>),
                forward_aalo_text_visibility.run_if(any_with_component::<AaloText>),
                wait_until_non_zero_transform
                    .run_if(any_with_component::<WaitUntilNonZeroTransform>),
                listen_for_pointer_release.run_if(
                    resource_exists::<OnPointerUpHandlers>
                        .and(resource_changed::<ButtonInput<MouseButton>>),
                ),
            ),
        )
        .init_resource::<FieldPathCache>()
        .insert_resource(bevy_cosmic_edit::CursorPluginDisabled)
        .add_observer(
            |event: Trigger<RemoveTarget>, parents: Query<&Parent>, mut commands: Commands| {
                let &RemoveTarget { from } = event.event();
                for parent in parents.iter_ancestors(from) {
                    if let Some(mut entity) = commands.get_entity(parent) {
                        entity.remove::<InspectionTargetProgress>();
                        entity.remove::<InspectionTarget>();
                    }
                }
            },
        )
        .add_observer(
            |event: Trigger<UpdateAssetHandles>, mut asset_roots: Query<&mut AssetRoot>| {
                let UpdateAssetHandles { entity, handles } = event.event();
                if let Ok(mut asset_root) = asset_roots.get_mut(*entity) {
                    asset_root.handles = handles.clone();
                }
            },
        )
        .add_observer(|_: Trigger<OnPointerUpFlush>, mut commands: Commands| {
            commands.queue(|world: &mut World| {
                if let Some(mut handlers) = world.remove_resource::<OnPointerUpHandlers>() {
                    for mut handler in handlers.0.drain(..) {
                        handler();
                    }
                }
            })
        });
}
