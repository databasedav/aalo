use super::{defaults::*, globals::*, style::*};
use crate::impl_syncers;
use bevy::prelude::*;
use haalka::prelude::*;
use std::fmt::Display;

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
    pub fn new() -> Self {
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
            .apply(text_style(font_size.signal(), color.signal()));
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
            .cursor(CursorIcon::Pointer)
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
            .apply(background_style(background_color.signal()))
            .hovered_sync(hovered.clone())
            .cursor(CursorIcon::Pointer)
            .child_signal(
                checked.signal()
                .map_true(clone!((size, hovered, highlighted_color, unhighlighted_color, border_radius) move ||
                    El::<NodeBundle>::new()
                        .apply(border_radius_style(border_radius.signal().map(|radius| radius * 0.5)))
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
}

#[derive(Clone)]
pub struct OptionData<T> {
    pub option: T,
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

pub struct Dropdown<T> {
    el: El<NodeBundle>,
    options: MutableVec<OptionData<T>>,
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
    type EL = El<NodeBundle>;
    fn element_mut(&mut self) -> &mut Self::EL {
        &mut self.el
    }

    fn into_el(mut self) -> Self::EL {
        let Self {
            el,
            options,
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
        el.child(
            El::<NodeBundle>::new()
                .child(
                    El::<TextBundle>::new()
                        .text(Text::from_section(
                            "test",
                            TextStyle {
                                font_size: font_size.get(),
                                color: unhighlighted_color.get(),
                                ..default()
                            },
                        ))
                        .apply(text_style(font_size.signal(), unhighlighted_color.signal())),
                )
                .apply(background_style(background_color.signal()))
                .apply(all_padding_style(padding.signal()))
                .apply(border_style(border_width.signal(), border_color.signal()))
                .border_radius(BorderRadius::all(Val::Px(border_radius.get())))
                .on_signal_with_border_radius(show_dropdown.signal().map_false_signal(clone!((border_radius) move || border_radius.signal())), |mut border_radius, radius_option| {
                    border_radius.bottom_left = radius_option.map(Val::Px).unwrap_or_default();
                    border_radius.bottom_right = radius_option.map(Val::Px).unwrap_or_default();
                })
                .cursor(CursorIcon::Pointer)
                .on_click(clone!((show_dropdown) move || flip(&show_dropdown))),
        )
        .child_signal(show_dropdown.signal().map_true(
            clone!((show_dropdown, options) move || {
                Column::<NodeBundle>::new()
                .width(Val::Percent(100.))
                .height(Val::Percent(100.))
                .with_style(|mut style| {
                    style.position_type = PositionType::Absolute;
                    style.top = Val::Percent(100.);
                })
                .apply(border_style(border_width.signal(), border_color.signal()))
                .apply(background_style(background_color.signal()))
                .cursor(CursorIcon::Pointer)
                .items_signal_vec(
                    options.signal_vec_cloned()
                    .filter_signal_cloned(|OptionData { filtered, .. }| filtered.signal())
                    .map(clone!((font_size, unhighlighted_color, background_color, padding, border_width, border_color, border_radius) move |OptionData { option, .. }| {
                        El::<NodeBundle>::new()
                        .apply(border_radius_style(border_radius.signal()))
                        .apply(background_style(background_color.signal()))
                        .apply(all_padding_style(padding.signal()))
                        .apply(border_style(border_width.signal(), border_color.signal()))
                        .width(Val::Percent(100.))
                        .height(Val::Px(30.))
                        .child(
                            El::<TextBundle>::new()
                            .text(Text::from_section(
                                // option.to_string(),
                                "test",
                                TextStyle {
                                    font_size: font_size.get(),
                                    color: unhighlighted_color.get(),
                                    ..default()
                                },
                            ))
                            .apply(text_style(font_size.signal(), unhighlighted_color.signal()))
                        )
                    }))
                )
            }),
        ))
    }
}

impl<T> Dropdown<T> {
    pub fn new(options: MutableVec<OptionData<T>>) -> Self
    where
        T: Clone + PartialEq + Display + Send + Sync + 'static,
    {
        Self {
            el: El::<NodeBundle>::new(),
            options,
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
}
