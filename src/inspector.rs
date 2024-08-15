use std::{
    collections::{HashMap, HashSet}, i32, sync::{Arc, Mutex}
};

use bevy::{
    ecs::{
        archetype::Archetypes,
        component::{ComponentId, Components},
        entity::Entities,
        system::{SystemId, SystemState},
    },
    prelude::*,
    reflect::{
        Access, DynamicEnum, DynamicStruct, DynamicTuple, DynamicVariant, ParsedPath, ReflectKind,
        ReflectMut, ReflectRef, TypeInfo, TypeRegistry, VariantInfo,
    },
};
use haalka::prelude::*;
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher,
};

use super::{defaults::*, globals::*, reflect::*, style::*, utils::*, widgets::*};
use crate::impl_syncers;

// TODO: scrollbars
// TODO: implement frontend for at least all ui node types
// TODO: `Name` component syncing
// TODO: document how to make custom type views
// TODO: popout windows
// TODO: drag and drop
// TODO: window resizing
// TODO: on hover tooltips
// TODO: unit struct handling (a tooltip with "unit struct" should suffice)
// TODO: asset based hot reloadable config
// TODO: optional limited components viewport within entity
// TODO: list modification abilities, add, remove, reorder

#[derive(Clone, Default)]
pub struct EntityData {
    pub name: Mutable<Option<String>>,
    pub expanded: Mutable<bool>,
    pub filtered: Mutable<bool>,
    pub components: MutableBTreeMap<ComponentId, ComponentData>,
    components_transformers:
        Arc<Mutex<Vec<Box<dyn FnMut(ComponentsSignalVec) -> ComponentsSignalVec + Send>>>>,
}

pub static ENTITIES: Lazy<MutableBTreeMap<Entity, EntityData>> = Lazy::new(default);

pub struct Search {
    pub search: Mutable<String>,
    pub fuzzy: Mutable<bool>,
}

type EntitySignalVec = std::pin::Pin<Box<dyn SignalVec<Item = (Entity, EntityData)> + Send>>;
type ComponentsSignalVec =
    std::pin::Pin<Box<dyn SignalVec<Item = (ComponentId, ComponentData)> + Send>>;

/// Configuration frontend for entity inspecting elements.
pub struct EntityInspector {
    base: Column<NodeBundle>,
    entities: MutableBTreeMap<Entity, EntityData>,
    entities_transformers: Vec<Box<dyn FnMut(EntitySignalVec) -> EntitySignalVec>>,
    components_transformers: Vec<Box<dyn FnMut(ComponentsSignalVec) -> ComponentsSignalVec + Send>>,
    search: Option<Search>,
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
}

impl ElementWrapper for EntityInspector {
    type EL = Column<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.base
    }

    fn into_el(self) -> Self::EL {
        let Self {
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
            ..
        } = self;
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
        self.base
            .update_raw_el(|raw_el| raw_el.hold_tasks(tasks))
            .apply(column_style(row_gap.signal()))
            .apply(all_padding_style(padding.signal()))
            .apply(border_style(border_width.signal(), border_color.signal()))
            .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
            .apply(background_style(primary_background_color.signal()))
            .apply(height_style(height.signal()))
            .apply(width_style(width.signal()))
            .cursor(CursorIcon::Default)
            .scrollable_on_hover(ScrollabilitySettings {
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                scroll_handler: BasicScrollHandler::new()
                    .direction(ScrollDirection::Vertical)
                    .pixels_signal(self.scroll_pixels.signal().dedupe())
                    .into(),
            })
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
    }
}

impl EntityInspector {
    pub fn new() -> Self {
        Self {
            base: Column::<NodeBundle>::new(),
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

    // TODO: get position of particular field and automatically scroll to it
    // pub fn jump_to<'p>(entity: &str, component: &str, path: impl ReflectPath<'p>) {

    // }

    pub fn search(mut self) -> Self {
        self.search = Some(Search {
            search: Mutable::new(String::new()),
            fuzzy: Mutable::new(true),
        });
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
struct SyncComponents {
    entity: Entity,
    components: HashSet<ComponentId>,
}

#[derive(Event)]
struct EntitiesAdded(Vec<Entity>);

#[derive(Event)]
struct EntitiesRemoved(Vec<Entity>);

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
        el
        .update_raw_el(clone!((components, expanded) move |raw_el| {
            raw_el
            .observe(clone!((components => components_map) move |event: Trigger<ComponentsAdded>, components: &Components| {
                let ComponentsAdded(added) = event.event();
                let mut lock = components_map.lock_mut();
                for &component in added {
                    if let Some(info) = components.get_info(component) {
                        let name = pretty_type_name::pretty_type_name_str(info.name());
                        lock.insert_cloned(component, ComponentData { name, viewable: Mutable::new(false), expanded: Mutable::new(false) });
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
            .component_signal::<SyncComponents, _>(
                // TODO: only sync when in view
                expanded.signal().map_true(move || SyncComponents{ entity, components: HashSet::from_iter(components.lock_ref().iter().map(|(&id, _)| id))}),
            )
        }))
        .apply(column_style(row_gap.signal()))
        .item(show_name.then(||
            HighlightableText::new()
            .with_text(clone!((font_size) move |text| {
                text
                .text_signal(name.signal_cloned().map_option(move |name| format!("{name} ({entity})"), move || format!("Entity ({entity})")))
                .font_size_signal(font_size.signal())
            }))
            .highlighted_color_signal(highlighted_color.signal())
            .unhighlighted_color_signal(unhighlighted_color.signal())
            .on_click(clone!((expanded) move || flip(&expanded))),
        ))
        .item_signal(if show_name { expanded.signal().boxed() } else { always(true).boxed() }.map_true(clone!((row_gap, column_gap, primary_background_color, secondary_background_color, border_width, border_color, padding, highlighted_color, unhighlighted_color) move || {
            Column::<NodeBundle>::new()
                .apply(column_style(row_gap.signal()))
                .apply(horizontal_padding_style(padding.signal()))
                .apply(left_bordered_style(border_width.signal(), border_color.signal()))
                .items_signal_vec({
                    let mut signal_vec = components.entries_cloned().boxed();
                    for f in components_transformers.lock().unwrap().iter_mut() {
                        signal_vec = f(signal_vec);
                    }
                    signal_vec
                    .map_signal(|(component, data)| {
                        data.viewable.signal().map(move |cur| (component, data.clone(), cur))
                    })
                    .sort_by_cloned(|(_, ComponentData { name: left_name, .. }, left_viewable), (_, ComponentData { name: right_name, .. }, right_viewable)| left_viewable.cmp(right_viewable).reverse().then(left_name.cmp(right_name)))
                    .map(clone!((row_gap, column_gap, secondary_background_color, border_width, border_color, padding, highlighted_color, unhighlighted_color) move |(component, ComponentData { name, expanded, viewable }, _)|
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
                    ))
                })
        })))
    }
}

impl EntityElement {
    fn new(entity: Entity, entity_data: EntityData) -> Self {
        let font_size = Mutable::new(DEFAULT_FONT_SIZE);
        let row_gap = Mutable::new(DEFAULT_ROW_GAP);
        let column_gap = Mutable::new(DEFAULT_COLUMN_GAP);
        let primary_background_color = Mutable::new(DEFAULT_PRIMARY_BACKGROUND_COLOR);
        let secondary_background_color = Mutable::new(DEFAULT_SECONDARY_BACKGROUND_COLOR);
        let border_width = Mutable::new(DEFAULT_BORDER_WIDTH);
        let border_color = Mutable::new(DEFAULT_BORDER_COLOR);
        let padding = Mutable::new(DEFAULT_ROW_GAP);
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
}

struct AccessData {
    access: Access<'static>,
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

#[derive(Clone)]
enum Node {
    Access(AccessFieldData),
    TypePath(String),
}

impl<T: Into<String>> From<T> for Node {
    fn from(s: T) -> Self {
        Self::TypePath(s.into())
    }
}

impl From<AccessFieldData> for Node {
    fn from(data: AccessFieldData) -> Self {
        Self::Access(data)
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
    initial: usize,
}

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
            FieldType::Component(name) => (name, None),
            FieldType::Access(access) => (access.to_string(), Some(access.clone())),
        };
        let type_path = Mutable::new(None);
        let node_type = Mutable::new(None);
        let enum_data_option = Mutable::new(None);
        let component_root = matches!(field_type, FieldType::Component(_));
        let el = Column::<NodeBundle>::new()
            .apply(column_style(row_gap.signal()))
            .update_raw_el(|raw_el| {
                raw_el.on_spawn(clone!((viewable, expanded, node_type, type_path, enum_data_option) move |world, ui_entity| {
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
                            let mut system_state = SystemState::<(Query<&Accessory>, Query<&Parent>, Query<&SyncComponents>, ResMut<FieldPathCache>)>::new(world);
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
                                            initial: enum_.variant_index()
                                        }
                                    ));
                                    set_viewable = true;
                                }
                            },
                            ReflectRef::Value(value) => {
                                let type_path = value.reflect_type_path();
                                node_type.set(Some(NodeType::Solo(type_path.to_string())));
                                expanded.set_neq(true);
                            },
                            _ => ()
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
                .width(Val::Percent(100.))
                .with_style(|mut style| style.width = Val::Percent(100.))
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
                    if !component_root {
                        type_path.signal_cloned().map_some(clone!((type_path_color) move |type_path| {
                            DynamicText::new()
                            .text_signal(hovered.signal().map_bool(clone!((type_path) move || type_path.clone()), move || pretty_type_name::pretty_type_name_str(&type_path)))
                            .color_signal(type_path_color.signal())
                        }))
                        .boxed()
                    } else {
                        hovered.signal().map_true_signal(move || type_path.signal_cloned())
                        .map(Option::flatten)
                        .map_some(clone!((type_path_color) move |type_path| {
                            DynamicText::new()
                            .text(type_path)
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
                    .apply(nested_fields_style(row_gap.signal(), padding.signal(), border_width.signal(), border_color.signal()))
                    .apply(column_style(row_gap.signal()))
                    .item_signal(
                        enum_data_option.signal_cloned()
                        .map_some(clone!((access_option, node_type) move |EnumData { variants, initial }| {
                            let options = variants.into_iter().map(Into::into).collect::<Vec<_>>().into();
                            let selected = Mutable::new(Some(initial));
                            let show_dropdown = Mutable::new(false);
                            let dropdown_entity = Mutable::new(None);
                            Dropdown::new(options)
                            .height(Val::Percent(100.))
                            .with_show_dropdown(show_dropdown.clone())
                            .update_raw_el(clone!((access_option, selected, dropdown_entity) move |raw_el| {
                                raw_el
                                .insert(Accessory { entity, component, access_option })
                                .with_entity(clone!((selected) move |mut entity| {
                                    dropdown_entity.set_neq(Some(entity.id()));
                                    let handler = entity.world_scope(move |world| {
                                        register_system(world, clone!((selected) move |In(reflect): In<Box<dyn Reflect>>| {
                                            if let ReflectRef::Enum(enum_) = reflect.reflect_ref() {
                                                selected.set_neq(Some(enum_.variant_index()));
                                            }
                                        }))
                                    });
                                    entity.insert(FieldListener { handler });
                                }))
                            }))
                            .width(Val::Percent(60.))
                            .selected_signal(selected.signal())
                            // TODO: this should just take a system instead
                            .option_handler_system(clone!((node_type) move |
                                In(i),
                                accessories: Query<&Accessory>,
                                parents: Query<&Parent>,
                                sync_components: Query<&SyncComponents>,
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
                                                        if let Some(TypeInfo::Enum(enum_info)) = enum_.get_represented_type_info() {
                                                            if let Some(variant_info) = enum_info.variant_at(i) {
                                                                match variant_info {
                                                                    VariantInfo::Struct(struct_info) => {
                                                                        let mut fields = vec![];
                                                                        for i in 0..struct_info.field_len() {
                                                                            if let Some(name) = struct_info.field_at(i).map(|field| field.name()) {
                                                                                let access = Access::Field(name.to_string().into());
                                                                                fields.push(AccessFieldData::new(access));
                                                                            }
                                                                        }
                                                                        node_type.set(Some(NodeType::Multi { items: fields.into(), size_dynamic: None }));
                                                                    },
                                                                    VariantInfo::Tuple(tuple_info) => {
                                                                        let mut fields = vec![];
                                                                        for i in 0..tuple_info.field_len() {
                                                                            let access = Access::TupleIndex(i);
                                                                            fields.push(AccessFieldData::new(access));
                                                                        }
                                                                        node_type.set(Some(NodeType::Multi { items: fields.into(), size_dynamic: None }));
                                                                    },
                                                                    VariantInfo::Unit(_) => {
                                                                        // TODO: unit enum indicator
                                                                        node_type.take();
                                                                    },
                                                                }
                                                                if let Some(default) = variant_default_value(variant_info, &type_registry.read()) {
                                                                    target.apply(&default);
                                                                }
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
                                .apply(column_style(row_gap.signal()))
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
                        .map(clone!((access_option) move |mut el_option| {
                            el_option = el_option.map(clone!((access_option) move |el| el.update_raw_el(|raw_el| raw_el.insert(Accessory { entity, component, access_option }))));
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
        "bool" => Some(bool_field().type_erase()),
        "bevy_ecs::entity::Entity" => Some(entity_field().type_erase()),
        _ => None,
    }
}

fn field_path(
    entity: Entity, // field's ui entity
    accessories: &Query<&Accessory>,
    parents: &Query<&Parent>,
    sync_components: &Query<&SyncComponents>,
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
    sync_components: &Query<&SyncComponents>,
    field_path_cache: &mut ResMut<FieldPathCache>,
) -> ParsedPath {
    if let Some(field_path) = field_path_cache.0.get(&entity) {
        field_path.clone()
    } else {
        let field_path = field_path(entity, accessories, parents, sync_components);
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
         sync_components: Query<&SyncComponents>,
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

#[derive(Component, Clone)]
struct FieldListener {
    handler: SystemId<Box<dyn Reflect>>,
}

#[derive(Component, Clone)]
struct Accessory {
    entity: Entity,
    component: ComponentId,
    access_option: Option<Access<'static>>,
}

fn entity_syncer(
    query: Query<
        Entity,
        (
            Without<Parent>,
            Without<HaalkaOneShotSystem>,
            Without<HaalkaObserver>,
            Without<AaloOneShotSystem>,
        ),
    >,
    mut entity_set: Local<HashSet<Entity>>,
    mut commands: Commands,
) {
    let new = query.into_iter().collect::<HashSet<_>>();
    let added = new.difference(&entity_set).copied().collect::<Vec<_>>();
    let removed = entity_set.difference(&new).copied().collect::<Vec<_>>();
    *entity_set = new;
    if !added.is_empty() {
        commands.trigger(EntitiesAdded(added));
    }
    if !removed.is_empty() {
        commands.trigger(EntitiesRemoved(removed));
    }
}

fn sync_components(
    mut sync_components: Query<(Entity, &mut SyncComponents)>,
    entities: &Entities,
    archetypes: &Archetypes,
    mut commands: Commands,
) {
    for (ui_entity, mut sync_components) in sync_components.iter_mut() {
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
    sync_components: Query<&SyncComponents>,
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
            &sync_components,
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

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            entity_syncer,
            sync_components.run_if(any_with_component::<SyncComponents>),
            sync_ui.run_if(any_with_component::<FieldListener>),
        ),
    )
    .init_resource::<FieldPathCache>()
    .observe(
        |event: Trigger<EntitiesAdded>, debug_names: Query<DebugName>| {
            let mut entities = ENTITIES.lock_mut();
            let EntitiesAdded(added) = event.event();
            for &entity in added {
                entities.insert_cloned(
                    entity,
                    EntityData {
                        name: Mutable::new(
                            debug_names
                                .get(entity)
                                .ok()
                                .and_then(|name| name.name)
                                .map(|name| name.to_string()),
                        ),
                        ..default()
                    },
                );
            }
        },
    )
    .observe(
        |event: Trigger<EntitiesRemoved>, mut field_path_cache: ResMut<FieldPathCache>| {
            let mut entities = ENTITIES.lock_mut();
            let EntitiesRemoved(removed) = event.event();
            for entity in removed {
                entities.remove(entity);
                field_path_cache.0.remove(&entity);
            }
        },
    );
}
