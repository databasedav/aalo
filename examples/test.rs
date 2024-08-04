use std::collections::HashSet;

use bevy::{
    ecs::{
        component::ComponentId,
        system::{SystemId, SystemState},
    },
    prelude::*,
    reflect::{Access, ParsedPath, ReflectFromPtr},
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use haalka::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
            HaalkaPlugin,
            // WorldInspectorPlugin::new(),
        ))
        .register_type::<BoolComponent>()
        .add_systems(Startup, (camera, ui_root /* entity_syncer */))
        .add_systems(
            Update,
            (
                entity_syncer,
                sync_components.run_if(any_with_component::<SyncComponents>),
                sync_ui.run_if(any_with_component::<FieldListener>),
            ),
        )
        .observe(|event: Trigger<EntitiesAdded>, names: Query<DebugName>| {
            let mut entities = ENTITIES.lock_mut();
            let EntitiesAdded(added) = event.event();
            for &entity in added {
                entities.insert_cloned(
                    entity,
                    EntityData {
                        name: names.get(entity).ok().and_then(|name| name.name).cloned(),
                    },
                );
            }
        })
        .observe(|event: Trigger<EntitiesRemoved>| {
            let mut entities = ENTITIES.lock_mut();
            let EntitiesRemoved(removed) = event.event();
            for entity in removed {
                entities.remove(entity);
            }
        })
        .init_resource::<EntitySet>()
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const CLICKED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
const LIL_BABY_BUTTON_SIZE: f32 = 30.;
const DEFAULT_BUTTON_HEIGHT: f32 = 65.;
const BASE_BORDER_WIDTH: f32 = 5.;

#[derive(Event)]
struct EntitiesAdded(Vec<Entity>);

#[derive(Event)]
struct EntitiesRemoved(Vec<Entity>);

#[derive(Event)]
struct ComponentsAdded(Vec<ComponentId>);

#[derive(Event)]
struct ComponentsRemoved(Vec<ComponentId>);

struct Checkbox {
    el: Button,
}

#[derive(Component)]
struct CheckboxData {
    checked: Mutable<bool>,
}

struct Button {
    el: Stack<NodeBundle>,
    selected: Mutable<bool>,
    hovered: Mutable<bool>,
}

// implementing `ElementWrapper` allows the struct to be passed directly to .child methods
impl ElementWrapper for Button {
    type EL = Stack<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Sizeable for Button {}
impl PointerEventAware for Button {}
impl Nameable for Button {}

impl Button {
    fn new() -> Self {
        let (selected, selected_signal) = Mutable::new_and_signal(false);
        let (pressed, pressed_signal) = Mutable::new_and_signal(false);
        let (hovered, hovered_signal) = Mutable::new_and_signal(false);
        let selected_hovered_broadcaster = map_ref!(selected_signal, pressed_signal, hovered_signal => (*selected_signal || *pressed_signal, *hovered_signal)).broadcast();
        let border_color_signal = {
            selected_hovered_broadcaster
                .signal()
                .map(|(selected, hovered)| {
                    if selected {
                        bevy::color::palettes::basic::RED.into()
                    } else if hovered {
                        Color::BLACK
                    } else {
                        Color::WHITE
                    }
                })
                .map(BorderColor)
        };
        let background_color_signal = {
            selected_hovered_broadcaster
                .signal()
                .map(|(selected, hovered)| {
                    if selected {
                        CLICKED_BUTTON
                    } else if hovered {
                        HOVERED_BUTTON
                    } else {
                        NORMAL_BUTTON
                    }
                })
                .map(BackgroundColor)
        };
        Self {
            el: {
                Stack::<NodeBundle>::new()
                    .update_raw_el(|raw_el| raw_el.insert(Name::new("checkbox")))
                    .height(Val::Px(DEFAULT_BUTTON_HEIGHT))
                    .with_style(|mut style| {
                        style.border = UiRect::all(Val::Px(BASE_BORDER_WIDTH));
                    })
                    .pressed_sync(pressed)
                    .align_content(Align::center())
                    // .hovered_sync(hovered.clone())
                    .border_color_signal(border_color_signal)
                    .background_color_signal(background_color_signal)
                    .layer({
                        let hovered = Mutable::new(false);
                        El::<NodeBundle>::new()
                            .with_style(|mut style| style.top = Val::Px(-BASE_BORDER_WIDTH))
                            .height(Val::Px(BASE_BORDER_WIDTH))
                            .width(Val::Px(LIL_BABY_BUTTON_SIZE))
                            .align(Align::new().center_x().top())
                            .background_color_signal(
                                hovered
                                    .signal()
                                    .map_bool(
                                        || Color::WHITE,
                                        || bevy::color::palettes::basic::GREEN.into(),
                                    )
                                    .map(BackgroundColor),
                            )
                            .hovered_sync(hovered)
                    })
            },
            selected,
            hovered,
        }
    }

    fn body(mut self, body: impl Element) -> Self {
        self.el = self.el.layer(body);
        self
    }

    fn selected_signal(
        mut self,
        selected_signal: impl Signal<Item = bool> + Send + 'static,
    ) -> Self {
        // syncing mutables like this is a helpful pattern for externally controlling reactive state that
        // has default widget-internal behavior; for example, all buttons are selected on press, but
        // what if we want the selectedness to persist? simply add another mutable that gets flipped
        // on click and then pass a signal of that to this method, which is exactly how the
        // `Checkbox` widget is implemented
        let syncer = spawn(sync(self.selected.clone(), selected_signal));
        self.el = self.el.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]));
        self
    }

    fn hovered_signal(mut self, hovered_signal: impl Signal<Item = bool> + Send + 'static) -> Self {
        let syncer = spawn(sync(self.hovered.clone(), hovered_signal));
        self.el = self.el.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]));
        self
    }
}

// TODO: make this a public util ?
async fn sync<T>(mutable: Mutable<T>, signal: impl Signal<Item = T> + Send + 'static) {
    signal.for_each_sync(|value| mutable.set(value)).await;
}

impl Checkbox {
    fn new(checked: Mutable<bool>) -> Self {
        Self {
            el: {
                Button::new()
                    .name("checkbox")
                    .width(Val::Px(LIL_BABY_BUTTON_SIZE))
                    .height(Val::Px(LIL_BABY_BUTTON_SIZE))
                    .on_click(clone!((checked) move || { checked.update(|c| !c) }))
                    .selected_signal(checked.signal())
                    .into_element()
            },
        }
    }
}

impl ElementWrapper for Checkbox {
    type EL = Button;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Clone)]
struct EntityData {
    name: Option<Name>,
}

static ENTITIES: Lazy<MutableBTreeMap<Entity, EntityData>> = Lazy::new(default);

const DEFAULT_FONT_SIZE: f32 = 20.;
const DEFAULT_ROW_GAP: f32 = 5.;
const DEFAULT_PADDING: f32 = 10.;
const DEFAULT_BORDER_RADIUS: f32 = 10.;
const DEFAULT_BORDER_WIDTH: f32 = 2.;

const DEFAULT_BACKGROUND_COLOR: Color = Color::srgb(27. / 255., 27. / 255., 27. / 255.);
const DEFAULT_HIGHLIGHTED_COLOR: Color = Color::srgb(210. / 255., 210. / 255., 210. / 255.);
const DEFAULT_UNHIGHLIGHTED_COLOR: Color = Color::srgb(150. / 255., 150. / 255., 150. / 255.);
const DEFAULT_BORDER_COLOR: Color = Color::srgb(56. / 255., 56. / 255., 56. / 255.);

fn inspector() -> impl Element {
    Column::<NodeBundle>::new()
        .name("inspector")
        .with_style(|mut style| {
            style.row_gap = Val::Px(DEFAULT_ROW_GAP);
            style.padding = UiRect::all(Val::Px(DEFAULT_PADDING));
            style.border = UiRect::all(Val::Px(DEFAULT_BORDER_WIDTH));
        })
        .scrollable_on_hover(ScrollabilitySettings {
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip_y(),
            scroll_handler: BasicScrollHandler::new()
                .direction(ScrollDirection::Vertical)
                .pixels(20.)
                .into(),
        })
        .height(Val::Percent(40.))
        .width(Val::Percent(30.))
        .cursor(CursorIcon::Default)
        .border_color(BorderColor(DEFAULT_BORDER_COLOR))
        .border_radius(BorderRadius::all(Val::Px(DEFAULT_BORDER_RADIUS)))
        .background_color(BackgroundColor::from(DEFAULT_BACKGROUND_COLOR))
        .items_signal_vec(
            ENTITIES
                .entries_cloned()
                .filter(|(_, data)| data.name == Some(Name::from("ui root")))
                .map(|(id, data)| entity(id, data)),
        )
}

#[derive(Clone)]
struct ComponentData {
    name: String,
    registered: bool,
}

#[derive(Component)]
struct Components(MutableBTreeMap<ComponentId, ComponentData>);

#[derive(Component)]
struct SyncComponents {
    entity: Entity,
    components: HashSet<ComponentId>,
}

fn entity(entity: Entity, EntityData { name }: EntityData) -> impl Element {
    let expanded = Mutable::new(true);
    let components = MutableBTreeMap::new();
    Column::<NodeBundle>::new()
        .update_raw_el(clone!((components, expanded) move |raw_el| {
            raw_el
            .observe(clone!((components) move |event: Trigger<ComponentsAdded>, mut commands: Commands| {
                let ComponentsAdded(added) = event.event();
                let added = added.clone();
                commands.add(clone!((components) move |world: &mut World| {
                    if let Some(type_registry) = world.get_resource::<AppTypeRegistry>() {
                        let type_registry = type_registry.read();
                        let mut lock = components.lock_mut();
                        for id in added {
                            if let Some(info) = world.components().get_info(id) {
                                let name = pretty_type_name::pretty_type_name_str(info.name());
                                let registered = info.type_id().and_then(|type_id| type_registry.get(type_id)).is_some();
                                lock.insert_cloned(id, ComponentData { name, registered });
                            }
                        }
                    }
                }));
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
        .with_style(|mut style| {
            style.row_gap = Val::Px(DEFAULT_ROW_GAP);
        })
        .item(
            highlightable_text(match name {
                Some(name) => format!("{name} ({entity})"),
                None => format!("Entity ({entity})"),
            })
            .on_click(clone!((expanded) move || flip(&expanded))),
        )
        .item_signal(expanded.signal().map_true(move || {
            Column::<NodeBundle>::new()
                .border_color(BorderColor(DEFAULT_BORDER_COLOR))
                .with_style(|mut style| {
                    style.row_gap = Val::Px(DEFAULT_ROW_GAP);
                    style.padding = UiRect::default().with_left(Val::Px(DEFAULT_PADDING));
                    style.border = UiRect::default().with_left(Val::Px(DEFAULT_BORDER_WIDTH));
                })
                .items_signal_vec(
                    components.entries_cloned()
                    .sort_by_cloned(|(_, ComponentData { name: left_name, registered: left_registered }), (_, ComponentData { name: right_name, registered: right_registered })| left_registered.cmp(right_registered).reverse().then(left_name.cmp(right_name)))
                    .map(move |(component_id, data)| component(entity, component_id, data)))
        }))
}

fn field(type_path: &str) -> Option<impl Element> {
    match type_path {
        "bool" => Some(bool_field()),
        _ => None,
    }
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

#[derive(Clone)]
struct FieldData {
    type_path: String,
    access: Access<'static>,
}

fn reflect(world: &mut World, entity: Entity, component: ComponentId) -> Option<&dyn Reflect> {
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

fn reflect_mut<'w>(
    entity: &'w mut EntityWorldMut,
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
                return Some(unsafe { reflect_from_ptr.as_reflect_mut(component_ptr.into_inner()) });
            }
        }
    }
    None
}

fn component(
    entity: Entity,
    component: ComponentId,
    ComponentData { name, registered }: ComponentData,
) -> impl Element {
    let expanded = Mutable::new(true);
    let fields = MutableVec::new();
    Column::<NodeBundle>::new()
        .with_style(|mut style| {
            style.row_gap = Val::Px(DEFAULT_ROW_GAP);
        })
        .update_raw_el(|raw_el| {
            raw_el.on_spawn(clone!((fields) move |world, _| {
                if let Some(reflect) = reflect(world, entity, component) {
                    match reflect.reflect_ref() {
                        bevy::reflect::ReflectRef::TupleStruct(tuple_struct) => {
                            for (index, field) in tuple_struct.iter_fields().enumerate() {
                                println!(
                                    "  {}: {:?}",
                                    index,
                                    field.reflect_type_path()
                                );
                                let type_path = field.reflect_type_path().to_string();
                                let access = Access::TupleIndex(index);
                                fields.lock_mut().push_cloned(FieldData { type_path, access });
                            }
                        },
                        bevy::reflect::ReflectRef::Struct(struct_ref) => {
                            for (index, field) in struct_ref.iter_fields().enumerate() {
                                println!(
                                    "  {}: {:?}",
                                    struct_ref.name_at(index).unwrap(),
                                    field.reflect_type_path()
                                );
                            }
                        },
                        _ => ()
                    }
                }
            }))
        })
        .item(match registered {
            true => highlightable_text(name).on_click(clone!((expanded) move || flip(&expanded))),
            false => El::<TextBundle>::new().text(colored_text(
                name,
                bevy::color::palettes::basic::MAROON.into(),
            )),
        })
        .item_signal(expanded.signal().map_true(move || {
            Column::<NodeBundle>::new()
                .border_color(BorderColor(DEFAULT_BORDER_COLOR))
                .with_style(|mut style| {
                    style.row_gap = Val::Px(DEFAULT_ROW_GAP);
                    style.padding = UiRect::default().with_left(Val::Px(DEFAULT_PADDING));
                    style.border = UiRect::default().with_left(Val::Px(DEFAULT_BORDER_WIDTH));
                })
                .items_signal_vec(fields.signal_vec_cloned().map(
                    move |FieldData { type_path, access }| {
                        field(&type_path).map(|el| {
                            el.update_raw_el(move |raw_el| {
                                raw_el.insert(Accessory {
                                    entity,
                                    component,
                                    access,
                                })
                            })
                        })
                    },
                ))
        }))
}

fn sync_ui(
    accessories: Query<&Accessory>,
    parents: Query<&Parent>,
    sync_components: Query<&SyncComponents>,
    field_listeners: Query<(Entity, &FieldListener)>,
    mut commands: Commands,
) {
    for (ui_entity, &FieldListener { handler }) in field_listeners.iter() {
        if let Ok(Accessory {
            entity, component, ..
        }) = accessories.get(ui_entity).cloned()
        {
            let field_path = field_path(ui_entity, &accessories, &parents, &sync_components);
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
}

fn sync_components(world: &mut World) {
    let mut system_state: SystemState<Query<(Entity, &SyncComponents)>> = SystemState::new(world);
    let entities = system_state
        .get_mut(world)
        .iter()
        .map(|(ui_entity, &SyncComponents { entity, .. })| (ui_entity, entity))
        .collect::<Vec<_>>();
    for (ui_entity, entity) in entities {
        if let Some(new) = world
            .get_entity(entity)
            .map(|entity| entity.archetype().components().collect::<HashSet<_>>())
        {
            if let Some(mut entity) = world.get_entity_mut(ui_entity) {
                if let Some(mut sync_components) = entity.get_mut::<SyncComponents>() {
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
                        entity.world_scope(|world| {
                            world.trigger_targets(ComponentsAdded(added), ui_entity)
                        });
                    }
                    if !removed.is_empty() {
                        entity.world_scope(|world| {
                            world.trigger_targets(ComponentsRemoved(removed), ui_entity)
                        });
                    }
                }
            }
        }
    }
}

fn highlightable_text(t: impl ToString) -> El<TextBundle> {
    let hovered = Mutable::new(false);
    El::<TextBundle>::new()
        .cursor(CursorIcon::Pointer)
        .text(text(t))
        .on_signal_with_text(
            hovered
                .signal()
                .map_bool(|| DEFAULT_HIGHLIGHTED_COLOR, || DEFAULT_UNHIGHLIGHTED_COLOR),
            |mut text, color| text.sections[0].style.color = color,
        )
        .hovered_sync(hovered)
}

fn text(text: impl ToString) -> Text {
    Text::from_section(
        text.to_string(),
        TextStyle {
            font_size: DEFAULT_FONT_SIZE,
            ..default()
        },
    )
}

fn colored_text(text: impl ToString, color: Color) -> Text {
    Text::from_section(
        text.to_string(),
        TextStyle {
            font_size: DEFAULT_FONT_SIZE,
            color,
            ..default()
        },
    )
}

#[derive(Component, Reflect)]
struct BoolComponent(bool);

fn field_path(
    entity: Entity, // field's ui entity
    accessories: &Query<&Accessory>,
    parents: &Query<&Parent>,
    sync_components: &Query<&SyncComponents>,
) -> ParsedPath {
    let mut path = vec![];
    if let Ok(Accessory { access, .. }) = accessories.get(entity).cloned() {
        path.push(access.clone());
        for parent in parents.iter_ancestors(entity) {
            if let Ok(accessory) = accessories.get(parent) {
                path.push(accessory.access.clone());
            }
            if sync_components.contains(parent) {
                // marks entity root
                break;
            }
        }
    }
    path.reverse();
    ParsedPath::from(path)
}

fn bool_field() -> impl Element {
    let checked = Mutable::new(false);
    Button::new()
        .width(Val::Px(LIL_BABY_BUTTON_SIZE))
        .height(Val::Px(LIL_BABY_BUTTON_SIZE))
        .on_click_with_system(clone!((checked) move |In((ui_entity, _)),
        accessories: Query<&Accessory>,
         parents: Query<&Parent>,
         sync_components: Query<&SyncComponents>, mut commands: Commands| {
             if let Ok(Accessory { entity, component, .. }) = accessories.get(ui_entity).cloned() {
                let field_path = field_path(ui_entity, &accessories, &parents, &sync_components);
                commands.add(clone!((checked) move |world: &mut World| {
                    if let Some(mut entity) = world.get_entity_mut(entity) {
                        if let Some(reflect) = reflect_mut(&mut entity, component) {
                            if let Ok(target) = reflect.reflect_path_mut(&field_path) {
                                let _ = target.try_apply((!checked.get()).as_reflect());
                            }
                        }
                    }
                }));
            }
        }))
        .selected_signal(checked.signal())
        .update_raw_el(|raw_el| {
            raw_el.with_entity(|mut entity| {
                let handler = entity.world_scope(|world| {
                    world.register_system(move |In(reflect): In<Box<dyn Reflect>>| {
                        if let Some(&cur) = reflect.downcast_ref::<bool>() {
                            checked.set_neq(cur);
                        }
                    })
                });
                entity.insert(FieldListener { handler });
            })
        })
}

fn ui_root(world: &mut World) {
    El::<NodeBundle>::new()
        .width(Val::Percent(100.))
        .height(Val::Percent(100.))
        .align_content(Align::center())
        .name("ui root")
        .update_raw_el(|raw_el| raw_el.insert(BoolComponent(true)))
        .child(
            Stack::<NodeBundle>::new()
                .width(Val::Percent(100.))
                .height(Val::Percent(100.))
                .name("stuff stack")
                .layer(inspector().align(Align::new().top().left()))
                .layer(Checkbox::new(Mutable::new(false)).align(Align::center())),
        )
        .spawn(world);
}

#[derive(Resource, Default)]
struct EntitySet(HashSet<Entity>);

fn entity_syncer(
    query: Query<
        Entity,
        (
            Without<Parent>,
            Without<HaalkaOneShotSystem>,
            Without<HaalkaObserver>,
        ),
    >,
    mut entity_set: ResMut<EntitySet>,
    mut commands: Commands,
) {
    let new = query.into_iter().collect::<HashSet<_>>();
    let added = new.difference(&entity_set.0).copied().collect::<Vec<_>>();
    let removed = entity_set.0.difference(&new).copied().collect::<Vec<_>>();
    entity_set.0 = new;
    if !added.is_empty() {
        commands.trigger(EntitiesAdded(added));
    }
    if !removed.is_empty() {
        commands.trigger(EntitiesRemoved(removed));
    }
}
