use super::{defaults::*, globals::*, style::*, utils::*};
use crate::impl_syncers;
use bevy::{ecs::system::SystemId, prelude::*};
use haalka::prelude::*;
use std::{
    fmt::Display,
    ops::Neg,
    sync::{Arc, Mutex},
};

pub struct DynamicText {
    el: El<TextBundle>,
    text: Mutable<String>,
    font: Mutable<Handle<Font>>,
    font_size: Mutable<f32>,
    color: Mutable<Color>,
}

impl ElementWrapper for DynamicText {
    type EL = El<TextBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Cursorable for DynamicText {}
impl PointerEventAware for DynamicText {}

impl DynamicText {
    pub fn new() -> Self {
        let text = Mutable::new(String::new());
        let font = Mutable::new(Default::default());
        let font_size = GLOBAL_FONT_SIZE.clone();
        let color = GLOBAL_UNHIGHLIGHTED_COLOR.clone();
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
            .apply(text_style(font_size.signal(), color.signal()));
        Self {
            el,
            text,
            font,
            font_size,
            color,
        }
    }

    pub fn text_signal(self, text_signal: impl Signal<Item = String> + Send + 'static) -> Self {
        let syncer = spawn(sync(text_signal, self.text.clone()));
        self.update_raw_el(|raw_el| raw_el.hold_tasks([syncer]))
    }

    pub fn text(self, text: String) -> Self {
        self.text_signal(always(text))
    }

    // impl_syncers! { text: String, font: Handle<Font>, font_size: f32, color: Color }
}

impl_syncers! { DynamicText { font: Handle<Font>, font_size: f32, color: Color } }

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

impl PointerEventAware for HighlightableText {}

impl HighlightableText {
    pub fn new() -> Self {
        let unhighlighted_color = GLOBAL_UNHIGHLIGHTED_COLOR.clone();
        let highlighted_color = GLOBAL_HIGHLIGHTED_COLOR.clone();
        let hovered = Mutable::new(false);
        let highlighted = Mutable::new(false);
        let dynamic_text = DynamicText::new()
            // .color_signal(
            //     signal::or(hovered.signal(), highlighted.signal()).map_bool_signal(
            //         clone!((highlighted_color) move || highlighted_color.signal()),
            //         clone!((unhighlighted_color) move || unhighlighted_color.signal()),
            //     ),
            // )
            .cursor(CursorIcon::Pointer)
            .hovered_sync(hovered)
            ;
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

    // impl_syncers! {
    //     highlighted_color: Color,
    //     unhighlighted_color: Color,
    //     highlighted: bool
    // }
}

impl_syncers! {
    HighlightableText {
        highlighted_color: Color,
        unhighlighted_color: Color,
        highlighted: bool
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

impl ElementWrapper for Checkbox {
    type EL = El<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }
}

impl Sizeable for Checkbox {}
impl PointerEventAware for Checkbox {}
impl Nameable for Checkbox {}

const CHECKBOX_BORDER_RADIUS_MODIFIER: f32 = 0.333;

impl Checkbox {
    pub fn new() -> Self {
        let size = GLOBAL_FONT_SIZE.clone();
        let background_color = GLOBAL_SECONDARY_BACKGROUND_COLOR.clone();
        let highlighted_color = GLOBAL_HIGHLIGHTED_COLOR.clone();
        let unhighlighted_color = GLOBAL_UNHIGHLIGHTED_COLOR.clone();
        let border_radius_base = GLOBAL_BORDER_RADIUS.get() * CHECKBOX_BORDER_RADIUS_MODIFIER;
        let border_radius = Mutable::new(border_radius_base);
        let hovered = Mutable::new(false);
        let checked = Mutable::new(false);
        let el = El::<NodeBundle>::new()
            .align_content(Align::center())
            .apply(square_style(size.signal()))
            .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
            .apply(border_style(border_radius.signal().map(move |border_radius| border_radius / border_radius_base), hovered.signal().map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), || GLOBAL_PRIMARY_BACKGROUND_COLOR.signal())))
            .apply(background_style(background_color.signal()))
            .hovered_sync(hovered.clone())
            .cursor(CursorIcon::Pointer)
            .child_signal(
                checked.signal()
                .map_true(clone!((size, hovered, highlighted_color, unhighlighted_color, border_radius) move ||
                    El::<NodeBundle>::new()
                        .apply(border_radius_style(BoxCorner::ALL, border_radius.signal().map(|radius| radius * 0.5)))
                        .apply(square_style(size.signal().map(|size| size * 0.6)))
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

    // impl_syncers! {
    //     size: f32,
    //     background_color: Color,
    //     highlighted_color: Color,
    //     unhighlighted_color: Color,
    //     border_radius: f32,
    //     hovered: bool,
    //     checked: bool,
    // }
}

impl_syncers! {
    Checkbox {
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
    // TODO: just doing this with selected index instead, but this can be used
    // in the future to have a filter integrated into the dropdown
    filtered: Mutable<bool>,
}

impl<T> From<T> for OptionData<T> {
    fn from(option: T) -> Self {
        Self {
            option,
            filtered: Mutable::new(false),
        }
    }
}

// TODO: these should be horizontally resizable
pub struct Dropdown<T> {
    el: Stack<NodeBundle>,
    options: MutableVec<OptionData<T>>,
    option_handler_system: Mutable<Option<SystemId<usize>>>,
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
}

impl<T: Clone + PartialEq + Display + Send + Sync + 'static> ElementWrapper for Dropdown<T> {
    type EL = Stack<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(self) -> Self::EL {
        let Self {
            el,
            options,
            option_handler_system,
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
        } = self;
        // TODO: we shouldn't need this, the auto height is being wacky
        let height = map_ref! {
            let font_size = font_size.signal(),
            let padding = padding.signal(),
            let border_width = border_width.signal() => *font_size + *padding * 2. + *border_width * 2. + 2.  // TODO: where did this 2. come from ?
        }
        .map(Val::Px)
        .broadcast();
        let hovered = Mutable::new(false);
        el
        .layer(
            // TODO: should be able to DRY most of the display element for the dropdown option elements
            Stack::<NodeBundle>::new()
                .apply(border_style(border_width.signal(), signal::and(hovered.signal(), signal::not(show_dropdown.signal())).map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), clone!((border_color) move || border_color.signal()))))
                .apply(border_width_style([BoxEdge::Top, BoxEdge::Left, BoxEdge::Right], border_width.signal()))
                .apply(border_width_style([BoxEdge::Bottom], show_dropdown.signal().map_false_signal(clone!((border_width) move || border_width.signal())).map(Option::unwrap_or_default)))
                .apply(border_radius_style([BoxCorner::TopLeft, BoxCorner::TopRight], border_radius.signal()))
                .apply(border_radius_style([BoxCorner::BottomLeft, BoxCorner::BottomRight], show_dropdown.signal().map_false_signal(clone!((border_radius) move || border_radius.signal())).map(Option::unwrap_or_default)))
                .width(Val::Percent(100.))
                .height_signal(height.signal())
                .apply(background_style(background_color.signal()))
                .cursor(CursorIcon::Pointer)
                .on_click(clone!((show_dropdown) move || flip(&show_dropdown)))
                .hovered_sync(hovered.clone())
                .layer(
                    El::<NodeBundle>::new()
                    .width(Val::Percent(100.))
                    .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
                    .apply(border_style(border_width.signal(), signal::and(show_dropdown.signal(), hovered.signal()).map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), clone!((background_color) move || background_color.signal()))))
                    .apply(all_padding_style(padding.signal()))
                    .child(
                        El::<TextBundle>::new()
                            .text(Text::from_section(
                                "",
                                TextStyle {
                                    font_size: font_size.get(),
                                    color: unhighlighted_color.get(),
                                    ..default()
                                },
                            ))
                            .on_signal_with_text(
                                selected.signal()
                                .map_some(clone!((options) move |i|
                                    options.signal_vec_cloned().to_signal_map(move |options| options.get(i).cloned())
                                ))
                                .map(signal::option).flatten().map(Option::flatten),
                                |mut text, option_option| {
                                    if let Some(section) = text.sections.first_mut() {
                                        section.value = option_option.map(|option| option.option.to_string()).unwrap_or_default();
                                    }
                                },
                            )
                            .apply(text_style(font_size.signal(), hovered.signal().map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), clone!((unhighlighted_color) move || unhighlighted_color.signal()))))
                    )
                )
            )
            .layer_signal(show_dropdown.signal().map_true(
                clone!((options, selected) move || {
                    let option_handler_system = option_handler_system.get();
                    Column::<NodeBundle>::new()
                    .border_color_signal(border_color.signal().dedupe().map(Into::into))
                    .width(Val::Percent(100.))
                    .with_style(|mut style| {
                        style.position_type = PositionType::Absolute;
                        // style.top = Val::Percent(100.);  // TODO: this should work
                    })
                    .on_signal_with_style(height.signal(), |mut style, height| style.top = height)
                    .apply(border_width_style([BoxEdge::Left, BoxEdge::Right, BoxEdge::Bottom], border_width.signal()))
                    .apply(border_radius_style([BoxCorner::BottomLeft, BoxCorner::BottomRight], border_radius.signal()))
                    .apply(background_style(background_color.signal()))
                    .align_content(Align::center())
                    .items_signal_vec(
                        options.signal_vec_cloned()
                        .enumerate()
                        .filter_signal_cloned(clone!((selected) move |(i, OptionData { filtered, .. })| signal::and(signal::not(signal_eq(i.signal(), selected.signal())), signal::not(filtered.signal()))))
                        .map(clone!((
                            font_size,
                            unhighlighted_color,
                            background_color,
                            padding,
                            border_width,
                            border_radius,
                            highlighted_color
                        ) move |(i, OptionData { option, .. })| {
                            let hovered = Mutable::new(false);
                            let mut el = Stack::<NodeBundle>::new()
                                .width(Val::Percent(100.))
                                // TODO: these negative margins have no effect ?
                                .apply(margin_style([BoxEdge::Left, BoxEdge::Right], border_width.signal().dedupe().map(Neg::neg)))
                                .apply(border_radius_style(BoxCorner::ALL, border_radius.signal()))
                                .apply(background_style(background_color.signal()))
                                .apply(all_padding_style(padding.signal()))
                                .apply(border_style(border_width.signal(), hovered.signal().map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), clone!((background_color) move || background_color.signal()))))
                                .cursor(CursorIcon::Pointer)
                                .width(Val::Percent(100.))
                                .hovered_sync(hovered.clone())
                                .layer(
                                    El::<TextBundle>::new()
                                    .text(Text::from_section(
                                        option.to_string(),
                                        TextStyle {
                                            font_size: font_size.get(),
                                            color: unhighlighted_color.get(),
                                            ..default()
                                        },
                                    ))
                                    .apply(text_style(font_size.signal(), hovered.signal().map_bool_signal(clone!((highlighted_color) move || highlighted_color.signal()), clone!((unhighlighted_color) move || unhighlighted_color.signal()))))
                                );
                            if let Some(system) = option_handler_system {
                                el = el.on_click_with_system(move |_: In<_>, mut commands: Commands| {
                                    if let Some(i) = i.get() {
                                        commands.run_system_with_input(system, i);
                                    }
                                });
                            }
                            el
                        }))
                    )
                }),
            ))
    }
}

impl<T: Clone + PartialEq + Display + Send + Sync + 'static> Sizeable for Dropdown<T> {}

impl<T> Dropdown<T> {
    pub fn new(options: MutableVec<OptionData<T>>) -> Self
    where
        T: Clone + PartialEq + Display + Send + Sync + 'static,
    {
        Self {
            el: Stack::<NodeBundle>::new(),
            options,
            option_handler_system: Mutable::new(None),
            selected: Mutable::new(None),
            show_dropdown: Mutable::new(false),
            font_size: GLOBAL_FONT_SIZE.clone(),
            padding: Mutable::new(GLOBAL_PADDING.get() * 0.5),
            border_radius: Mutable::new(GLOBAL_BORDER_RADIUS.get() * 0.5),
            border_width: GLOBAL_BORDER_WIDTH.clone(),
            background_color: GLOBAL_PRIMARY_BACKGROUND_COLOR.clone(),
            highlighted_color: GLOBAL_HIGHLIGHTED_COLOR.clone(),
            unhighlighted_color: GLOBAL_UNHIGHLIGHTED_COLOR.clone(),
            border_color: GLOBAL_BORDER_COLOR.clone(),
        }
    }

    // impl_syncers! { selected: Option<usize>, show_dropdown: bool, font_size: f32, padding: f32, border_radius: f32, border_width: f32, background_color: Color, highlighted_color: Color, unhighlighted_color: Color, border_color: Color }

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
        handler: impl IntoSystem<usize, (), Marker> + Send + 'static,
    ) -> Self
    where
        Self: ElementWrapper,
    {
        let system_holder = self.option_handler_system.clone();
        self.update_raw_el(|raw_el| {
            raw_el.with_entity(clone!((system_holder) move |mut entity| {
                let handler = entity.world_scope(|world| register_system(world, handler));
                system_holder.set(Some(handler));
            }))
            .on_remove(move |world, _| {
                if let Some(handler) = system_holder.get() {
                    world.commands().add(move |world: &mut World| {
                        let _ = world.remove_system(handler);
                    })
                }
            })
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
}


// pub fn resize_border<E: Element>(
//     width: impl Signal<Item = f32> + Send + Sync + 'static,
//     radius: impl Signal<Item = f32> + Send + Sync + 'static,
//     unhighlighted_color: impl Signal<Item = Color> + Send + Sync + 'static,
//     highlighted_color: impl Signal<Item = Color> + Send + Sync + 'static,
// ) -> impl Element {
//     Stack::<NodeBundle>::new()
// }
