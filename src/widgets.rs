use super::{defaults::*, globals::*, style::*, utils::*};
use crate::impl_syncers;
use bevy_asset::prelude::*;
use bevy_color::prelude::*;
use bevy_ecs::{prelude::*, system::SystemId};
use bevy_hierarchy::*;
use bevy_text::prelude::*;
use bevy_ui::prelude::*;
use haalka::{prelude::*, raw::utils::remove_system_holder_on_remove};
use std::{
    fmt::Display,
    sync::{Arc, OnceLock},
};

#[derive(Default)]
pub struct DynamicText {
    el: El<Text>,
    text: Mutable<String>,
    font: Mutable<Handle<Font>>,
    font_size: Mutable<f32>,
    color: Mutable<Color>,
}

impl ElementWrapper for DynamicText {
    type EL = El<Text>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl CursorOnHoverable for DynamicText {}
impl GlobalEventAware for DynamicText {}
impl PointerEventAware for DynamicText {}

impl DynamicText {
    pub fn new() -> Self {
        let text = Mutable::new(String::new());
        let font = Mutable::new(Default::default());
        let font_size = Mutable::new(DEFAULT_FONT_SIZE);
        let color = Mutable::new(DEFAULT_UNHIGHLIGHTED_COLOR);
        let el: El<Text> = El::<Text>::new()
            .text_font(
                TextFont::default()
                    .with_font(font.get_cloned())
                    .with_font_size(font_size.get()),
            )
            .text_color(TextColor(color.get()))
            .text(Text(text.get_cloned()))
            .text_signal(text.signal_cloned().dedupe_cloned().map(Text))
            .on_signal_with_text_font(font.signal_cloned(), |mut text_font, font| {
                text_font.font = font;
            })
            .apply(text_style(font_size.signal(), color.signal()));
        Self {
            el,
            text,
            font,
            font_size,
            color,
        }
    }

    impl_syncers! { text: String, font: Handle<Font>, font_size: f32, color: Color }
}

pub struct HighlightableText {
    pub text: DynamicText,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    highlighted: Mutable<bool>,
}

impl ElementWrapper for HighlightableText {
    type EL = DynamicText;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.text
    }
}

impl GlobalEventAware for HighlightableText {}
impl PointerEventAware for HighlightableText {}

impl HighlightableText {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
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
            .hovered_sync(hovered);
        Self {
            text: dynamic_text,
            highlighted_color,
            unhighlighted_color,
            highlighted,
        }
    }

    pub fn with_text(mut self, f: impl FnOnce(DynamicText) -> DynamicText) -> Self {
        self.text = f(self.text);
        self
    }

    impl_syncers! {
        highlighted_color: Color,
        unhighlighted_color: Color,
        highlighted: bool
    }
}

pub struct Checkbox {
    el: El<Node>,
    size: Mutable<f32>,
    background_color: Mutable<Color>,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    border_radius: Mutable<f32>,
    hovered: Mutable<bool>,
    checked: Mutable<bool>,
}

impl ElementWrapper for Checkbox {
    type EL = El<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Sizeable for Checkbox {}
impl GlobalEventAware for Checkbox {}
impl PointerEventAware for Checkbox {}
impl Nameable for Checkbox {}

const CHECKBOX_BORDER_RADIUS_MODIFIER: f32 = 0.333;

impl Checkbox {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let size = GLOBAL_FONT_SIZE.clone();
        let background_color = GLOBAL_SECONDARY_BACKGROUND_COLOR.clone();
        let highlighted_color = GLOBAL_HIGHLIGHTED_COLOR.clone();
        let unhighlighted_color = GLOBAL_UNHIGHLIGHTED_COLOR.clone();
        let border_radius_base = GLOBAL_BORDER_RADIUS.get() * CHECKBOX_BORDER_RADIUS_MODIFIER;
        let border_width = GLOBAL_BORDER_WIDTH.clone();
        let border_radius = Mutable::new(border_radius_base);
        let hovered = Mutable::new(false);
        let checked = Mutable::new(false);
        let el = El::<Node>::new()
            .align_content(Align::center())
            .apply(square_style(size.signal()))
            .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
            .apply(border_width_style(BoxEdge::ALL, border_width.signal()))
            .apply(border_color_style(hovered.signal().map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), || GLOBAL_BORDER_COLOR.signal())))
            .hovered_sync(hovered.clone())
            .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
            .apply(padding_style(BoxEdge::ALL, always(1.)))
            .child_signal(
                checked.signal()
                .map_true(clone!((hovered, highlighted_color, unhighlighted_color, border_radius) move ||
                    El::<Node>::new()
                        .apply(border_radius_style(BoxCorner::ALL, border_radius.signal().map(mul(0.5))))
                        .width(Val::Percent(100.))
                        .height(Val::Percent(100.))
                        .apply(background_style(hovered.signal().map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), clone!((unhighlighted_color) move || unhighlighted_color.signal()))))
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

    impl_syncers! {
        size: f32,
        background_color: Color,
        highlighted_color: Color,
        unhighlighted_color: Color,
        border_radius: f32,
        hovered: bool,
        checked: bool,
    }
}

#[derive(Clone)]
pub struct OptionData<T> {
    pub option: T,
    pub blocked: bool,
    // TODO: just doing this with selected index instead, but this can be used
    // in the future to have a filter integrated into the dropdown
    filtered: Mutable<bool>,
}

impl<T> OptionData<T> {
    pub fn new(option: T, blocked: bool) -> Self {
        Self {
            option,
            blocked,
            filtered: Mutable::new(false),
        }
    }
}

// TODO: these should be horizontally resizable
pub struct Dropdown<T> {
    el: El<Node>,
    options: MutableVec<OptionData<T>>,
    option_handler_system: Arc<OnceLock<SystemId<In<usize>>>>,
    blocked_tooltip: Option<String>,
    selected: Mutable<Option<usize>>,
    show_dropdown: Mutable<bool>,
    font_size: Mutable<f32>,
    padding: Mutable<f32>,
    border_radius: Mutable<f32>,
    border_width: Mutable<f32>,
    background_color: Mutable<Color>,
    highlighted_color: Mutable<Color>,
    unhighlighted_color: Mutable<Color>,
    border_color: Mutable<Color>,
    error_color: Mutable<Color>,
}

impl<T: Clone + PartialEq + Display + Send + Sync + 'static> ElementWrapper for Dropdown<T> {
    type EL = El<Node>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el,
            options,
            option_handler_system,
            blocked_tooltip,
            selected,
            show_dropdown,
            font_size,
            padding,
            border_radius,
            border_width,
            background_color,
            highlighted_color,
            unhighlighted_color,
            border_color,
            error_color,
        } = self;
        let hovered = Mutable::new(false);
        // TODO: more intelligent way to get this height? waiting for node to reach "full size" is pretty cringe
        // TODO: where did this 3. come from ?
        let expected_tooltip_height =
            font_size.get() + padding.get() + border_width.get() * 2. + 3.;
        let blocked_tooltip = Arc::new(blocked_tooltip);
        // TODO: `Stack` does not play well with flex column, using an `El` allows us to avoid managing the height entirely
        el
        .child(
            // TODO: should be able to DRY most of the display element for the dropdown option elements
            El::<Node>::new()
                .apply(border_color_style(signal::and(hovered.signal(), signal::not(show_dropdown.signal())).map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), clone!((border_color) move || border_color.signal()))))
                .apply(border_width_style(BoxEdge::ALL, border_width.signal()))
                // .apply(border_width_style([BoxEdge::Top, BoxEdge::Left, BoxEdge::Right], border_width.signal()))
                // .apply(border_width_style([BoxEdge::Bottom], show_dropdown.signal().map_false_signal(clone!((border_width) move || border_width.signal())).map(Option::unwrap_or_default)))
                .apply(border_radius_style(BoxCorner::TOP, border_radius.signal()))
                .apply(border_radius_style(BoxCorner::BOTTOM, show_dropdown.signal().map_false_signal(clone!((border_radius) move || border_radius.signal())).map(Option::unwrap_or_default)))
                .width(Val::Percent(100.))
                .apply(background_style(background_color.signal()))
                .cursor(CursorIcon::System(SystemCursorIcon::Pointer))
                .on_click(clone!((show_dropdown) move || flip(&show_dropdown)))
                .hovered_sync(hovered.clone())
                .child(
                    El::<Node>::new()
                    .width(Val::Percent(100.))
                    .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
                    .apply(border_style(border_width.signal(), signal::and(show_dropdown.signal(), hovered.signal()).map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), clone!((background_color) move || background_color.signal()))))
                    .apply(padding_style(BoxEdge::ALL, padding.signal()))
                    .child(
                        El::<Text>::new()
                            .text_font_signal(font_size.signal().map(TextFont::from_font_size))
                            .text_color_signal(unhighlighted_color.signal().map(TextColor))
                            .text_signal(
                                selected.signal()
                                .map_some(clone!((options) move |i|
                                    options.signal_vec_cloned().to_signal_map(move |options| options.get(i).cloned())
                                ))
                                .map(signal::option).flatten().map(Option::flatten)
                                .map(|option_option| option_option.map(|option| option.option.to_string()).unwrap_or_default())
                                .map(Text)
                            )
                            .apply(text_style(font_size.signal(), hovered.signal().map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), clone!((unhighlighted_color) move || unhighlighted_color.signal()))))
                    )
                )
        )
        .child_signal(show_dropdown.signal().map_true(
            clone!((options, selected, blocked_tooltip) move || {
                let option_handler_system = option_handler_system.get().copied();
                Column::<Node>::new()
                .global_z_index(GlobalZIndex(z_order("dropdown")))
                .apply(border_color_style(border_color.signal()))
                .width(Val::Percent(100.))
                .with_node(|mut node| node.position_type = PositionType::Absolute)
                .update_raw_el(|raw_el| raw_el.on_spawn_with_system(|In(entity), parents: Query<&Parent>, childrens: Query<&Children>, computed_nodes: Query<&ComputedNode>, mut nodes: Query<&mut Node>| {
                    if let Ok(parent) = parents.get(entity) {
                        if let Ok(siblings) = childrens.get(parent.get()) {
                            if let Some(&sibling) = siblings.first() {
                                if let Ok(sibling_node) = computed_nodes.get(sibling) {
                                    if let Ok(mut node) = nodes.get_mut(entity) {
                                        // TODO: this is not robust to larger font sizes
                                        node.top = Val::Px(sibling_node.size().y + 1.);  // TODO: y do i need this 1. ?
                                    }
                                }
                            }
                        }
                    }
                }))
                .apply(border_width_style([BoxEdge::Left, BoxEdge::Right, BoxEdge::Bottom], border_width.signal()))
                .apply(border_radius_style([BoxCorner::BottomLeft, BoxCorner::BottomRight], border_radius.signal()))
                .apply(background_style(background_color.signal()))
                .align_content(Align::center())
                .items_signal_vec(
                    options.signal_vec_cloned()
                    .enumerate()
                    .sort_by_cloned(|(_, left), (_, right)| left.blocked.cmp(&right.blocked))
                    .filter_signal_cloned(clone!((selected) move |(i, OptionData { filtered, .. })| signal::and(signal::not(signal_eq(i.signal(), selected.signal())), signal::not(filtered.signal()))))
                    .map(clone!((
                        font_size,
                        unhighlighted_color,
                        background_color,
                        padding,
                        border_width,
                        border_radius,
                        highlighted_color,
                        error_color,
                        blocked_tooltip
                    ) move |(i, OptionData { option, blocked, .. })| {
                        let hovered = Mutable::new(false);
                        let mut el = El::<Node>::new()
                            .width(Val::Percent(100.))
                            .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
                            .apply(background_style(background_color.signal()))
                            .apply(padding_style(BoxEdge::ALL, padding.signal()))
                            .apply(border_width_style(BoxEdge::ALL, border_width.signal()))
                            .apply(
                                border_style(
                                    border_width.signal(),
                                    hovered.signal().map_bool_signal(
                                        clone!((error_color, highlighted_color) move || if blocked { error_color.signal() } else {  highlighted_color.signal() }),
                                        clone!((background_color) move || background_color.signal())
                                    )
                                )
                            )
                            .hovered_sync(hovered.clone())
                            .child(
                                El::<Text>::new()
                                .text_font(TextFont::from_font_size(font_size.get()))  // can see text shrink from larger font otherwise
                                .text_font_signal(font_size.signal().map(TextFont::from_font_size))
                                .text_color_signal(
                                    if blocked {
                                        error_color.signal().apply(SignalEither::Left)
                                    } else {
                                        hovered.signal().map_bool_signal(
                                            clone!((highlighted_color) move || highlighted_color.signal()),
                                            clone!((unhighlighted_color) move || unhighlighted_color.signal())
                                        )
                                        .apply(SignalEither::Right)
                                    }
                                    .map(TextColor)
                                )
                                .text(Text(option.to_string()))
                            );
                        if blocked {
                            if let Some(text) = &*blocked_tooltip.clone() {
                                el = el
                                .update_raw_el(|raw_el| {
                                    raw_el
                                    .apply(sync_tooltip_position(expected_tooltip_height))
                                    .on_signal_with_system(
                                        hovered.signal(),
                                        clone!((text) move |
                                            In((entity, hovered)),
                                            mut tooltip_cache: TooltipCache,
                                        | {
                                            if let Some(tooltip) = tooltip_cache.get(entity) {
                                                let data = Some(TooltipData::new(entity, text.clone()));
                                                let mut lock = tooltip.lock_mut();
                                                if hovered {
                                                    if *lock != data {
                                                        *lock = data;
                                                    }
                                                } else if *lock == data {
                                                    *lock = None;
                                                }
                                            }
                                        })
                                    )
                                });
                            }
                        } else {
                            el = el
                            .cursor(CursorIcon::System(SystemCursorIcon::Pointer));
                            if let Some(system) = option_handler_system {
                                el = el
                                .on_click_with_system(move |_: In<_>, mut commands: Commands| {
                                    if let Some(i) = i.get() {
                                        commands.run_system_with_input(system, i);
                                    }
                                });
                            }
                        }
                        el
                    }))
                )
            }),
        ))
    }
}

impl<T: Clone + PartialEq + Display + Send + Sync + 'static> Sizeable for Dropdown<T> {}
impl<T: Clone + PartialEq + Display + Send + Sync + 'static> GlobalEventAware for Dropdown<T> {}
impl<T: Clone + PartialEq + Display + Send + Sync + 'static> PointerEventAware for Dropdown<T> {}

impl<T> Dropdown<T> {
    pub fn new(options: MutableVec<OptionData<T>>) -> Self
    where
        T: Clone + PartialEq + Display + Send + Sync + 'static,
    {
        Self {
            el: El::<Node>::new(),
            options,
            option_handler_system: Arc::new(OnceLock::new()),
            blocked_tooltip: None,
            selected: Mutable::new(None),
            show_dropdown: Mutable::new(false),
            font_size: GLOBAL_FONT_SIZE.clone(),
            padding: Mutable::new(GLOBAL_PADDING.get() * 0.5),
            border_radius: GLOBAL_BORDER_RADIUS.clone(),
            // border_radius: Mutable::new(GLOBAL_BORDER_RADIUS.get() * 0.5),
            border_width: GLOBAL_BORDER_WIDTH.clone(),
            background_color: GLOBAL_PRIMARY_BACKGROUND_COLOR.clone(),
            highlighted_color: GLOBAL_HIGHLIGHTED_COLOR.clone(),
            unhighlighted_color: GLOBAL_UNHIGHLIGHTED_COLOR.clone(),
            border_color: GLOBAL_BORDER_COLOR.clone(),
            error_color: GLOBAL_ERROR_COLOR.clone(),
        }
    }

    impl_syncers! { selected: Option<usize>, show_dropdown: bool, font_size: f32, padding: f32, border_radius: f32, border_width: f32, background_color: Color, highlighted_color: Color, unhighlighted_color: Color, border_color: Color }

    pub fn with_show_dropdown(mut self, show_dropdown: Mutable<bool>) -> Self {
        self.show_dropdown = show_dropdown;
        self
    }

    pub fn sync_selected(self, selected: Mutable<Option<usize>>) -> Self
    where
        Self: ElementWrapper,
    {
        let syncer = spawn(sync_neq(selected.signal(), self.selected.clone()));
        self.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]))
    }

    pub fn option_handler_system<Marker>(
        self,
        handler: impl IntoSystem<In<usize>, (), Marker> + Send + 'static,
    ) -> Self
    where
        Self: ElementWrapper,
    {
        let system_holder = self.option_handler_system.clone();
        self.update_raw_el(|raw_el| {
            raw_el
                .with_entity(clone!((system_holder) move |mut entity| {
                    let handler = entity.world_scope(|world| register_system(world, handler));
                    let _ = system_holder.set(handler);
                }))
                .apply(remove_system_holder_on_remove(system_holder))
        })
    }

    pub fn option_handler(
        self,
        mut option_handler: impl FnMut(usize) + Send + Sync + 'static,
    ) -> Self
    where
        Self: ElementWrapper,
    {
        self.option_handler_system(move |In(i)| option_handler(i))
    }

    pub fn basic_option_handler(self) -> Self
    where
        Self: ElementWrapper,
    {
        let f = clone!((self.selected => selected, self.show_dropdown => show_dropdown) move |i| {
            selected.set(Some(i));
            show_dropdown.set(false);
        });
        self.option_handler(f)
    }

    pub fn blocked_tooltip(mut self, text: String) -> Self {
        self.blocked_tooltip = Some(text);
        self
    }
}
