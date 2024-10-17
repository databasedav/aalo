use std::{
    collections::{HashMap, HashSet, VecDeque},
    convert::identity,
    fmt::Display,
    i32,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy::{
    ecs::{
        archetype::Archetypes,
        component::{ComponentHooks, ComponentId, Components, StorageType},
        entity::Entities,
        system::{BoxedSystem, SystemId, SystemState},
    },
    input::mouse::MouseWheel,
    prelude::*,
    reflect::{
        Access, DynamicEnum, DynamicStruct, DynamicTuple, DynamicVariant, Enum, OffsetAccess,
        ParsedPath, ReflectKind, ReflectMut, ReflectRef, TypeInfo, TypeRegistry, VariantInfo,
    },
};
use bevy_cosmic_edit::{
    cosmic_text::{Action, Edit, Family, FamilyOwned, Motion, Selection},
    CosmicBackgroundColor, CosmicEditor, CosmicFontSystem, CosmicTextAlign, CosmicWrap,
    CursorColor, FontWeight, MaxLines, ScrollDisabled, SelectionColor,
};
use bevy_mod_picking::{
    events::{Click, Drag, DragEnd, DragStart, Pointer},
    prelude::PointerButton,
};
use haalka::{
    align::AlignabilityFacade,
    prelude::*,
    raw::{utils::flush_deferred_updaters, HaalkaObserver, HaalkaOneShotSystem},
    text_input::{FocusedTextInput, TextInputFocusOnDownDisabled},
    viewport_mutable::{MutableViewport, OnViewportLocationChange},
};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher,
};
use num::Bounded;
use disqualified::ShortName;

use super::{defaults::*, globals::*, reflect::*, style::*, utils::*, widgets::*};
use crate::{impl_syncers, signal_or};

// TODO: scrolling parent context, see vscode
// TODO: searching
// TODO: should error highlight + affect ordering when enum variant does not have default impl
// TODO: on hover tooltips, all errors should be hoverable with tooltips
// TODO: unit struct handling (a tooltip with "unit struct" should suffice)
// TODO: separate groups for entities, resources, assets, observers, and counters for haalka and aalo systems with tooltips saying they can't be expanded because that would cause infinite recursion
// TODO: custom views for vector/matrix types
// TODO: runtime targeting
// TODO: drag handle
// TODO: optional title
// TODO: only sync components/execute field listeners when in view
// TODO: `Name` component syncing
// TODO: implement frontend for at least all ui node types; how abt char, str, unit ? test
// TODO: scrollbars

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
// TODO: inspector entities appear above resize borders, prolly just wait for https://github.com/bevyengine/bevy/issues/14773
// TODO: dropdowns cannot extend past bounds of inspector

#[derive(Clone, Default)]
pub struct EntityData {
    pub name: Mutable<Option<String>>,
    pub expanded: Mutable<bool>,
    pub filtered: Mutable<bool>,
    pub components: MutableBTreeMap<ComponentId, ComponentData>,
    components_transformers:
        Arc<Mutex<Vec<Box<dyn FnMut(ComponentsSignalVec) -> ComponentsSignalVec + Send>>>>,
}

/// Entities without a `Parent`.
pub static ORPHAN_ENTITIES: Lazy<MutableBTreeMap<Entity, EntityData>> = Lazy::new(default);

/// Entities with a `Parent`.
pub static ENTITIES: Lazy<MutableBTreeMap<Entity, EntityData>> = Lazy::new(default);

pub struct Search {
    pub search: Mutable<String>,
    pub fuzzy: Mutable<bool>,
}

type EntitySignalVec = std::pin::Pin<Box<dyn SignalVec<Item = (Entity, EntityData)> + Send>>;
type ComponentsSignalVec =
    std::pin::Pin<Box<dyn SignalVec<Item = (ComponentId, ComponentData)> + Send>>;

/// Configuration frontend for entity inspecting elements.
#[derive(Default)]
pub struct EntityInspector {
    column: Column<NodeBundle>,
    wrapper_stack: Stack<NodeBundle>,
    entities: MutableBTreeMap<Entity, EntityData>,
    entities_transformers: Vec<Box<dyn FnMut(EntitySignalVec) -> EntitySignalVec>>,
    components_transformers: Vec<Box<dyn FnMut(ComponentsSignalVec) -> ComponentsSignalVec + Send>>,
    search: Option<Search>,
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
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    border_color: Mutable<Color>,
    scroll_pixels: Mutable<f32>,
    unnest_children: bool,
}

#[derive(Component)]
struct EntityInspectorMarker;

impl ElementWrapper for EntityInspector {
    type EL = Stack<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.wrapper_stack
    }

    fn into_el(self) -> Self::EL {
        let Self {
            column,
            wrapper_stack,
            entities,
            entities_transformers,
            components_transformers,
            search,
            font_size,
            row_gap,
            column_gap,
            primary_background_color,
            secondary_background_color,
            border_color,
            border_width,
            padding,
            border_radius,
            height,
            width,
            highlighted_color,
            unhighlighted_color,
            unnest_children,
            ..
        } = self;
        let unnest_children = Mutable::new(unnest_children);
        let components_transformers = Arc::new(Mutex::new(components_transformers));
        let mut tasks = vec![];
        if let Some(Search { search, fuzzy }) = search {
            let task = clone!((entities) map_ref! {
                let search = search.signal_cloned(),
                let &fuzzy = fuzzy.signal() => move {
                    if !search.is_empty() {
                        let ref mut matcher = Matcher::new(Config::DEFAULT);
                        let atom = Pattern::new(&search, CaseMatching::Ignore, Normalization::Smart, if fuzzy { nucleo_matcher::pattern::AtomKind::Fuzzy } else { nucleo_matcher::pattern::AtomKind::Substring });
                        for (_, EntityData { name: name_option, filtered, .. }) in entities.lock_ref().iter() {
                            filtered.set_neq(
                                if let Some(name) = &*name_option.lock_ref() {
                                    atom.score(nucleo_matcher::Utf32String::from(name.as_str()).slice(..), matcher).is_none()
                                } else {
                                    true
                                }
                            )
                        }
                    }
                }
            }).to_future().apply(spawn);
            tasks.push(task);
        }
        column
            .update_raw_el(move |raw_el| {
                raw_el
                    .hold_tasks(tasks)
                    .insert(EntityInspectorMarker)
                    .insert(OnViewportLocationChange)
                    .component_signal::<SyncEntities, _>(unnest_children.signal().map_true(default))
                    .component_signal::<SyncOrphanEntities, _>(
                        unnest_children.signal().map_false(default),
                    )
            })
            .mutable_viewport(Overflow::clip(), LimitToBody::Both)
            .on_scroll_with_system_on_hover(
                BasicScrollHandler::new()
                    .direction(ScrollDirection::Both)
                    .pixels_signal(self.scroll_pixels.signal().dedupe())
                    .into_system(),
            )
            .items_signal_vec({
                let mut signal_vec = entities.entries_cloned().boxed();
                signal_vec = signal_vec
                    .map(move |mut data| {
                        data.1.components_transformers = components_transformers.clone();
                        data
                    })
                    .boxed();
                for mut f in entities_transformers {
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
                        EntityElement::new(id, data)
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
            })
            .apply(flush_deferred_updaters)
            .apply(El::<NodeBundle>::from)
            .apply(resize_border(
                border_width.signal(),
                border_radius.signal(),
                border_color.signal(),
                highlighted_color.signal(),
                Some(wrapper_stack),
            ))
            .apply(background_style(primary_background_color.signal()))
            .cursor(CursorIcon::Default)
    }
}

impl Sizeable for EntityInspector {}

impl EntityInspector {
    pub fn new() -> Self {
        Self {
            column: Column::<NodeBundle>::new(),
            wrapper_stack: Stack::<NodeBundle>::new(),
            entities: MutableBTreeMap::new(),
            entities_transformers: vec![],
            components_transformers: vec![],
            search: None,
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
            highlighted_color: GLOBAL_HIGHLIGHTED_COLOR.clone(),
            unhighlighted_color: GLOBAL_UNHIGHLIGHTED_COLOR.clone(),
            border_color: GLOBAL_BORDER_COLOR.clone(),
            scroll_pixels: GLOBAL_SCROLL_PIXELS.clone(),
            unnest_children: false,
        }
    }

    pub fn entities(mut self, mut entities: MutableBTreeMap<Entity, EntityData>) -> Self {
        std::mem::swap(&mut self.entities, &mut entities);
        self
    }

    pub fn with_entities(
        mut self,
        f: impl FnMut(EntitySignalVec) -> EntitySignalVec + 'static,
    ) -> Self {
        self.entities_transformers.push(Box::new(f));
        self
    }

    pub fn with_components(
        mut self,
        f: impl FnMut(ComponentsSignalVec) -> ComponentsSignalVec + Send + 'static,
    ) -> Self {
        self.components_transformers.push(Box::new(f));
        self
    }

    pub fn search(mut self) -> Self {
        self.search = Some(Search {
            search: Mutable::new(String::new()),
            fuzzy: Mutable::new(true),
        });
        self
    }

    pub fn jump_to(self, target: impl Into<InspectionTarget>) -> Self {
        let target = target.into();
        self.update_raw_el(move |raw_el| {
            raw_el.with_entity(|mut entity| {
                if let Some(mut inspection_targets) = entity.get_mut::<InspectionTargets>() {
                    inspection_targets.0.push(target);
                } else {
                    entity.insert(InspectionTargets(vec![target]));
                }
            })
        })
    }

    pub fn unnest_children(mut self) -> Self {
        self.unnest_children = true;
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
        highlighted_color: Color,
        unhighlighted_color: Color,
        border_color: Color,
        scroll_pixels: f32,
    }
}

#[derive(Clone, Default)]
pub struct ComponentData {
    pub name: String,
    pub expanded: Mutable<bool>,
    viewable: Mutable<bool>,
}

#[derive(Component)]
struct EntityRoot {
    entity: Entity, // target
    components: HashSet<ComponentId>,
}

#[derive(Event)]
struct ComponentsAdded(Vec<ComponentId>);

#[derive(Event)]
struct ComponentsRemoved(Vec<ComponentId>);

struct EntityElement {
    el: Column<NodeBundle>,
    entity: Entity,
    entity_data: EntityData,
    show_name: bool,
    font_size: Mutable<f32>,
    row_gap: Mutable<f32>,
    column_gap: Mutable<f32>,
    primary_background_color: Mutable<Color>,
    secondary_background_color: Mutable<Color>,
    border_width: Mutable<f32>,
    border_color: Mutable<Color>,
    padding: Mutable<f32>,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    expanded: Mutable<bool>,
}

impl EntityElement {
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

    fn new(entity: Entity, entity_data: EntityData) -> Self {
        let font_size = Mutable::new(DEFAULT_FONT_SIZE);
        let row_gap = Mutable::new(DEFAULT_ROW_GAP);
        let column_gap = Mutable::new(DEFAULT_COLUMN_GAP);
        let primary_background_color = Mutable::new(DEFAULT_PRIMARY_BACKGROUND_COLOR);
        let secondary_background_color = Mutable::new(DEFAULT_SECONDARY_BACKGROUND_COLOR);
        let border_width = Mutable::new(DEFAULT_BORDER_WIDTH);
        let border_color = Mutable::new(DEFAULT_BORDER_COLOR);
        let padding = Mutable::new(DEFAULT_PADDING);
        let highlighted_color = Mutable::new(DEFAULT_HIGHLIGHTED_COLOR);
        let unhighlighted_color = Mutable::new(DEFAULT_UNHIGHLIGHTED_COLOR);
        Self {
            el: Column::<NodeBundle>::new(),
            expanded: entity_data.expanded.clone(),
            entity,
            entity_data,
            show_name: false,
            font_size,
            row_gap,
            column_gap,
            primary_background_color,
            secondary_background_color,
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
struct SyncComponents;

#[derive(Component, Default)]
struct Expanded;

#[derive(Component)]
struct LastExpandedHeader(Mutable<bool>);

impl ElementWrapper for EntityElement {
    type EL = Column<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el,
            entity,
            entity_data:
                EntityData {
                    name,
                    expanded,
                    filtered,
                    components,
                    components_transformers,
                },
            show_name,
            font_size,
            row_gap,
            column_gap,
            primary_background_color,
            secondary_background_color,
            border_width,
            border_color,
            padding,
            highlighted_color,
            unhighlighted_color,
            ..
        } = self;
        let name_option = name.get_cloned();
        el
        .update_raw_el(clone!((components, expanded) move |raw_el| {
            raw_el
            .insert(EntityRoot { entity, components: HashSet::from_iter(components.lock_ref().iter().map(|(&id, _)| id))})
            .component_signal::<Expanded, _>(expanded.signal().map_true(default))
            .on_spawn_with_system(clone!((expanded) move |In(ui_entity): In<Entity>, parents: Query<&Parent>, inspection_targets: Query<&InspectionTargets>, mut commands: Commands| {
                for parent in parents.iter_ancestors(ui_entity) {
                    if let Ok(InspectionTargets(inspection_targets)) = inspection_targets.get(parent) {
                        let entity_string = entity.to_string();
                        let index = [Some(&entity_string), name_option.as_ref()];
                        for target in inspection_targets {
                            if index.contains(&Some(&target.entity)) {
                                if let Some(mut entity) = commands.get_entity(ui_entity) {
                                    let mut pending = VecDeque::new();
                                    if let Some(component) = &target.component {
                                        pending.push_back(ProgressPart::Component(component.clone()));
                                        if let Some(path) = &target.path {
                                            for OffsetAccess { access, .. } in &path.0 {
                                                pending.push_back(ProgressPart::Access(access.clone()));
                                            }
                                        }
                                    }
                                    entity.try_insert(InspectionTargetProgress { target: target.clone(), pending });
                                }
                                expanded.set_neq(true);
                                return
                            }
                        }
                    }
                }
            }))
            .observe(clone!((components => components_map) move |event: Trigger<ComponentsAdded>, components: &Components| {
                let ComponentsAdded(added) = event.event();
                let mut lock = components_map.lock_mut();
                for &component in added {
                    if let Some(info) = components.get_info(component) {
                        lock.insert_cloned(component, ComponentData { name: info.name().to_string(), viewable: Mutable::new(false), expanded: Mutable::new(false) });
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
            // TODO: only sync when in view
            .component_signal::<SyncComponents, _>(expanded.signal().map_true(default))
        }))
        .width(Val::Percent(100.))
        .item(show_name.then(|| {
            let last_expanded_header = Mutable::new(false);
            let hovered = Mutable::new(false);
            El::<NodeBundle>::new()
            .width(Val::Percent(100.))
            .cursor(CursorIcon::Pointer)
            .apply(padding_style(BoxEdge::VERTICAL, row_gap.signal().map(|row_gap| row_gap / 2.)))
            .apply(padding_style(BoxEdge::HORIZONTAL, padding.signal()))
            .apply(background_style(primary_background_color.signal()))
            .hovered_sync(hovered.clone())
            .child(
                HighlightableText::new()
                .align(Align::new().center_y())
                .highlighted_signal(hovered.signal())
                .with_text(clone!((font_size) move |text| {
                    text
                    .text_signal(name.signal_cloned().map_option(identity, || "Entity".to_string()).map(move |prefix| format!("{prefix} ({entity})")))
                    .font_size_signal(font_size.signal())
                }))
                .highlighted_color_signal(highlighted_color.signal())
                .unhighlighted_color_signal(unhighlighted_color.signal())
            )
            .on_click_with_system(clone!((expanded) move |In((entity, _)), mut styles: Query<&mut Style>| {
                flip(&expanded);
                if let Ok(mut style) = styles.get_mut(entity) {
                    style.top = Val::Px(0.);
                }
            }))
            // TODO: shouldn't use El here but Stack is being weird with heights
            .child_signal(
                last_expanded_header
                .signal()
                .map_true(|| {
                    El::<NodeBundle>::new()
                    .height(Val::Px(4.))
                    .width(Val::Percent(120.))
                    .with_style(|mut style| {
                        style.position_type = PositionType::Absolute;
                        style.top = Val::Px(18. + 5.);  // TODOTODO: these should not be hardcoded
                        style.left = Val::Px(-20.);
                    })
                    .apply(background_style(always(Color::BLACK.with_alpha(0.5))))
                })
            )
            .update_raw_el(|raw_el| raw_el.insert(LastExpandedHeader(last_expanded_header)))
        }))
        .item_signal(if show_name { expanded.signal().boxed() } else { always(true).boxed() }.map_true(clone!((row_gap, column_gap, secondary_background_color, border_width, border_color, padding, highlighted_color, unhighlighted_color) move || {
            Column::<NodeBundle>::new()
                .apply(move_style(Move::Right, padding.signal()))
                .apply(horizontal_padding_style(padding.signal()))
                .apply(left_bordered_style(border_width.signal(), border_color.signal()))
                .items_signal_vec({
                    let mut signal_vec = components.entries_cloned().boxed();
                    for f in components_transformers.lock().unwrap().iter_mut() {
                        signal_vec = f(signal_vec);
                    }
                    signal_vec
                    // this is an emulation of something like .sort_by_signal_cloned
                    .map_signal(|(component, data)| {
                        data.viewable.signal().map(move |cur| (component, data.clone(), cur))
                    })
                    .sort_by_cloned(|(_, ComponentData { name: left_name, .. }, left_viewable), (_, ComponentData { name: right_name, .. }, right_viewable)| left_viewable.cmp(right_viewable).reverse().then(left_name.cmp(right_name)))
                    .map(clone!((row_gap, column_gap, secondary_background_color, border_width, border_color, padding, highlighted_color, unhighlighted_color) move |(component, ComponentData { name, expanded, viewable }, _)| {
                        FieldElement::new(entity, component, FieldType::Component(name), viewable)
                        .row_gap_signal(row_gap.signal())
                        .column_gap_signal(column_gap.signal())
                        .type_path_color_signal(secondary_background_color.signal())
                        .border_width_signal(border_width.signal())
                        .border_color_signal(border_color.signal())
                        .padding_signal(padding.signal())
                        .highlighted_color_signal(highlighted_color.signal())
                        .unhighlighted_color_signal(unhighlighted_color.signal())
                        .expanded_signal(expanded.signal())
                    }))
                })
        })))
    }
}

#[derive(Clone)]
enum FieldType {
    Component(String),
    Access(Access<'static>),
}

#[derive(Clone)]
struct AccessFieldData {
    access: Access<'static>,
    viewable: Mutable<bool>,
}

impl AccessFieldData {
    fn new(access: Access<'static>) -> Self {
        Self {
            access,
            viewable: Mutable::new(false),
        }
    }
}

struct FieldElement {
    el: Column<NodeBundle>,
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
    type EL = Column<NodeBundle>;
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
struct EnumData {
    variants: Vec<&'static str>,
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

#[derive(Component)]
struct DroppedDown(Mutable<bool>);

impl FieldElement {
    fn new(
        entity: Entity,
        component: ComponentId,
        field_type: FieldType,
        viewable: Mutable<bool>,
    ) -> Self {
        let row_gap = Mutable::new(DEFAULT_ROW_GAP);
        let column_gap = Mutable::new(DEFAULT_COLUMN_GAP);
        let border_width = Mutable::new(DEFAULT_BORDER_WIDTH);
        let border_color = Mutable::new(DEFAULT_BORDER_COLOR);
        let padding = Mutable::new(DEFAULT_PADDING);
        let highlighted_color = Mutable::new(DEFAULT_HIGHLIGHTED_COLOR);
        let unhighlighted_color = Mutable::new(DEFAULT_UNHIGHLIGHTED_COLOR);
        let type_path_color = Mutable::new(DEFAULT_SECONDARY_BACKGROUND_COLOR);
        let expanded = Mutable::new(false);
        let (name, access_option) = match field_type.clone() {
            FieldType::Component(type_path) => {
                (ShortName(&type_path).to_string(), None)
            }
            FieldType::Access(access) => (access.to_string(), Some(access.clone())),
        };
        let type_path = Mutable::new(None);
        let node_type = Mutable::new(None);
        let enum_data_option = Mutable::new(None);
        let el = Column::<NodeBundle>::new()
            .update_raw_el(|raw_el| {
                raw_el
                .on_spawn_with_system(clone!((expanded, field_type) move |In(ui_entity): In<Entity>, parents: Query<&Parent>, progresses: Query<&InspectionTargetProgress>, mut commands: Commands| {
                    for parent in parents.iter_ancestors(ui_entity) {
                        if let Ok(InspectionTargetProgress { target, pending }) = progresses.get(parent) {
                            if let Some(first) = pending.front() {
                                if match (first, field_type.clone()) {
                                    (ProgressPart::Component(target_component), FieldType::Component(component)) => {
                                        target_component == &component
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
                                    if let Some(mut entity) = commands.get_entity(ui_entity) {
                                        let mut pending = pending.clone();
                                        pending.pop_front();
                                        if pending.is_empty() {
                                            entity.commands().trigger(RemoveTarget { target: target.clone(), from: ui_entity });
                                            // TODO: this should work, but scrolling is initiated before all elements can be fully rendered,
                                            // the synchronous alternative to this would be to somehow have a way to wait for all elements above 
                                            // to fully render, including recursive signals outputs, not sure how to do that ...
                                            // let system = Box::new(IntoSystem::into_system(|In(entity): In<Entity>, mut commands: Commands| {
                                            //     commands.trigger(ScrollTo(entity))
                                            // }));
                                            // entity.try_insert(AfterNodely(system));
                                            async move {
                                                sleep(Duration::from_millis(200)).await;  // TODO: this wait should be configurable, mostly cuz some ppl might need *more* time
                                                async_world().apply(move |world: &mut World| {
                                                    if let Some(mut entity) = world.get_entity_mut(ui_entity) {
                                                        let system = Box::new(IntoSystem::into_system(|In(entity): In<Entity>, mut commands: Commands| {
                                                            commands.trigger(ScrollTo(entity))
                                                        }));
                                                        entity.insert(AfterNodely(system));
                                                    }
                                                }).await;
                                            }
                                            .apply(spawn).detach();
                                        } else {
                                            entity.try_insert(InspectionTargetProgress { target: target.clone(), pending });
                                        }
                                    }
                                    expanded.set_neq(true);
                                }
                            }
                            return
                        }
                    }
                }))
                .on_spawn(clone!((viewable, expanded, node_type, type_path, enum_data_option, field_type) move |world, ui_entity| {
                    let mut field_path_option = None;
                    match field_type {
                        FieldType::Component(_) => {
                            let mut system_state = SystemState::<(&Components, Res<AppTypeRegistry>)>::new(world);
                            let (components, type_registry) = system_state.get(world);
                            let type_registry = type_registry.read();
                            if let Some(info) = components.get_info(component) {
                                viewable.set_neq(info.type_id().and_then(|type_id| type_registry.get(type_id)).is_some());
                            }
                        },
                        FieldType::Access(access) => {
                            let mut system_state = SystemState::<(Query<&Accessory>, Query<&Parent>, Query<&EntityRoot>, ResMut<FieldPathCache>)>::new(world);
                            let (accessories, parents, sync_components, ref mut field_path_cache) = system_state.get_mut(world);
                            let mut field_path = field_path_cached(ui_entity, &accessories, &parents, &sync_components, field_path_cache);
                            field_path.0.push(access.into());
                            field_path_option = Some(field_path);
                        },
                    }
                    if let Some(mut reflect) = reflect(world, entity, component) {
                        if let Some(path) = field_path_option {
                            if let Ok(result) = reflect.reflect_path(&path) {
                                reflect = result;
                            }
                        }
                        type_path.set(Some(reflect.reflect_type_path().to_string()));
                        let mut set_viewable = false;
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
                                set_viewable = true;
                            },
                            ReflectRef::TupleStruct(tuple_struct) => {
                                let mut fields = vec![];
                                for i in 0..tuple_struct.field_len() {
                                    let access = Access::TupleIndex(i);
                                    fields.push(AccessFieldData::new(access));
                                }
                                node_type.set(Some(NodeType::Multi { items: fields.into(), size_dynamic: None }));
                                set_viewable = true;
                            },
                            ReflectRef::Tuple(tuple) => {
                                let mut fields = vec![];
                                for i in 0..tuple.field_len() {
                                    let access = Access::TupleIndex(i);
                                    fields.push(AccessFieldData::new(access));
                                }
                                node_type.set(Some(NodeType::Multi { items: fields.into(), size_dynamic: None }));
                                set_viewable = true;
                            },
                            ReflectRef::List(list) => {
                                let mut fields = vec![];
                                for i in 0..list.len() {
                                    let access = Access::ListIndex(i);
                                    fields.push(AccessFieldData::new(access));
                                }
                                node_type.set(Some(NodeType::Multi { items: fields.into(), size_dynamic: Some(ReflectKind::List) }));
                                set_viewable = true;
                            },
                            ReflectRef::Array(array) => {
                                let mut fields = vec![];
                                for i in 0..array.len() {
                                    let access = Access::ListIndex(i);
                                    fields.push(AccessFieldData::new(access));
                                }
                                node_type.set(Some(NodeType::Multi { items: fields.into(), size_dynamic: None }));
                                set_viewable = true;
                            },
                            ReflectRef::Map(map) => {
                                // TODO: might require adding map support to Access ?
                            },
                            ReflectRef::Enum(enum_) => {
                                if let Some(TypeInfo::Enum(enum_info)) = enum_.get_represented_type_info() {
                                    enum_data_option.set(Some(
                                        EnumData {
                                            variants: enum_info.variant_names().into_iter().map(std::ops::Deref::deref).collect::<Vec<_>>(),
                                        }
                                    ));
                                    set_viewable = true;
                                    expanded.set_neq(true);
                                }
                            },
                            ReflectRef::Value(value) => {
                                let type_path = value.reflect_type_path();
                                node_type.set(Some(NodeType::Solo(type_path.to_string())));
                                expanded.set_neq(true);
                            },
                        }
                        if set_viewable {
                            viewable.set_neq(true);
                        }
                    }
                }))
            })
            .item({
                let hovered = Mutable::new(false);
                Row::<NodeBundle>::new()
                .with_style(|mut style| style.width = Val::Percent(100.))
                .apply(padding_style(BoxEdge::VERTICAL, row_gap.signal().map(|row_gap| row_gap / 2.)))
                .apply(row_style(column_gap.signal()))
                .on_click(clone!((expanded, viewable) move || {
                    if viewable.get() {
                        flip(&expanded)
                    }
                }))
                .cursor_disableable_signal(CursorIcon::Pointer, signal::not(viewable.signal()))
                .hovered_sync(hovered.clone())
                .item_signal(
                    viewable.signal().map_bool(
                    clone!((name, highlighted_color, unhighlighted_color, hovered) move || HighlightableText::new().with_text(|text| text.text(name.clone()))
                        .highlighted_color_signal(highlighted_color.signal())
                        .unhighlighted_color_signal(unhighlighted_color.signal())
                        .highlighted_signal(hovered.signal())
                        .type_erase()),
                    move || DynamicText::new()
                        .text(name.clone())
                        .color(bevy::color::palettes::basic::MAROON.into())
                        .type_erase(),
                    )
                    .map(|el| el.align(Align::new().top()))
                )
                .item_signal(
                    if let FieldType::Component(type_path) = field_type {
                        hovered.signal()
                        .map_true(clone!((type_path_color, type_path) move || {
                            DynamicText::new()
                            .text(type_path.clone())
                            .color_signal(type_path_color.signal())
                        }))
                        .boxed()
                    } else {
                        type_path.signal_cloned().map_some(clone!((type_path_color) move |type_path| {
                            DynamicText::new()
                            .text_signal(hovered.signal().map_bool(clone!((type_path) move || type_path.clone()), move || ShortName(&type_path).to_string()))
                            .color_signal(type_path_color.signal())
                        }))
                        .boxed()
                    }
                )
            })
            .item_signal(expanded.signal().map_true(
                clone!((
                    row_gap,
                    border_width,
                    border_color,
                    padding,
                    highlighted_color,
                    unhighlighted_color,
                    type_path_color,
                    viewable,
                    enum_data_option
                ) move || {
                    Column::<NodeBundle>::new()
                    .apply(padding_style(BoxEdge::HORIZONTAL, padding.signal()))
                    .apply(left_bordered_style(border_width.signal(), border_color.signal()))
                    .item_signal(
                        enum_data_option.signal_cloned()
                        .map_some(clone!((access_option, node_type, row_gap) move |EnumData { variants }| {
                            let options = variants.into_iter().map(Into::into).collect::<Vec<_>>().into();
                            let selected = Mutable::new(None);
                            let show_dropdown = Mutable::new(false);
                            let dropdown_entity = Mutable::new(None);
                            Dropdown::new(options)
                            .on_click_outside(clone!((show_dropdown) move || show_dropdown.set_neq(false)))
                            .with_show_dropdown(show_dropdown.clone())
                            .apply(padding_style(BoxEdge::VERTICAL, row_gap.signal().map(|row_gap| row_gap / 2.)))
                            .update_raw_el(clone!((access_option, selected, dropdown_entity, node_type) move |raw_el| {
                                raw_el
                                .insert(Accessory { entity, component, access_option })
                                .with_entity(clone!((selected, node_type) move |mut entity| {
                                    dropdown_entity.set_neq(Some(entity.id()));
                                    let handler = entity.world_scope(move |world| {
                                        register_system(world, clone!((selected, node_type) move |In(reflect): In<Box<dyn Reflect>>| {
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
                                In(i),
                                accessories: Query<&Accessory>,
                                parents: Query<&Parent>,
                                sync_components: Query<&EntityRoot>,
                                mut field_path_cache: ResMut<FieldPathCache>,
                                type_registry: Res<AppTypeRegistry>,
                                mut commands: Commands| {
                                    let ui_entity = dropdown_entity.get().unwrap();
                                    if let Ok(Accessory { entity, component, .. }) = accessories.get(ui_entity).cloned() {
                                        let field_path = field_path_cached(ui_entity, &accessories, &parents, &sync_components, &mut field_path_cache);
                                        let type_registry = type_registry.0.clone();
                                        commands.add(clone!((node_type) move |world: &mut World| {
                                            with_reflect_mut(world, entity, component, |reflect| {
                                                if let Ok(target) = reflect.reflect_path_mut(&field_path) {
                                                    if let ReflectMut::Enum(enum_) = target.reflect_mut() {
                                                        if let Some(variant_info) = get_variant_info(enum_, i) {
                                                            if let Some(default) = variant_default_value(variant_info, &type_registry.read()) {
                                                                populate_enum_with_variant(enum_, i, &node_type);
                                                                target.apply(&default);
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                        }));
                                    }
                                    show_dropdown.set_neq(false);
                            }))
                            .into_el()
                            .z_index(ZIndex::Local(i32::MAX))
                        }))
                    )
                    .item_signal(
                        node_type.signal_cloned()
                        .map_some(clone!((row_gap, border_width, border_color, padding, highlighted_color, unhighlighted_color, type_path_color, viewable) move |node_type| match node_type {
                            NodeType::Solo(type_path) => {
                                let el_option = field(&type_path).map(TypeEraseable::type_erase);
                                if el_option.is_some() {
                                    viewable.set_neq(true);
                                }
                                el_option
                                .map(|el| el.apply(padding_style(BoxEdge::VERTICAL, row_gap.signal().map(|row_gap| row_gap / 2.))))
                            },
                            NodeType::Multi { items, size_dynamic } => {
                                Column::<NodeBundle>::new()
                                .update_raw_el(clone!((items) move |mut raw_el| {
                                    if let Some(reflect_kind) = size_dynamic {
                                        raw_el = raw_el.with_entity(move |mut entity| {
                                            let handler = entity.world_scope(|world| {
                                                register_system(world, move |In(reflect): In<Box<dyn Reflect>>| {
                                                    match reflect.reflect_ref() {
                                                        bevy::reflect::ReflectRef::List(list) => {
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
                                                        _ => ()
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
                                    .map(clone!((row_gap, border_width, border_color, padding, highlighted_color, unhighlighted_color, type_path_color) move |AccessFieldData { access, viewable }| {
                                        FieldElement::new(entity, component, FieldType::Access(access.clone()), viewable)
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
                        .map(clone!((access_option, row_gap) move |mut el_option| {
                            el_option = el_option
                            .map(|el| el.apply(padding_style(BoxEdge::VERTICAL, row_gap.signal().map(|row_gap| row_gap / 2.))))
                            .map(clone!((access_option) move |el| el.update_raw_el(|raw_el| raw_el.insert(Accessory { entity, component, access_option }))));
                            el_option
                        }))
                    )
                })
            ));
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

pub(crate) static CUSTOM_FIELD_FUNCTIONS: Lazy<
    Vec<Box<dyn Fn(&str) -> Option<AlignabilityFacade> + Send + Sync + 'static>>,
> = Lazy::new(default);

fn field(type_path: &str) -> Option<impl Element> {
    for f in CUSTOM_FIELD_FUNCTIONS.iter() {
        if let Some(el) = f(type_path) {
            return Some(el);
        }
    }
    match type_path {
        "bool" => bool_field().type_erase().apply(Some),
        "isize" => numeric_field::<isize>().type_erase().apply(Some),
        "i8" => numeric_field::<i8>().type_erase().apply(Some),
        "i16" => numeric_field::<i16>().type_erase().apply(Some),
        "i32" => numeric_field::<i32>().type_erase().apply(Some),
        "i64" => numeric_field::<i64>().type_erase().apply(Some),
        "i128" => numeric_field::<i128>().type_erase().apply(Some),
        "usize" => numeric_field::<usize>().type_erase().apply(Some),
        "u8" => numeric_field::<u8>().type_erase().apply(Some),
        "u16" => numeric_field::<u16>().type_erase().apply(Some),
        "u32" => numeric_field::<u32>().type_erase().apply(Some),
        "u64" => numeric_field::<u64>().type_erase().apply(Some),
        "u128" => numeric_field::<u128>().type_erase().apply(Some),
        "f32" => numeric_field::<f32>().type_erase().apply(Some),
        "f64" => numeric_field::<f64>().type_erase().apply(Some),
        "alloc::string::String" => string_field().type_erase().apply(Some),
        "alloc::borrow::Cow<str>" => string_field().type_erase().apply(Some),
        "bevy_ecs::entity::Entity" => entity_field().type_erase().apply(Some),
        _ => None,
    }
}

fn field_path(
    entity: Entity, // field's ui entity
    accessories: &Query<&Accessory>,
    parents: &Query<&Parent>,
    sync_components: &Query<&EntityRoot>,
) -> ParsedPath {
    let mut path = vec![];
    for parent in [entity].into_iter().chain(parents.iter_ancestors(entity)) {
        if let Ok(Accessory { access_option, .. }) = accessories.get(parent) {
            if let Some(access) = access_option {
                path.push(access.clone());
            }
        }
        // marks entity root
        if sync_components.contains(parent) {
            break;
        }
    }
    path.reverse();
    ParsedPath::from(path)
}

fn field_path_cached(
    entity: Entity, // field's ui entity
    accessories: &Query<&Accessory>,
    parents: &Query<&Parent>,
    entity_roots: &Query<&EntityRoot>,
    field_path_cache: &mut ResMut<FieldPathCache>,
) -> ParsedPath {
    if let Some(field_path) = field_path_cache.0.get(&entity) {
        field_path.clone()
    } else {
        let field_path = field_path(entity, accessories, parents, entity_roots);
        field_path_cache.0.insert(entity, field_path.clone());
        field_path
    }
}

// adapted from Quill https://github.com/viridia/quill/blob/cecbc35426a095f56bad1f12df546f5a79dece32/crates/bevy_quill_obsidian_inspect/src/inspectors/enum.rs#L171
fn variant_default_value(variant: &VariantInfo, registry: &TypeRegistry) -> Option<DynamicEnum> {
    match variant {
        bevy::reflect::VariantInfo::Struct(struct_) => {
            let mut dynamic_struct = DynamicStruct::default();
            for field in struct_.iter() {
                if let Some(field_type_default) =
                    registry.get_type_data::<ReflectDefault>(field.type_id())
                {
                    dynamic_struct.insert_boxed(field.name(), field_type_default.default());
                } else {
                    return None;
                }
            }
            Some(DynamicEnum::new(variant.name(), dynamic_struct))
        }
        bevy::reflect::VariantInfo::Tuple(tpl) => {
            let mut dynamic_tuple = DynamicTuple::default();
            for field in tpl.iter() {
                if let Some(field_type_default) =
                    registry.get_type_data::<ReflectDefault>(field.type_id())
                {
                    dynamic_tuple.insert_boxed(field_type_default.default());
                } else {
                    return None;
                }
            }
            Some(DynamicEnum::new(variant.name(), dynamic_tuple))
        }
        bevy::reflect::VariantInfo::Unit(_) => {
            Some(DynamicEnum::new(variant.name(), DynamicVariant::Unit))
        }
    }
}

fn bool_field() -> impl Element {
    let checked: Mutable<bool> = Mutable::new(false);
    Checkbox::new()
        .checked_signal(checked.signal())
        .on_click_with_system(clone!((checked) move |In((ui_entity, click)): In<(Entity, Pointer<Click>)>,
         accessories: Query<&Accessory>,
         parents: Query<&Parent>,
         sync_components: Query<&EntityRoot>,
         mut field_path_cache: ResMut<FieldPathCache>,
         mut commands: Commands| {
            if matches!(click.button, PointerButton::Primary) {
                if let Ok(Accessory { entity, component, .. }) = accessories.get(ui_entity).cloned() {
                    let field_path = field_path_cached(ui_entity, &accessories, &parents, &sync_components, &mut field_path_cache);
                    commands.add(clone!((checked) move |world: &mut World| {
                        with_reflect_mut(world, entity, component, |reflect| {
                            if let Ok(target) = reflect.reflect_path_mut(&field_path) {
                                let _ = target.try_apply((!checked.get()).as_reflect());
                            }
                        });
                    }));
                }
            }
        }))
        .update_raw_el(|raw_el| {
            raw_el.with_entity(|mut entity| {
                let handler = entity.world_scope(|world| {
                    register_system(world, move |In(reflect): In<Box<dyn Reflect>>| {
                        if let Ok(cur) = reflect.downcast::<bool>() {
                            checked.set_neq(*cur);
                        }
                    })
                });
                entity.insert(FieldListener { handler });
            })
        })
}

fn entity_field() -> impl Element {
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
    El::<NodeBundle>::new()
        .update_raw_el(clone!((name, entity_holder) move |raw_el| {
            raw_el.on_spawn(move |world, entity| {
                let mut system_state = SystemState::<Query<DebugName>>::new(world);
                let debug_names = system_state.get(world);
                if let Some(debug_name) =
                    debug_names.get(entity).ok().and_then(|name| name.name)
                {
                    name.set(Some(debug_name.to_string()));
                }
            })
            .with_entity(move |mut entity| {
                let handler = entity.world_scope(|world| {
                    register_system(world, move |In(reflect): In<Box<dyn Reflect>>| {
                        if let Ok(cur) = reflect.downcast::<Entity>() {
                            entity_holder.set_neq(Some(*cur));
                        }
                    })
                });
                entity.insert(FieldListener { handler });
            })
        }))
        .child_signal(entity_holder.signal().map_some(move |entity| {
            EntityElement::new(entity, entity_data.clone())
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

// TODO: but y tho
const TEXT_INPUT_FONT_SIZE_MULTIPLIER: f32 = 16. / 18.;

fn text_input_font_size_style(
    font_size: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(TextInput) -> TextInput {
    move |el| {
        el.font_size_signal(
            font_size
                .dedupe()
                .map(|v| v * TEXT_INPUT_FONT_SIZE_MULTIPLIER),
        )
    }
}

// TODO: get these from futures-signals https://github.com/Pauan/rust-signals/pull/85
pub type SyncBoxSignal<'a, T> = Pin<Box<dyn Signal<Item = T> + Send + Sync + 'a>>;

fn boxed_sync<'a, S, T>(signal: S) -> Pin<Box<dyn Signal<Item = T> + Send + Sync + 'a>>
where
    S: Sized + Send + Sync + Signal<Item = T> + 'a,
{
    Box::pin(signal)
}

#[derive(Default)]
struct TextInputField<T, F> {
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
    fn new(initial: T, formatter: F) -> Self {
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

    fn with_value(mut self, value: Mutable<T>) -> Self {
        self.value = Some(value);
        self
    }

    fn with_focused(mut self, focused: Mutable<bool>) -> Self {
        self.focused = Some(focused);
        self
    }

    fn with_highlight(mut self, highlight: Mutable<bool>) -> Self {
        self.highlight = highlight;
        self
    }

    fn with_border_color_option(
        mut self,
        border_color_option: impl Signal<Item = Option<Color>> + Send + Sync + 'static,
    ) -> Self {
        self.border_color_option = Some(boxed_sync(border_color_option));
        self
    }

    fn with_text_color_option(
        mut self,
        text_color_option: impl Signal<Item = Option<Color>> + Send + Sync + 'static,
    ) -> Self {
        self.text_color_option = Some(boxed_sync(text_color_option));
        self
    }

    fn with_text_signal(
        mut self,
        f: Box<dyn FnMut(TextInput, BoxSignal<'static, String>) -> TextInput>,
    ) -> Self {
        self.with_text_signal.push(f);
        self
    }
}

impl<T: Send + Sync + PartialEq + Reflect + Clone, F: Fn(T) -> String + Send + Sync + 'static>
    PointerEventAware for TextInputField<T, F>
{
}
impl<T: Send + Sync + PartialEq + Reflect + Clone, F: Fn(T) -> String + Send + Sync + 'static>
    CursorOnHoverable for TextInputField<T, F>
{
}
impl<T: Send + Sync + PartialEq + Reflect + Clone, F: Fn(T) -> String + Send + Sync + 'static>
    Sizeable for TextInputField<T, F>
{
}

impl<T: Send + Sync + PartialEq + Reflect + Clone, F: Fn(T) -> String + Send + Sync + 'static>
    ElementWrapper for TextInputField<T, F>
{
    type EL = TextInput;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let background_color = GLOBAL_PRIMARY_BACKGROUND_COLOR.clone();
        let font_size = GLOBAL_FONT_SIZE.clone();
        let highlighted_color = GLOBAL_HIGHLIGHTED_COLOR.clone();
        let unhighlighted_color = GLOBAL_UNHIGHLIGHTED_COLOR.clone();
        let border_radius = GLOBAL_BORDER_RADIUS.clone();
        let border_width = GLOBAL_BORDER_WIDTH.clone();
        let border_color = GLOBAL_BORDER_COLOR.clone();
        let value = self.value.unwrap_or_else(|| Mutable::new(self.initial));
        let hovered = Mutable::new(false);
        let focused = self.focused.unwrap_or_else(|| Mutable::new(false));
        let highlight = self.highlight;
        let highlighting =
            signal_or!(hovered.signal(), focused.signal(), highlight.signal()).broadcast();
        let text = value.signal_cloned().map(self.formatter).broadcast();
        self.el
            .height(Val::Px(34.))
            .on_signal_with_cosmic_edit(focused.signal(), |entity, focused| {
                if focused {
                    let id = entity.id();
                    let world = entity.into_world_mut();
                    let mut system_state = SystemState::<(
                        Query<&mut CosmicEditor>,
                        ResMut<CosmicFontSystem>,
                    )>::new(world);
                    let (mut cosmic_editors, mut font_system) = system_state.get_mut(world);
                    if let Ok(mut cosmic_editor) = cosmic_editors.get_mut(id) {
                        cosmic_editor.action(&mut font_system.0, Action::Motion(Motion::BufferEnd));
                        let current_cursor = cosmic_editor.cursor();
                        cosmic_editor.set_selection(Selection::Normal(
                            bevy_cosmic_edit::cosmic_text::Cursor {
                                line: 0,
                                index: 0,
                                affinity: current_cursor.affinity,
                            },
                        ));
                    }
                }
            })
            .update_raw_el(clone!((value) move |raw_el| {
                raw_el
                .on_remove(|world, _| {
                    world.commands().insert_resource(bevy_cosmic_edit::CursorPluginDisabled);
                })
                .with_entity(clone!((value) move |mut entity| {
                    let handler = entity.world_scope(|world| {
                        register_system(world, move |In(reflect): In<Box<dyn Reflect>>| {
                            if let Ok(cur) = reflect.downcast::<T>() {
                                value.set_neq(*cur);
                            }
                        })
                    });
                    entity.insert(FieldListener { handler });
                }))
            }))
            .hovered_sync(hovered.clone())
            .text_signal(text.signal_cloned())
            .focus_signal(focused.signal())
            .focused_sync(focused.clone())
            .on_click_outside_with_system(
                |In((entity, _)),
                 focused_option: Option<Res<FocusedTextInput>>,
                 mut commands: Commands| {
                    if focused_option.as_deref().map(Deref::deref).copied() == Some(entity) {
                        commands.remove_resource::<FocusedTextInput>();
                    }
                },
            )
            .apply(text_input_font_size_style(font_size.signal()))
            .line_height_signal(font_size.signal())
            .attrs(
                TextAttrs::new()
                    .family(FamilyOwned::new(Family::Name("Fira Mono")))
                    .weight(FontWeight::MEDIUM)
                    .color_signal(
                        signal_short_circuit(
                            self.text_color_option,
                            highlighting.signal(),
                            highlighted_color.signal(),
                            unhighlighted_color.signal(),
                        )
                        .map(Some),
                    ),
            )
            .cursor_color_signal(unhighlighted_color.signal().map(CursorColor))
            .fill_color_signal(background_color.signal().map(CosmicBackgroundColor))
            .selection_color_signal(border_color.signal().map(SelectionColor))
            .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
            .apply(border_width_style(BoxEdge::ALL, border_width.signal()))
            .apply(border_color_style(signal_short_circuit(
                self.border_color_option,
                highlighting.signal(),
                highlighted_color.signal(),
                border_color.signal(),
            )))
            .apply(|mut el| {
                for mut f in self.with_text_signal {
                    el = f(el, text.signal_cloned().boxed());
                }
                el
            })
    }
}

fn signal_short_circuit<T: Copy + Send + Sync + 'static>(
    short_circuit_option: Option<impl Signal<Item = Option<T>> + Send + Sync + 'static>,
    bool_signal: impl Signal<Item = bool> + Send + Sync + 'static,
    true_signal: impl Signal<Item = T> + Send + Sync + 'static,
    false_signal: impl Signal<Item = T> + Send + Sync + 'static,
) -> impl Signal<Item = T> {
    let true_signal = true_signal.broadcast();
    let false_signal = false_signal.broadcast();
    let alt_path = bool_signal
        .map_bool_signal(
            clone!((true_signal) move || true_signal.signal().apply(boxed_sync)),
            clone!((false_signal) move || false_signal.signal().apply(boxed_sync)),
        )
        .broadcast();
    if let Some(short_circuit_option) = short_circuit_option {
        short_circuit_option
            .map_option(
                |x| always(x).apply(boxed_sync),
                clone!((alt_path) move || alt_path.signal().apply(boxed_sync)),
            )
            .flatten()
            .apply(SignalEither::Left)
    } else {
        alt_path.signal().apply(SignalEither::Right)
    }
}

pub trait NumericFieldable {
    type T: Default
        + PartialEq
        + Reflect
        + Copy
        + std::ops::Add<Self::T, Output = Self::T>
        + std::ops::Sub<Self::T, Output = Self::T>
        + Display
        + num::Bounded
        + PartialOrd
        + std::str::FromStr
        + TypePath;
    const STEP: Self::T;
}

macro_rules! impl_numeric_fieldable {
    ($type:ty, $step:expr) => {
        impl NumericFieldable for $type {
            type T = $type;
            const STEP: $type = $step;
        }
    };
}

impl_numeric_fieldable!(isize, 1);
impl_numeric_fieldable!(i8, 1);
impl_numeric_fieldable!(i16, 1);
impl_numeric_fieldable!(i32, 1);
impl_numeric_fieldable!(i64, 1);
impl_numeric_fieldable!(i128, 1);
impl_numeric_fieldable!(usize, 1);
impl_numeric_fieldable!(u8, 1);
impl_numeric_fieldable!(u16, 1);
impl_numeric_fieldable!(u32, 1);
impl_numeric_fieldable!(u64, 1);
impl_numeric_fieldable!(u128, 1);
impl_numeric_fieldable!(f32, 0.1);
impl_numeric_fieldable!(f64, 0.1);

const INITIAL_NUMERIC_FIELD_INPUT_WIDTH: f32 = 100.;

const INPUT_WIDTH_PER_CHAR: f32 = 10.;
const NUMERIC_FIELD_GROW_THRESHOLD: usize = 4;

fn numeric_field<T: NumericFieldable>() -> impl Element {
    let dragging = Mutable::new(false);
    let parse_failed = Mutable::new(false);
    let highlight = Mutable::new(false);
    let focused = Mutable::new(false);
    let value = Mutable::new(T::T::default());
    let error_color = GLOBAL_ERROR_COLOR.clone();
    let parse_failure_color = parse_failed
        .signal()
        .map_true_signal(move || error_color.signal())
        .broadcast();
    // TODO: float formatting should be configurable
    TextInputField::new(T::T::default(), |x| format!("{:.1}", x))
        .with_value(value.clone())
        .with_focused(focused.clone())
        .with_highlight(highlight.clone())
        .with_border_color_option(parse_failure_color.signal())
        .with_text_color_option(parse_failure_color.signal())
        // TODO: without this initial static value, width snaps from 100% due to signal runtime lag
        .width(Val::Px(INITIAL_NUMERIC_FIELD_INPUT_WIDTH))
        .with_text_signal(Box::new(|self_, text_signal| self_.width_signal(text_signal.map(|text| text.len()).map(|len| INITIAL_NUMERIC_FIELD_INPUT_WIDTH + if len > NUMERIC_FIELD_GROW_THRESHOLD { (len - NUMERIC_FIELD_GROW_THRESHOLD) as f32 * INPUT_WIDTH_PER_CHAR } else { 0. }).map(Val::Px))))
        .update_raw_el(clone!((value, parse_failed, focused, dragging) move |raw_el| {
            raw_el
            .insert(TextInputFocusOnDownDisabled)
            .on_signal_sync(focused.signal(), clone!((value) move |_, focused| {
                if !focused {
                    let mut lock = parse_failed.lock_mut();
                    if *lock {
                        value.lock_mut().deref_mut();  // resurface valid value
                        *lock = false;
                    }
                }
            }))
            .on_event_with_system_stop_propagation::<Pointer<DragStart>, _>(clone!((highlight, dragging) move |_: In<_>, mut commands: Commands| {
                commands.insert_resource(CursorOnHoverDisabled);
                highlight.set_neq(true);
                dragging.set_neq(true);
            }))
            .on_event_with_system_stop_propagation::<Pointer<DragEnd>, _>(clone!((dragging) move |_: In<_>, mut commands: Commands| {
                commands.remove_resource::<CursorOnHoverDisabled>();
                highlight.set_neq(false);
                dragging.set_neq(false);
            }))
            .on_event_with_system_stop_propagation::<Pointer<Drag>, _>(move |
                In((ui_entity, drag)): In<(Entity, Pointer<Drag>)>,
                accessories: Query<&Accessory>,
                parents: Query<&Parent>,
                sync_components: Query<&EntityRoot>,
                mut field_path_cache: ResMut<FieldPathCache>,
                mut commands: Commands| {
                    if let Ok(Accessory { entity, component, .. }) = accessories.get(ui_entity).cloned() {
                        let new = {
                            let cur = value.get();
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
                        let field_path = field_path_cached(ui_entity, &accessories, &parents, &sync_components, &mut field_path_cache);
                        commands.add(move |world: &mut World| {
                            with_reflect_mut(world, entity, component, |reflect| {
                                if let Ok(target) = reflect.reflect_path_mut(&field_path) {
                                    let _ = target.try_apply(new.as_reflect());
                                }
                            });
                        });
                    }
                }
            )
        }))
        .cursor_signal(focused.signal().map_bool(|| CursorIcon::Text, || CursorIcon::EwResize))
        .on_click_with_system(move |In((entity, _)), click: Listener<Pointer<Click>>, mut commands: Commands| {
            if matches!(click.button, PointerButton::Primary) {
                if !dragging.get() {
                    focused.set_neq(true);
                    commands.insert_resource(FocusedTextInput(entity));
                }
            }
        })
        .into_el()
        .mode(CosmicWrap::InfiniteLine)
        .max_lines(MaxLines(1))
        .scroll_disabled()
        .on_change_with_system(clone!((parse_failed) move |
            In((ui_entity, text)): In<(Entity, String)>,
            accessories: Query<&Accessory>,
            parents: Query<&Parent>,
            sync_components: Query<&EntityRoot>,
            mut field_path_cache: ResMut<FieldPathCache>,
            mut commands: Commands| {
                let result = text.parse::<T::T>();
                parse_failed.set_neq(result.is_err());
                if let Ok(new) = result {
                    if let Ok(Accessory { entity, component, .. }) = accessories.get(ui_entity).cloned() {
                        let field_path = field_path_cached(ui_entity, &accessories, &parents, &sync_components, &mut field_path_cache);
                        commands.add(move |world: &mut World| {
                            with_reflect_mut(world, entity, component, |reflect| {
                                if let Ok(target) = reflect.reflect_path_mut(&field_path) {
                                    let _ = target.try_apply(new.as_reflect());
                                }
                            });
                        });
                    }
                }
            }
        ))
}

const INITIAL_STRING_FIELD_INPUT_WIDTH: f32 = 200.;
const STRING_FIELD_GROW_THRESHOLD: usize = 16;

fn string_field() -> impl Element {
    let padding = GLOBAL_PADDING.clone();
    TextInputField::new(String::new(), identity)
        .cursor(CursorIcon::Text)
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
        .on_focused_change_with_system(|In((entity, focused)), mut commands: Commands| {
            if focused {
                commands.insert_resource(FocusedTextInput(entity));
            }
        })
        .on_change_with_system(
            move |In((ui_entity, text)): In<(Entity, String)>,
                  accessories: Query<&Accessory>,
                  parents: Query<&Parent>,
                  sync_components: Query<&EntityRoot>,
                  mut field_path_cache: ResMut<FieldPathCache>,
                  mut commands: Commands| {
                if let Ok(Accessory {
                    entity, component, ..
                }) = accessories.get(ui_entity).cloned()
                {
                    let field_path = field_path_cached(
                        ui_entity,
                        &accessories,
                        &parents,
                        &sync_components,
                        &mut field_path_cache,
                    );
                    commands.add(move |world: &mut World| {
                        with_reflect_mut(world, entity, component, |reflect| {
                            if let Ok(target) = reflect.reflect_path_mut(&field_path) {
                                let _ = target.try_apply(text.as_reflect());
                            }
                        });
                    });
                }
            },
        )
}

#[derive(Clone)]
struct FieldListener {
    handler: SystemId<Box<dyn Reflect>>,
}

impl Component for FieldListener {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_remove(|mut world, entity, _| {
            if let Some(&Self { handler }) = world.get::<Self>(entity) {
                world.commands().add(move |world: &mut World| {
                    let _ = world.remove_system(handler);
                });
            }
        });
    }
}

#[derive(Component, Clone, Debug)]
struct Accessory {
    entity: Entity,
    component: ComponentId,
    access_option: Option<Access<'static>>,
}

fn sync_entities(
    entities: &MutableBTreeMap<Entity, EntityData>,
    new: impl IntoIterator<Item = Entity>,
    debug_names: &Query<DebugName>,
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
                name: Mutable::new(name_option.clone()),
                ..default()
            },
        );
    }
    for entity in old.difference(&new) {
        entities.remove(entity);
        field_path_cache.0.remove(entity);
    }
}

fn orphan_entity_syncer(
    query: Query<
        Entity,
        (
            Without<Parent>,
            Without<HaalkaOneShotSystem>,
            Without<HaalkaObserver>,
            Without<AaloOneShotSystem>,
        ),
    >,
    debug_names: Query<DebugName>,
    mut field_path_cache: ResMut<FieldPathCache>,
) {
    sync_entities(
        &ORPHAN_ENTITIES,
        query.into_iter(),
        &debug_names,
        &mut field_path_cache,
    );
}

fn entity_syncer(
    query: Query<
        Entity,
        (
            Without<HaalkaOneShotSystem>,
            Without<HaalkaObserver>,
            Without<AaloOneShotSystem>,
        ),
    >,
    debug_names: Query<DebugName>,
    mut field_path_cache: ResMut<FieldPathCache>,
) {
    sync_entities(
        &ENTITIES,
        query.into_iter(),
        &debug_names,
        &mut field_path_cache,
    );
}

fn sync_components(
    mut entity_roots: Query<(Entity, &mut EntityRoot), With<SyncComponents>>,
    entities: &Entities,
    archetypes: &Archetypes,
    mut commands: Commands,
) {
    for (ui_entity, mut sync_components) in entity_roots.iter_mut() {
        if let Some(location) = entities.get(sync_components.entity) {
            if let Some(archetype) = archetypes.get(location.archetype_id) {
                let new = archetype.components().collect::<HashSet<_>>();
                let added = new
                    .difference(&sync_components.components)
                    .copied()
                    .collect::<Vec<_>>();
                let removed = sync_components
                    .components
                    .difference(&new)
                    .copied()
                    .collect::<Vec<_>>();
                sync_components.components = new;
                if !added.is_empty() {
                    commands.trigger_targets(ComponentsAdded(added), ui_entity);
                }
                if !removed.is_empty() {
                    commands.trigger_targets(ComponentsRemoved(removed), ui_entity);
                }
            }
        }
    }
}

// TODO: limit size of the cache
#[derive(Resource, Default)]
pub struct FieldPathCache(HashMap<Entity, ParsedPath>);

fn sync_ui(
    accessories: Query<&Accessory>,
    parents: Query<&Parent>,
    entity_roots: Query<&EntityRoot>,
    field_listeners: Query<(Entity, &Accessory, &FieldListener)>,
    mut field_path_cache: ResMut<FieldPathCache>,
    mut commands: Commands,
) {
    for (
        ui_entity,
        &Accessory {
            entity, component, ..
        },
        &FieldListener { handler },
    ) in field_listeners.iter()
    {
        let field_path = field_path_cached(
            ui_entity,
            &accessories,
            &parents,
            &entity_roots,
            &mut field_path_cache,
        );
        commands.add(move |world: &mut World| {
            if let Some(Ok(cur)) = reflect(world, entity, component).map(|reflect| {
                reflect
                    .reflect_path(&field_path)
                    .map(|target| target.clone_value())
            }) {
                let _ = world.run_system_with_input(handler, cur);
            }
        });
    }
}

// (entity name or id, optional component name, optional reflect path as string)

#[derive(Clone, PartialEq)]
pub struct InspectionTarget {
    entity: String,
    component: Option<String>,
    path: Option<ParsedPath>,
}

impl From<(&str, &str, &str)> for InspectionTarget {
    fn from((entity, component, path): (&str, &str, &str)) -> Self {
        InspectionTarget {
            entity: entity.to_string(),
            component: if component.is_empty() {
                None
            } else {
                Some(component.to_string())
            },
            path: if path.is_empty() {
                None
            } else {
                ParsedPath::parse(path).ok()
            },
        }
    }
}

#[derive(Clone, Debug)]
enum ProgressPart {
    Component(String),
    Access(Access<'static>),
}

#[derive(Event, Clone)]
struct InspectionTargetProgress {
    target: InspectionTarget,
    pending: VecDeque<ProgressPart>,
}

#[derive(Component)]
pub(super) struct InspectionTargets(pub(super) Vec<InspectionTarget>);

#[derive(Event)]
struct RemoveTarget {
    target: InspectionTarget,
    from: Entity,
}

#[derive(Event)]
struct ScrollTo(Entity);

#[derive(Component)]
struct AfterNodely(BoxedSystem<Entity>);

fn wait_for_nodely(
    after_nodelies: Query<(Entity, &Node), With<AfterNodely>>,
    mut commands: Commands,
) {
    for (id, node) in after_nodelies.iter() {
        if node.size() != Vec2::ZERO {
            commands.add(move |world: &mut World| {
                if let Some(mut entity) = world.get_entity_mut(id) {
                    if let Some(AfterNodely(mut system)) = entity.take::<AfterNodely>() {
                        system.initialize(world);
                        system.run(id, world);
                    }
                }
            });
        }
    }
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

#[derive(Component, Default)]
struct SyncOrphanEntities;

#[derive(Component, Default)]
struct SyncEntities;

// could not use `.on_viewport_location_change` (ideal) or `.on_scroll` (less ideal) because this requires frame perfection;
// the former has 4(?) observer/one shot system indirections and the latter has 2(?)
// actually turns out even this isn't scroll perfect if one scrolls fast enough, and using
// scrollbars complicates things further ...
fn expanded_parent_pinner(
    inspectors: Query<(Entity, &MutableViewport), (With<EntityInspectorMarker>, Without<ScrollDisabled>)>,
    rect: Query<(&Node, &GlobalTransform)>,
    children: Query<&Children>,
    parents: Query<&Parent>,
    expanded: Query<&Expanded>,
    mut styles: Query<&mut Style>,
    mut mouse_wheels: EventReader<MouseWheel>,
    last_expanded_headers: Query<&LastExpandedHeader>,
    resize_parents: Query<(Entity, &ResizeParent)>,
    mut resize_parents_cache: Local<HashMap<Entity, Entity>>,
    mut commands: Commands,
) {
    for &MouseWheel { y, .. } in mouse_wheels.read() {
        for (entity, mutable_viewport) in inspectors.iter() {
            // TODO: use relations for this
            if !resize_parents_cache.contains_key(&entity) {
                for parent in parents.iter_ancestors(entity) {
                    if resize_parents.contains(parent) {
                        resize_parents_cache.insert(entity, parent);
                        break;
                    }
                }
            }
            if let Some(&resize_parent) = resize_parents_cache.get(&entity) {
                let top_offset = if let Ok(style) = styles.get(resize_parent) {
                    if let Val::Px(top) = style.top {
                        top
                    } else {
                        0.
                    }
                } else {
                    0.
                };
                let mut pinned = 0;
                for child in children.iter_descendants(entity) {
                    if expanded.contains(child) {
                        if let Ok((node, global_transform)) = rect.get(child) {
                            let rect1 = node.logical_rect(global_transform);
                            // TODOTODO: need to get this 20. from the node's scroll settings, which means i need to make basic scroll handler a component
                            let rel_y = rect1.min.y + if y < 0. { -20. } else { 20. } - top_offset;
                            if let Some(&child) = children
                                .get(child)
                                .ok()
                                .and_then(|children| children.first())
                            {
                                if let Ok((node, global_transform)) = rect.get(child) {
                                    let rect1 = node.logical_rect(global_transform);
                                    // println!("{:?}", mutable_viewport.scene());
                                    // println!("{:?}", mutable_viewport.viewport());
                                    println!("rect 2: {:?}", rect1);
                                    if let Ok(mut style) = styles.get_mut(child) {
                                        if rel_y < 0. {
                                            style.top = Val::Px(-rel_y);
                                            // TODOTODO: get this 5. from row gap somehow
                                            if matches!(style.height, Val::Auto) || {
                                                if let Val::Px(height) = style.height {
                                                    height < 18. + 5.
                                                } else {
                                                    false
                                                }
                                            } {
                                                style.height = Val::Px(18. + 5.);
                                            }
                                            if let Some(mut entity) = commands.get_entity(child) {
                                                entity.try_insert(ZIndex::Local(i32::MAX));
                                            }
                                            if let Ok(LastExpandedHeader(last_expanded_header)) = last_expanded_headers.get(child) {
                                                last_expanded_header.set_neq(true);
                                            }
                                        } else {
                                            style.top = Val::Px(0.);
                                            // style.height = Val::Auto;
                                            if let Some(mut entity) = commands.get_entity(child) {
                                                entity.remove::<ZIndex>();
                                            }
                                            if let Ok(LastExpandedHeader(last_expanded_header)) = last_expanded_headers.get(child) {
                                                last_expanded_header.set_neq(false);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            orphan_entity_syncer.run_if(any_with_component::<SyncOrphanEntities>),
            (|| ORPHAN_ENTITIES.lock_mut().clear())
                .run_if(not(any_with_component::<SyncOrphanEntities>)),
            entity_syncer.run_if(any_with_component::<SyncEntities>),
            (|| ENTITIES.lock_mut().clear()).run_if(not(any_with_component::<SyncEntities>)),
            sync_components.run_if(any_with_component::<EntityRoot>),
            sync_ui.run_if(any_with_component::<FieldListener>),
            wait_for_nodely.run_if(any_with_component::<AfterNodely>),
            (
                unfocus_text_input_on_keys,
                left_align_editors.run_if(resource_removed::<FocusedTextInput>()),
            )
                .chain(),
            expanded_parent_pinner.run_if(any_with_component::<EntityInspectorMarker>),
        ),
    )
    .init_resource::<FieldPathCache>()
    .insert_resource(bevy_cosmic_edit::CursorPluginDisabled)
    .observe(
        |event: Trigger<RemoveTarget>,
         parents: Query<&Parent>,
         mut inspection_targets: Query<(Entity, &mut InspectionTargets)>,
         mut commands: Commands| {
            let RemoveTarget { target, from } = event.event();
            for parent in parents.iter_ancestors(*from) {
                if let Some(mut entity) = commands.get_entity(parent) {
                    entity.remove::<InspectionTargetProgress>();
                }
                if let Some((ui_entity, mut inspection_targets)) =
                    inspection_targets.get_mut(parent).ok()
                {
                    inspection_targets.0.retain(|t| t != target);
                    if inspection_targets.0.is_empty() {
                        if let Some(mut entity) = commands.get_entity(ui_entity) {
                            entity.remove::<InspectionTargets>();
                        }
                    }
                }
            }
        },
    )
    .observe(
        |event: Trigger<ScrollTo>,
         parents: Query<&Parent>,
         mutable_viewports: Query<&MutableViewport>,
         nodes: Query<(&Node, &GlobalTransform)>,
         mut styles: Query<&mut Style>| {
            let &ScrollTo(entity) = event.event();
            for parent in parents.iter_ancestors(entity) {
                if mutable_viewports.contains(parent) {
                    if let Ok((node, global_transform)) = nodes.get(entity) {
                        if let Ok(mut style) = styles.get_mut(parent) {
                            style.top = Val::Px(-node.logical_rect(global_transform).min.y);
                        }
                    }
                    return;
                }
            }
        },
    );
}
