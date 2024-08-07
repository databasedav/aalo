use std::collections::{HashMap, HashSet};

use bevy::{
    ecs::{
        archetype::Archetypes,
        component::{ComponentId, Components},
        entity::Entities,
        system::{SystemId, SystemState},
    },
    prelude::*,
    reflect::{Access, ParsedPath},
};
use haalka::prelude::*;

use super::reflect::*;
use crate::impl_syncers;

// TODO: scrollbars
// TODO: fuzzy search
// TODO: implement frontend for at least all ui node types

const DEFAULT_HEIGHT: f32 = 400.;
const DEFAULT_WIDTH: f32 = 600.;
const DEFAULT_FONT_SIZE: f32 = 20.;
const DEFAULT_ROW_GAP: f32 = 5.;
const DEFAULT_COLUMN_GAP: f32 = 10.;
const DEFAULT_PADDING: f32 = 10.;
const DEFAULT_BORDER_RADIUS: f32 = 10.;
const DEFAULT_BORDER_WIDTH: f32 = 2.;

const DEFAULT_PRIMARY_BACKGROUND_COLOR: Color = Color::srgb(27. / 255., 27. / 255., 27. / 255.);
const DEFAULT_SECONDARY_BACKGROUND_COLOR: Color = Color::srgb(60. / 255., 60. / 255., 60. / 255.);
const DEFAULT_HIGHLIGHTED_COLOR: Color = Color::srgb(210. / 255., 210. / 255., 210. / 255.);
const DEFAULT_UNHIGHLIGHTED_COLOR: Color = Color::srgb(150. / 255., 150. / 255., 150. / 255.);
const DEFAULT_BORDER_COLOR: Color = Color::srgb(56. / 255., 56. / 255., 56. / 255.);

const DEFAULT_SCROLL_PIXELS: f32 = 20.;

#[derive(Clone)]
pub struct EntityData {
    pub name: Option<Name>,
    pub expanded: Mutable<bool>,
}

pub static ENTITIES: Lazy<MutableBTreeMap<Entity, EntityData>> = Lazy::new(default);

pub struct Inspector {
    el: Column<NodeBundle>,
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

impl_syncers!(Inspector {
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
});

impl ElementWrapper for Inspector {
    type EL = Column<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

static GLOBAL_FONT_SIZE: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(DEFAULT_FONT_SIZE));
static GLOBAL_ROW_GAP: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(DEFAULT_ROW_GAP));
static GLOBAL_COLUMN_GAP: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(DEFAULT_COLUMN_GAP));
static GLOBAL_PADDING: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(DEFAULT_PADDING));
static GLOBAL_BORDER_RADIUS: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(DEFAULT_BORDER_RADIUS));
static GLOBAL_BORDER_WIDTH: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(DEFAULT_BORDER_WIDTH));
static GLOBAL_PRIMARY_BACKGROUND_COLOR: Lazy<Mutable<Color>> =
    Lazy::new(|| Mutable::new(DEFAULT_PRIMARY_BACKGROUND_COLOR));
static GLOBAL_SECONDARY_BACKGROUND_COLOR: Lazy<Mutable<Color>> =
    Lazy::new(|| Mutable::new(DEFAULT_SECONDARY_BACKGROUND_COLOR));
static GLOBAL_HIGHLIGHTED_COLOR: Lazy<Mutable<Color>> =
    Lazy::new(|| Mutable::new(DEFAULT_HIGHLIGHTED_COLOR));
static GLOBAL_UNHIGHLIGHTED_COLOR: Lazy<Mutable<Color>> =
    Lazy::new(|| Mutable::new(DEFAULT_UNHIGHLIGHTED_COLOR));
static GLOBAL_BORDER_COLOR: Lazy<Mutable<Color>> = Lazy::new(|| Mutable::new(DEFAULT_BORDER_COLOR));
static GLOBAL_SCROLL_PIXELS: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(DEFAULT_SCROLL_PIXELS));

impl Inspector {
    pub fn new() -> Self {
        let height = Mutable::new(DEFAULT_HEIGHT);
        let width = Mutable::new(DEFAULT_WIDTH);
        let font_size = GLOBAL_FONT_SIZE.clone();
        let row_gap = GLOBAL_ROW_GAP.clone();
        let column_gap = GLOBAL_COLUMN_GAP.clone();
        let padding = GLOBAL_PADDING.clone();
        let border_radius = GLOBAL_BORDER_RADIUS.clone();
        let border_width = GLOBAL_BORDER_WIDTH.clone();
        let primary_background_color = GLOBAL_PRIMARY_BACKGROUND_COLOR.clone();
        let secondary_background_color = GLOBAL_SECONDARY_BACKGROUND_COLOR.clone();
        let highlighted_color = GLOBAL_HIGHLIGHTED_COLOR.clone();
        let unhighlighted_color = GLOBAL_UNHIGHLIGHTED_COLOR.clone();
        let border_color = GLOBAL_BORDER_COLOR.clone();
        let scroll_pixels = GLOBAL_SCROLL_PIXELS.clone();
        let el = {
            Column::<NodeBundle>::new()
                .apply(column_style(row_gap.signal()))
                .apply(all_padding_style(padding.signal()))
                .apply(border_style(border_width.signal(), border_color.signal()))
                .apply(border_radius_style(border_radius.signal()))
                .apply(background_color_style(primary_background_color.signal()))
                .height_signal(height.signal().dedupe().map(Val::Px))
                .width_signal(width.signal().dedupe().map(Val::Px))
                .cursor(CursorIcon::Default)
                .scrollable_on_hover(ScrollabilitySettings {
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip_y(),
                    scroll_handler: BasicScrollHandler::new()
                        .direction(ScrollDirection::Vertical)
                        .pixels_signal(scroll_pixels.signal().dedupe())
                        .into(),
                })
                .items_signal_vec(ENTITIES.entries_cloned()
                    .map(
                    clone!((font_size, row_gap, border_color, border_width, padding, highlighted_color, unhighlighted_color) move |(id, data)| {
                        EntityElement::new(id, data)
                        .font_size_signal(font_size.signal())
                        .row_gap_signal(row_gap.signal())
                        .border_color_signal(border_color.signal())
                        .border_width_signal(border_width.signal())
                        .padding_signal(padding.signal())
                        .highlighted_color_signal(highlighted_color.signal())
                        .unhighlighted_color_signal(unhighlighted_color.signal())
                    }),
                ))
        };
        Self {
            el,
            height,
            width,
            font_size,
            row_gap,
            column_gap,
            padding,
            border_radius,
            border_width,
            primary_background_color,
            secondary_background_color,
            highlighted_color,
            unhighlighted_color,
            border_color,
            scroll_pixels,
        }
    }
}

fn nested_fields_style<E: Element>(
    row_gap: impl Signal<Item = f32> + Send + Sync + 'static,
    padding: impl Signal<Item = f32> + Send + 'static,
    border_width: impl Signal<Item = f32> + Send + 'static,
    border_color: impl Signal<Item = Color> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        let row_gap = row_gap.dedupe().broadcast();
        el.apply(column_style(row_gap.signal()))
            .apply(vertical_padding_style(row_gap.signal()))
            .apply(horizontal_padding_style(padding))
            .apply(left_bordered_style(border_width, border_color))
    }
}

fn column_style<E: Element>(
    row_gap: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Style>(
                row_gap.dedupe().map(Val::Px),
                |mut style, row_gap| style.row_gap = row_gap,
            )
        })
    }
}

fn row_style<E: Element>(
    column_gap: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Style>(
                column_gap.dedupe().map(Val::Px),
                |mut style, column_gap| style.column_gap = column_gap,
            )
        })
    }
}

#[derive(Clone, Default)]
struct ComponentData {
    name: String,
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

pub struct DynamicText {
    el: El<TextBundle>,
    text: Mutable<String>,
    font: Mutable<Handle<Font>>,
    font_size: Mutable<f32>,
    color: Mutable<Color>,
}

impl_syncers!(DynamicText { text: String, font: Handle<Font>, font_size: f32, color: Color });

impl ElementWrapper for DynamicText {
    type EL = El<TextBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Cursorable for DynamicText {}
impl PointerEventAware for DynamicText {}

impl DynamicText {
    fn new() -> Self {
        let text = Mutable::new(String::new());
        let font = Mutable::new(Default::default());
        let font_size = Mutable::new(DEFAULT_FONT_SIZE);
        let color = Mutable::new(DEFAULT_UNHIGHLIGHTED_COLOR);
        let el: El<TextBundle> = El::<TextBundle>::new()
            .text(Text::from_section(
                text.get_cloned(),
                TextStyle {
                    font: font.get_cloned(),
                    font_size: font_size.get(),
                    color: color.get(),
                },
            ))
            .on_signal_with_text(text.signal_cloned().dedupe_cloned(), |mut text, t| {
                if let Some(section) = text.sections.first_mut() {
                    section.value = t;
                }
            })
            .on_signal_with_text(font.signal_cloned(), |mut style, font| {
                if let Some(section) = style.sections.first_mut() {
                    section.style.font = font;
                }
            })
            .on_signal_with_text(font_size.signal().dedupe(), |mut style, font_size| {
                if let Some(section) = style.sections.first_mut() {
                    section.style.font_size = font_size;
                }
            })
            .on_signal_with_text(color.signal().dedupe(), |mut style, color| {
                if let Some(section) = style.sections.first_mut() {
                    section.style.color = color;
                }
            });
        Self {
            el,
            text,
            font,
            font_size,
            color,
        }
    }
}

pub struct HighlightableText {
    pub text: DynamicText,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    highlighted: Mutable<bool>,
}

impl_syncers!(HighlightableText {
    highlighted_color: Color,
    unhighlighted_color: Color,
    highlighted: bool
});

impl ElementWrapper for HighlightableText {
    type EL = DynamicText;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.text
    }
}

impl PointerEventAware for HighlightableText {}

impl HighlightableText {
    fn new(text: impl Into<String>) -> Self {
        let unhighlighted_color = Mutable::new(DEFAULT_UNHIGHLIGHTED_COLOR);
        let highlighted_color = Mutable::new(DEFAULT_HIGHLIGHTED_COLOR);
        let hovered = Mutable::new(false);
        let highlighted = Mutable::new(false);
        let dynamic_text = DynamicText::new()
            .color_signal(
                signal::or(hovered.signal(), highlighted.signal()).map_bool_signal(
                    clone!((highlighted_color) move || highlighted_color.signal()),
                    clone!((unhighlighted_color) move || unhighlighted_color.signal()),
                ),
            )
            .cursor(CursorIcon::Pointer)
            .hovered_sync(hovered);
        Self {
            text: dynamic_text,
            highlighted_color,
            unhighlighted_color,
            highlighted,
        }
        .with_text(|dynamic_text| dynamic_text.text(text.into()))
    }

    fn with_text(mut self, f: impl FnOnce(DynamicText) -> DynamicText) -> Self {
        self.text = f(self.text);
        self
    }
}

struct EntityElement {
    el: Column<NodeBundle>,
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

impl_syncers!(EntityElement {
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
});

impl ElementWrapper for EntityElement {
    type EL = Column<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl EntityElement {
    fn new(entity: Entity, EntityData { name, expanded }: EntityData) -> Self {
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
        let components = MutableBTreeMap::new();
        let el = {
            Column::<NodeBundle>::new()
            .update_raw_el(clone!((components, expanded) move |raw_el| {
                raw_el
                .observe(clone!((components => components_map) move |event: Trigger<ComponentsAdded>, components: &Components| {
                    let ComponentsAdded(added) = event.event();
                    let mut lock = components_map.lock_mut();
                    for &component in added {
                        if let Some(info) = components.get_info(component) {
                            let name = pretty_type_name::pretty_type_name_str(info.name());
                            lock.insert_cloned(component, ComponentData { name, viewable: Mutable::new(false) });
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
                    expanded.signal().map_true(move || SyncComponents{ entity: entity, components: HashSet::from_iter(components.lock_ref().iter().map(|(&id, _)| id))}),
                )
            }))
            .apply(column_style(row_gap.signal()))
            .item(
                HighlightableText::new(match name {
                    Some(name) => format!("{name} ({entity})"),
                    None => format!("Entity ({entity})"),
                })
                .with_text(|text| text.font_size_signal(font_size.signal()))
                .highlighted_color_signal(highlighted_color.signal())
                .unhighlighted_color_signal(unhighlighted_color.signal())
                .on_click(clone!((expanded) move || flip(&expanded))),
            )
            .item_signal(expanded.signal().map_true(clone!((row_gap, column_gap, primary_background_color, secondary_background_color, border_width, border_color, padding, highlighted_color, unhighlighted_color) move || {
                Column::<NodeBundle>::new()
                    .apply(column_style(row_gap.signal()))
                    .apply(horizontal_padding_style(padding.signal()))
                    .apply(left_bordered_style(border_width.signal(), border_color.signal()))
                    .items_signal_vec(
                        components.entries_cloned()
                        .map_signal(|(component, data)| {
                            data.viewable.signal().map(move |cur| (component, data.clone(), cur))
                        })
                        .sort_by_cloned(|(_, ComponentData { name: left_name, .. }, left_viewable), (_, ComponentData { name: right_name, .. }, right_viewable)| left_viewable.cmp(right_viewable).reverse().then(left_name.cmp(right_name)))
                        .map(clone!((row_gap, column_gap, primary_background_color, secondary_background_color, border_width, border_color, padding, highlighted_color, unhighlighted_color) move |(component, ComponentData { name, viewable }, _)|
                            FieldElement::new(entity, component, FieldType::Component(name), viewable)
                            .row_gap_signal(row_gap.signal())
                            .column_gap_signal(column_gap.signal())
                            .type_path_color_signal(secondary_background_color.signal())
                            .border_width_signal(border_width.signal())
                            .border_color_signal(border_color.signal())
                            .padding_signal(padding.signal())
                            .highlighted_color_signal(highlighted_color.signal())
                            .unhighlighted_color_signal(unhighlighted_color.signal())
                        ))
                    )
            })))
        };
        Self {
            el,
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
            expanded,
        }
    }
}

fn all_padding_style<E: Element>(
    padding: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    move |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Style>(
                padding.dedupe().map(Val::Px).map(UiRect::all),
                |mut style, padding| style.padding = padding,
            )
        })
    }
}

fn vertical_padding_style<E: Element>(
    padding: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    move |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Style>(
                padding.dedupe().map(Val::Px),
                |mut style, padding| {
                    style.padding.top = padding;
                    style.padding.bottom = padding;
                },
            )
        })
    }
}

fn horizontal_padding_style<E: Element>(
    padding: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    move |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Style>(
                padding.dedupe().map(Val::Px),
                |mut style, padding| {
                    style.padding.left = padding;
                    style.padding.right = padding;
                },
            )
        })
    }
}

fn left_bordered_style<E: Element>(
    border_width: impl Signal<Item = f32> + Send + 'static,
    border_color: impl Signal<Item = Color> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el
                .component_signal::<BorderColor, _>(border_color.dedupe().map(BorderColor))
                .on_signal_with_component::<_, Style>(
                    border_width.dedupe().map(Val::Px),
                    |mut style, width| style.border.left = width,
                )
        })
    }
}

fn square_style<E: Sizeable>(
    size: impl Signal<Item = f32> + Send + Sync + 'static,
) -> impl FnOnce(E) -> E {
    move |el| {
        let size = size.dedupe().map(Val::Px).broadcast();
        el.height_signal(size.signal()).width_signal(size.signal())
    }
}

fn outline_style<E: Element>(
    active: impl Signal<Item = bool> + Send + 'static,
    width: impl Signal<Item = f32> + Send + Sync + 'static,
    offset: impl Signal<Item = f32> + Send + Sync + 'static,
    color: impl Signal<Item = Color> + Send + Sync + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            let width = width.dedupe().map(Val::Px).broadcast();
            let offset = offset.dedupe().map(Val::Px).broadcast();
            let color = color.dedupe().broadcast();
            raw_el.component_signal::<Outline, _>(active.dedupe().map_true_signal(move || {
                map_ref! {
                    let &width = width.signal(),
                    let &offset = offset.signal(),
                    let &color = color.signal()
                    => Outline {
                        width: width,
                        offset: offset,
                        color: color,
                    }
                }
            }))
        })
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

impl_syncers!(FieldElement {
    row_gap: f32,
    column_gap: f32,
    border_width: f32,
    border_color: Color,
    padding: f32,
    highlighted_color: Color,
    unhighlighted_color: Color,
    type_path_color: Color,
    expanded: bool,
});

impl ElementWrapper for FieldElement {
    type EL = Column<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

#[derive(Clone)]
enum NodeType {
    Solo(String), // type path
    Multi(MutableVec<AccessFieldData>),
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
        let component_root = matches!(field_type, FieldType::Component(_));
        let el = Column::<NodeBundle>::new()
            .apply(column_style(row_gap.signal()))
            .update_raw_el(|raw_el| {
                raw_el.on_spawn(clone!((viewable, expanded, node_type, type_path) move |world, ui_entity| {
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
                            bevy::reflect::ReflectRef::Struct(struct_) => {
                                let mut fields = vec![];
                                for i in 0..struct_.field_len() {
                                    if let Some(name) = struct_.name_at(i) {
                                        let access = Access::Field(name.to_string().into());
                                        fields.push(AccessFieldData::new(access));
                                    }
                                }
                                node_type.set(Some(NodeType::Multi(fields.into())));
                                set_viewable = true;
                            },
                            bevy::reflect::ReflectRef::TupleStruct(tuple_struct) => {
                                let mut fields = vec![];
                                for i in 0..tuple_struct.field_len() {
                                    let access = Access::TupleIndex(i);
                                    fields.push(AccessFieldData::new(access));
                                }
                                node_type.set(Some(NodeType::Multi(fields.into())));
                                set_viewable = true;
                            },
                            bevy::reflect::ReflectRef::Tuple(tuple) => {
                                let mut fields = vec![];
                                for i in 0..tuple.field_len() {
                                    let access = Access::TupleIndex(i);
                                    fields.push(AccessFieldData::new(access));
                                }
                                node_type.set(Some(NodeType::Multi(fields.into())));
                                set_viewable = true;
                            },
                            bevy::reflect::ReflectRef::List(list) => {
                                let mut fields = vec![];
                                for i in 0..list.len() {
                                    let access = Access::ListIndex(i);
                                    fields.push(AccessFieldData::new(access));
                                }
                                node_type.set(Some(NodeType::Multi(fields.into())));
                                set_viewable = true;
                            },
                            bevy::reflect::ReflectRef::Array(array) => {
                                let mut fields = vec![];
                                for i in 0..array.len() {
                                    let access = Access::ListIndex(i);
                                    fields.push(AccessFieldData::new(access));
                                }
                                node_type.set(Some(NodeType::Multi(fields.into())));
                                set_viewable = true;
                            },
                            bevy::reflect::ReflectRef::Map(map) => {
                                // TODO 
                            },
                            bevy::reflect::ReflectRef::Enum(enum_) => {
                                // TODO
                            },
                            bevy::reflect::ReflectRef::Value(value) => {
                                let type_path = value.reflect_short_type_path();
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
                    clone!((name, highlighted_color, unhighlighted_color, hovered) move || HighlightableText::new(name.clone())
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
                        hovered.signal().map_true_signal(clone!((type_path_color) move || type_path.signal_cloned().map_some(clone!((type_path_color) move |type_path| {
                            DynamicText::new()
                            .text(type_path)
                            .color_signal(type_path_color.signal())
                        }))))
                        .map(Option::flatten)
                        .boxed()
                    }
                )
            })
            .item_signal(expanded.signal().map_true(
                clone!((row_gap, border_width, border_color, padding, highlighted_color, unhighlighted_color, type_path_color, viewable) move || {
                    El::<NodeBundle>::new()
                    .apply(nested_fields_style(row_gap.signal(), padding.signal(), border_width.signal(), border_color.signal()))
                    .child_signal(
                        node_type.signal_cloned()
                        .map_some(clone!((row_gap, border_width, border_color, padding, highlighted_color, unhighlighted_color, type_path_color, viewable) move |node_type| match node_type {
                            NodeType::Solo(type_path) => {
                                let el_option = field(&type_path).map(TypeEraseable::type_erase);
                                if el_option.is_some() {
                                    viewable.set_neq(true);
                                }
                                el_option
                            },
                            NodeType::Multi(fields) => {
                                Column::<NodeBundle>::new()
                                .apply(column_style(row_gap.signal()))
                                .items_signal_vec(
                                    fields.signal_vec_cloned()
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
                            if let Some(access) = access_option.clone() {
                                el_option = el_option.map(|el| el.update_raw_el(|raw_el| raw_el.insert(Accessory { entity, component, access })))
                            }
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
}

fn field(type_path: &str) -> Option<impl Element> {
    match type_path {
        "bool" => Some(bool_field()),
        _ => None,
    }
}

pub struct Checkbox {
    el: El<NodeBundle>,
    size: Mutable<f32>,
    background_color: Mutable<Color>,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    border_radius: Mutable<f32>,
    hovered: Mutable<bool>,
    checked: Mutable<bool>,
}

impl_syncers!(Checkbox {
    size: f32,
    background_color: Color,
    highlighted_color: Color,
    unhighlighted_color: Color,
    border_radius: f32,
    hovered: bool,
    checked: bool,
});

// implementing `ElementWrapper` allows the struct to be passed directly to .child methods
impl ElementWrapper for Checkbox {
    type EL = El<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Sizeable for Checkbox {}
impl PointerEventAware for Checkbox {}
impl Nameable for Checkbox {}

impl Checkbox {
    pub fn new() -> Self {
        let size = GLOBAL_FONT_SIZE.clone();
        let background_color = GLOBAL_SECONDARY_BACKGROUND_COLOR.clone();
        let highlighted_color = GLOBAL_HIGHLIGHTED_COLOR.clone();
        let unhighlighted_color = GLOBAL_UNHIGHLIGHTED_COLOR.clone();
        let border_radius = Mutable::new(5.);
        let hovered = Mutable::new(false);
        let checked = Mutable::new(false);
        let el = El::<NodeBundle>::new()
            .align_content(Align::center())
            .apply(square_style(size.signal()))
            .apply(border_radius_style(border_radius.signal()))
            .apply(border_style(always(1.), hovered.signal().map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), || GLOBAL_PRIMARY_BACKGROUND_COLOR.signal())))
            .apply(background_color_style(background_color.signal()))
            .hovered_sync(hovered.clone())
            .cursor(CursorIcon::Pointer)
            .child_signal(
                checked.signal()
                .map_true(clone!((size, hovered, highlighted_color, unhighlighted_color, border_radius) move ||
                    El::<NodeBundle>::new()
                        .apply(border_radius_style(border_radius.signal().map(|radius| radius * 0.5)))
                        .apply(square_style(size.signal().map(|size| size * 0.6)))
                        .apply(background_color_style(hovered.signal().map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), clone!((unhighlighted_color) move || unhighlighted_color.signal()))))
                ))
            );
        Self {
            el,
            size,
            background_color,
            highlighted_color,
            unhighlighted_color,
            border_radius,
            hovered,
            checked,
        }
    }
}

fn background_color_style<E: Element>(
    background_color: impl Signal<Item = Color> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el.component_signal::<BackgroundColor, _>(
                background_color.dedupe().map(BackgroundColor),
            )
        })
    }
}

fn border_radius_style<E: Element>(
    border_radius: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el.component_signal::<BorderRadius, _>(
                border_radius.dedupe().map(Val::Px).map(BorderRadius::all),
            )
        })
    }
}

fn border_style<E: Element>(
    border_width: impl Signal<Item = f32> + Send + 'static,
    border_color: impl Signal<Item = Color> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el
                .component_signal::<BorderColor, _>(border_color.dedupe().map(BorderColor))
                .on_signal_with_component::<_, Style>(
                    border_width.dedupe().map(Val::Px).map(UiRect::all),
                    |mut style, width| style.border = width,
                )
        })
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
        if let Ok(accessory) = accessories.get(parent) {
            path.push(accessory.access.clone());
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

fn bool_field() -> impl Element {
    let checked: Mutable<bool> = Mutable::new(false);
    Checkbox::new()
        .checked_signal(checked.signal())
        .on_click_with_system(clone!((checked) move |In((ui_entity, _)),
         accessories: Query<&Accessory>,
         parents: Query<&Parent>,
         sync_components: Query<&SyncComponents>,
         mut field_path_cache: ResMut<FieldPathCache>,
         mut commands: Commands| {
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

#[derive(Component, Clone)]
struct FieldListener {
    handler: SystemId<Box<dyn Reflect>>,
}

#[derive(Component, Clone)]
struct Accessory {
    entity: Entity,
    component: ComponentId,
    access: Access<'static>,
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
    .observe(|event: Trigger<EntitiesAdded>, names: Query<DebugName>| {
        let mut entities = ENTITIES.lock_mut();
        let EntitiesAdded(added) = event.event();
        for &entity in added {
            entities.insert_cloned(
                entity,
                EntityData {
                    name: names.get(entity).ok().and_then(|name| name.name).cloned(),
                    expanded: Mutable::new(false),
                },
            );
        }
    })
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
