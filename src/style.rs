#[allow(dead_code)]
use std::ops::Neg;

use bevy_app::prelude::*;
use bevy_color::prelude::*;
use bevy_ecs::prelude::*;
use bevy_input::prelude::*;
use bevy_text::prelude::*;
use bevy_ui::prelude::*;
use haalka::prelude::*;
use strum::EnumIter;

use crate::signal_or;

pub(crate) static Z_ORDER: &[&str] = &[
    "tooltip",
    "dropdown",
    "target/search",
    "scrollbar",
    "header",
];

pub(crate) fn z_order(name: &str) -> i32 {
    Z_ORDER
        .iter()
        .position(|&s| s == name)
        .map_or(i32::MIN, |i| i32::MAX - (i as i32))
}

pub(crate) fn div(x: f32) -> impl FnMut(f32) -> f32 {
    move |y| y / x
}

#[allow(dead_code)]
pub(crate) fn mul(x: f32) -> impl FnMut(f32) -> f32 {
    move |y| y * x
}

#[allow(dead_code)]
pub(crate) fn add(x: f32) -> impl FnMut(f32) -> f32 {
    move |y| y + x
}

#[allow(dead_code)]
pub(crate) fn sub(x: f32) -> impl FnMut(f32) -> f32 {
    move |y| y - x
}

pub fn font_size_style<E: Element>(
    font_size: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, TextFont>(
                font_size.dedupe(),
                |mut text_font, font_size| text_font.font_size = font_size,
            )
        })
    }
}

pub fn text_style<E: Element>(
    font_size: impl Signal<Item = f32> + Send + 'static,
    color: impl Signal<Item = Color> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el
                .on_signal_with_component::<_, TextFont>(
                    font_size.dedupe(),
                    |mut text_font, font_size| {
                        text_font.font_size = font_size;
                    },
                )
                .component_signal(color.dedupe().map(TextColor))
        })
    }
}

pub fn column_style<E: Element>(
    row_gap: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Node>(
                row_gap.dedupe().map(Val::Px),
                |mut node, row_gap| node.row_gap = row_gap,
            )
        })
    }
}

pub fn row_style<E: Element>(
    column_gap: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el: RawHaalkaEl| {
            raw_el.on_signal_with_component::<_, Node>(
                column_gap.dedupe().map(Val::Px),
                |mut node, column_gap| node.column_gap = column_gap,
            )
        })
    }
}

pub fn padding_style<E: RawElWrapper>(
    edges: impl IntoIterator<Item = BoxEdge>,
    padding: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    let edges = edges.into_iter().collect::<Vec<_>>();
    move |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Node>(
                padding.dedupe().map(Val::Px),
                move |mut node, p| {
                    let ref mut padding = node.padding;
                    for edge in edges.iter() {
                        match edge {
                            BoxEdge::Top => padding.top = p,
                            BoxEdge::Bottom => padding.bottom = p,
                            BoxEdge::Left => padding.left = p,
                            BoxEdge::Right => padding.right = p,
                        }
                    }
                },
            )
        })
    }
}

pub fn left_bordered_style<E: Element>(
    border_width: impl Signal<Item = f32> + Send + 'static,
    border_color: impl Signal<Item = Color> + Send + 'static,
    padding: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Node>(
                border_width.dedupe().map(Val::Px),
                |mut node, width| node.border.left = width,
            )
        })
        .apply(border_color_style(border_color))
        .apply(margin_style([BoxEdge::Bottom], padding.map(div(2.))))
    }
}

pub fn square_style<E: Sizeable>(
    size: impl Signal<Item = f32> + Send + Sync + 'static,
) -> impl FnOnce(E) -> E {
    move |el| {
        let size = size.dedupe().broadcast();
        el.apply(height_style(size.signal()))
            .apply(width_style(size.signal()))
    }
}

pub fn outline_style<E: Element>(
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

pub fn background_style<E: Element>(
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

pub fn height_style<E: Sizeable>(
    height: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| el.height_signal(height.dedupe().map(Val::Px))
}

pub fn width_style<E: Sizeable>(
    width: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| el.width_signal(width.dedupe().map(Val::Px))
}

pub fn border_style<E: Element>(
    border_width: impl Signal<Item = f32> + Send + 'static,
    border_color: impl Signal<Item = Color> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el
                .component_signal::<BorderColor, _>(border_color.dedupe().map(BorderColor))
                .on_signal_with_component::<_, Node>(
                    border_width.dedupe().map(Val::Px).map(UiRect::all),
                    |mut node, width| node.border = width,
                )
        })
    }
}

pub fn border_color_style<E: Element>(
    border_color: impl Signal<Item = impl Into<Option<Color>> + 'static> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        let border_color = border_color.map(Into::into);
        el.update_raw_el(|raw_el| {
            raw_el.component_signal::<BorderColor, _>(border_color.dedupe().map_some(BorderColor))
        })
    }
}

pub fn left_style<E: Element>(
    left: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Node>(
                left.dedupe().map(Val::Px),
                |mut node, left| node.left = left,
            )
        })
    }
}

pub fn top_style<E: Element>(top: impl Signal<Item = f32> + Send + 'static) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el
                .on_signal_with_component::<_, Node>(top.dedupe().map(Val::Px), |mut node, top| {
                    node.top = top
                })
        })
    }
}

#[derive(Clone, Copy, EnumIter, PartialEq, Debug)]
pub enum BoxEdge {
    Top,
    Bottom,
    Left,
    Right,
}

impl BoxEdge {
    pub const ALL: [BoxEdge; 4] = [BoxEdge::Top, BoxEdge::Bottom, BoxEdge::Left, BoxEdge::Right];
    pub const VERTICAL: [BoxEdge; 2] = [BoxEdge::Top, BoxEdge::Bottom];
    pub const HORIZONTAL: [BoxEdge; 2] = [BoxEdge::Left, BoxEdge::Right];
}

#[derive(Clone, Copy, EnumIter, PartialEq, Debug)]
pub enum BoxCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl BoxCorner {
    pub const ALL: [BoxCorner; 4] = [
        BoxCorner::TopLeft,
        BoxCorner::TopRight,
        BoxCorner::BottomLeft,
        BoxCorner::BottomRight,
    ];
    pub const TOP: [BoxCorner; 2] = [BoxCorner::TopLeft, BoxCorner::TopRight];
    pub const BOTTOM: [BoxCorner; 2] = [BoxCorner::BottomLeft, BoxCorner::BottomRight];
    pub const LEFT: [BoxCorner; 2] = [BoxCorner::TopLeft, BoxCorner::BottomLeft];
    pub const RIGHT: [BoxCorner; 2] = [BoxCorner::TopRight, BoxCorner::BottomRight];
}

pub fn border_width_style<E: Element>(
    edges: impl IntoIterator<Item = BoxEdge>,
    border_width: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    let edges = edges.into_iter().collect::<Vec<_>>();
    move |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Node>(
                border_width.dedupe().map(Val::Px),
                move |mut node, border_width| {
                    let ref mut border = node.border;
                    for edge in edges.iter() {
                        match edge {
                            BoxEdge::Top => border.top = border_width,
                            BoxEdge::Bottom => border.bottom = border_width,
                            BoxEdge::Left => border.left = border_width,
                            BoxEdge::Right => border.right = border_width,
                        }
                    }
                },
            )
        })
    }
}

pub fn border_radius_style<E: Element>(
    corners: impl IntoIterator<Item = BoxCorner>,
    border_radius: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    let corners = corners.into_iter().collect::<Vec<_>>();
    move |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, BorderRadius>(
                border_radius.dedupe().map(Val::Px),
                move |mut border_radius, radius| {
                    for corner in corners.iter() {
                        match corner {
                            BoxCorner::TopLeft => border_radius.top_left = radius,
                            BoxCorner::TopRight => border_radius.top_right = radius,
                            BoxCorner::BottomLeft => border_radius.bottom_left = radius,
                            BoxCorner::BottomRight => border_radius.bottom_right = radius,
                        }
                    }
                },
            )
        })
    }
}

pub fn margin_style<E: Element>(
    edges: impl IntoIterator<Item = BoxEdge>,
    margin: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    let edges = edges.into_iter().collect::<Vec<_>>();
    move |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Node>(
                margin.dedupe().map(Val::Px),
                move |mut node, m| {
                    let ref mut margin = node.margin;
                    for edge in edges.iter() {
                        match edge {
                            BoxEdge::Top => margin.top = m,
                            BoxEdge::Bottom => margin.bottom = m,
                            BoxEdge::Left => margin.left = m,
                            BoxEdge::Right => margin.right = m,
                        }
                    }
                },
            )
        })
    }
}

#[derive(Clone, Copy)]
pub enum Move_ {
    Up,
    Down,
    Left,
    Right,
}

pub fn move_style<E: Element>(
    move_: Move_,
    magnitude: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    move |el| {
        el.update_raw_el(move |raw_el| {
            raw_el.on_signal_with_component::<_, Node>(
                magnitude.dedupe().map(Val::Px),
                move |mut node, magnitude| match move_ {
                    Move_::Up => node.top = magnitude.neg(),
                    Move_::Down => node.top = magnitude,
                    Move_::Left => node.left = magnitude.neg(),
                    Move_::Right => node.left = magnitude,
                },
            )
        })
    }
}
