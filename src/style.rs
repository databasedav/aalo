use std::{ops::Neg, sync::Arc};

use bevy::{color::palettes::css::MAROON, prelude::*, ui::widget};
use haalka::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::globals::GLOBAL_PRIMARY_BACKGROUND_COLOR;

pub fn nested_fields_style<E: Element>(
    row_gap: impl Signal<Item = f32> + Send + Sync + 'static,
    padding: impl Signal<Item = f32> + Send + 'static,
    border_width: impl Signal<Item = f32> + Send + 'static,
    border_color: impl Signal<Item = Color> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        let row_gap = row_gap.dedupe().broadcast();
        el.apply(column_style(row_gap.signal()))
            .apply(horizontal_padding_style(padding))
            .apply(left_bordered_style(border_width, border_color))
    }
}

pub fn text_style<E: Element>(
    font_size: impl Signal<Item = f32> + Send + 'static,
    color: impl Signal<Item = Color> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el
                .on_signal_with_component::<_, Text>(font_size.dedupe(), |mut text, font_size| {
                    if let Some(section) = text.sections.first_mut() {
                        section.style.font_size = font_size;
                    }
                })
                .on_signal_with_component::<_, Text>(color.dedupe(), |mut text, color| {
                    if let Some(section) = text.sections.first_mut() {
                        section.style.color = color;
                    }
                })
        })
    }
}

pub fn column_style<E: Element>(
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

pub fn row_style<E: Element>(
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

pub fn padding_style<E: Element>(
    edges: impl IntoIterator<Item = BoxEdge>,
    padding: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    let edges = edges.into_iter().collect::<Vec<_>>();
    move |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Style>(
                padding.dedupe().map(Val::Px),
                move |mut style, p| {
                    let ref mut padding = style.padding;
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

pub fn all_padding_style<E: Element>(
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

pub fn vertical_padding_style<E: Element>(
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

pub fn horizontal_padding_style<E: Element>(
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

pub fn left_bordered_style<E: Element>(
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

pub fn square_style<E: Sizeable>(
    size: impl Signal<Item = f32> + Send + Sync + 'static,
) -> impl FnOnce(E) -> E {
    move |el| {
        let size = size.dedupe().map(Val::Px).broadcast();
        el.height_signal(size.signal()).width_signal(size.signal())
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
                .on_signal_with_component::<_, Style>(
                    border_width.dedupe().map(Val::Px).map(UiRect::all),
                    |mut style, width| style.border = width,
                )
        })
    }
}

pub fn border_color_style<E: Element>(
    border_color: impl Signal<Item = Color> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el.component_signal::<BorderColor, _>(border_color.dedupe().map(BorderColor))
        })
    }
}

pub fn left_style<E: Element>(
    left: impl Signal<Item = f32> + Send + 'static,
) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Style>(
                left.dedupe().map(Val::Px),
                |mut style, left| style.left = left,
            )
        })
    }
}

pub fn top_style<E: Element>(top: impl Signal<Item = f32> + Send + 'static) -> impl FnOnce(E) -> E {
    |el| {
        el.update_raw_el(|raw_el| {
            raw_el.on_signal_with_component::<_, Style>(
                top.dedupe().map(Val::Px),
                |mut style, top| style.top = top,
            )
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
            raw_el.on_signal_with_component::<_, Style>(
                border_width.dedupe().map(Val::Px),
                move |mut style, border_width| {
                    let ref mut border = style.border;
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
            raw_el.on_signal_with_component::<_, Style>(
                margin.dedupe().map(Val::Px),
                move |mut style, m| {
                    let ref mut margin = style.margin;
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

const RESIZE_BORDER_SLACK_PERCENT: f32 = 100.;

#[derive(Component)]
struct ResizeParent;

macro_rules! signal_or {
    ($signal:expr) => {
        $signal
    };
    ($first:expr, $($rest:expr),+) => {
        signal::or($first, signal_or!($($rest),+))
    };
}

pub fn resize_border<E: Element + Sizeable>(
    border_width: impl Signal<Item = f32> + Send + Sync + 'static,
    radius: impl Signal<Item = f32> + Send + Sync + 'static,
    unhighlighted_color: impl Signal<Item = Color> + Send + 'static,
    highlighted_color: impl Signal<Item = Color> + Send + 'static,
    wrapper_stack_option: Option<Stack<NodeBundle>>,
) -> impl FnOnce(E) -> Stack<NodeBundle> {
    move |mut el| {
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
        let radius = radius.dedupe().broadcast();
        let edge_highlighted = |edge| match edge {
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
        let mut el = wrapper_stack_option
            .unwrap_or_else(|| Stack::<NodeBundle>::new())
            .update_raw_el(|raw_el| raw_el.insert(ResizeParent))
            .apply(border_radius_style(BoxCorner::ALL, radius.signal()))
            .layer({
                let mut el = El::<NodeBundle>::new()
                    .align(Align::center())
                    .height(Val::Percent(100.))
                    .width(Val::Percent(100.))
                    .apply(border_radius_style(BoxCorner::ALL, radius.signal()))
                    .apply(border_color_style(highlighted_color));
                for edge in BoxEdge::iter() {
                    el = el.apply(border_width_style(
                        [edge],
                        edge_highlighted(edge)
                            .map_true_signal(clone!((border_width) move || border_width.signal()))
                            .map(Option::unwrap_or_default),
                    ));
                }
                el
            })
            .layer({
                el = el
                    .align(Align::center())
                    .height(Val::Percent(100.))
                    .width(Val::Percent(100.))
                    .apply(border_radius_style(
                        BoxCorner::ALL,
                        radius.signal().map(|radius| radius * 0.8),
                    ))
                    .apply(border_color_style(unhighlighted_color))
                    .apply(padding_style(
                        [BoxEdge::Top],
                        edge_highlighted(BoxEdge::Top)
                            .map_true_signal(clone!((border_width) move || border_width.signal()))
                            .map(Option::unwrap_or_default),
                    ))
                    .apply(padding_style(
                        [BoxEdge::Left],
                        edge_highlighted(BoxEdge::Left)
                            .map_true_signal(clone!((border_width) move || border_width.signal()))
                            .map(Option::unwrap_or_default),
                    ));
                for edge in BoxEdge::iter() {
                    el = el.apply(border_width_style(
                        [edge],
                        edge_highlighted(edge)
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
        let hovered_iter = hovereds.lock_ref().into_iter().cloned().collect::<Vec<_>>();
        for (edge, hovered) in BoxEdge::iter().zip(hovered_iter) {
            el = el.layer({
                let mut el = El::<NodeBundle>::new()
                    .update_raw_el(clone!(
                        (edge_downs) | raw_el | {
                            raw_el
                        .on_event_with_system_stop_propagation::<Pointer<Down>, _>(clone!((edge_downs) move |_: In<_>, mut on_pointer_up_handlers: ResMut<OnPointerUpHandlers>| {
                            match edge {
                                BoxEdge::Top => {
                                    edge_downs[0].set_neq(true);
                                    on_pointer_up_handlers.0.push(Box::new(clone!((edge_downs) move || {
                                        edge_downs[0].set_neq(false);
                                    })));
                                },
                                BoxEdge::Bottom => {
                                    edge_downs[1].set_neq(true);
                                    on_pointer_up_handlers.0.push(Box::new(clone!((edge_downs) move || {
                                        edge_downs[1].set_neq(false);
                                    })));
                                },
                                BoxEdge::Left => {
                                    edge_downs[2].set_neq(true);
                                    on_pointer_up_handlers.0.push(Box::new(clone!((edge_downs) move || {
                                        edge_downs[2].set_neq(false);
                                    })));
                                },
                                BoxEdge::Right => {
                                    edge_downs[3].set_neq(true);
                                    on_pointer_up_handlers.0.push(Box::new(clone!((edge_downs) move || {
                                        edge_downs[3].set_neq(false);
                                    })));
                                },
                            }
                        }))
                        .on_event_with_system_stop_propagation::<Pointer<Drag>, _>(
                            move |In((entity, drag)): In<(Entity, Pointer<Drag>)>,
                                parents: Query<&Parent>,
                                mut resize_parent: Local<Option<Entity>>,
                                resize_parents: Query<&ResizeParent>,
                                mut styles: Query<&mut Style>| {
                            if resize_parent.is_none() {
                                for parent in parents.iter_ancestors(entity) {
                                    if resize_parents.contains(parent) {
                                        *resize_parent = Some(parent);
                                    }
                                }
                            }
                            if let Some(resize_parent) = *resize_parent {
                                if let Ok(mut style) = styles.get_mut(resize_parent) {
                                    match edge {
                                        BoxEdge::Top => {
                                            if let Val::Px(cur) = style.height {
                                                style.height = Val::Px(cur - drag.delta.y);
                                            }
                                            match style.top {
                                                Val::Auto => style.top = Val::Px(0.),
                                                Val::Px(cur) => style.top = Val::Px(cur + drag.delta.y),
                                                _ => (),
                                            }
                                        }
                                        BoxEdge::Bottom => {
                                            if let Val::Px(cur) = style.height {
                                                style.height = Val::Px(cur + drag.delta.y);
                                            }
                                        }
                                        BoxEdge::Left => {
                                            if let Val::Px(cur) = style.width {
                                                style.width = Val::Px(cur - drag.delta.x);
                                            }
                                            match style.left {
                                                Val::Auto => style.left = Val::Px(0.),
                                                Val::Px(cur) => style.left = Val::Px(cur + drag.delta.x),
                                                _ => (),
                                            }
                                        }
                                        BoxEdge::Right => {
                                            if let Val::Px(cur) = style.width {
                                                style.width = Val::Px(cur + drag.delta.x);
                                            }
                                    }
                                }
                            }
                        }})
                        }
                    ))
                    .on_signal_with_style(
                        border_width_slack.signal().map(Val::Px),
                        move |mut style, slack| match edge {
                            BoxEdge::Top => style.top = -slack,
                            BoxEdge::Bottom => style.bottom = -slack,
                            BoxEdge::Left => style.left = -slack,
                            BoxEdge::Right => style.right = -slack,
                        },
                    )
                    .hovered_sync(hovered.clone())
                    .cursor(match edge {
                        BoxEdge::Top | BoxEdge::Bottom => CursorIcon::NsResize,
                        BoxEdge::Left | BoxEdge::Right => CursorIcon::EwResize,
                    })
                    .align(match edge {
                        BoxEdge::Top => Align::new().top(),
                        BoxEdge::Bottom => Align::new().bottom(),
                        BoxEdge::Left => Align::new().left(),
                        BoxEdge::Right => Align::new().right(),
                    })
                    .background_color(BackgroundColor(Color::NONE));
                // .background_color(BackgroundColor(Color::BLACK.with_alpha(0.3)));
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
            el = el.layer({
                let mut el = El::<NodeBundle>::new()
                    .update_raw_el(clone!((edge_downs) move |raw_el| {
                        raw_el
                        .on_event_with_system_stop_propagation::<Pointer<Down>, _>(clone!((edge_downs) move |_: In<_>, mut on_pointer_up_handlers: ResMut<OnPointerUpHandlers>| {
                            match corner {
                                BoxCorner::TopLeft => {
                                    edge_downs[0].set_neq(true);
                                    edge_downs[2].set_neq(true);
                                    on_pointer_up_handlers.0.push(Box::new(clone!((edge_downs) move || {
                                        edge_downs[0].set_neq(false);
                                        edge_downs[2].set_neq(false);
                                    })));
                                },
                                BoxCorner::TopRight => {
                                    edge_downs[0].set_neq(true);
                                    edge_downs[3].set_neq(true);
                                    on_pointer_up_handlers.0.push(Box::new(clone!((edge_downs) move || {
                                        edge_downs[0].set_neq(false);
                                        edge_downs[3].set_neq(false);
                                    })));
                                },
                                BoxCorner::BottomLeft => {
                                    edge_downs[1].set_neq(true);
                                    edge_downs[2].set_neq(true);
                                    on_pointer_up_handlers.0.push(Box::new(clone!((edge_downs) move || {
                                        edge_downs[1].set_neq(false);
                                        edge_downs[2].set_neq(false);
                                    })));
                                },
                                BoxCorner::BottomRight => {
                                    edge_downs[1].set_neq(true);
                                    edge_downs[3].set_neq(true);
                                    on_pointer_up_handlers.0.push(Box::new(clone!((edge_downs) move || {
                                        edge_downs[1].set_neq(false);
                                        edge_downs[3].set_neq(false);
                                    })));
                                },
                            }
                        }))
                        .on_event_with_system_stop_propagation::<Pointer<Drag>, _>(move |In((entity, drag)): In<(Entity, Pointer<Drag>)>, parents: Query<&Parent>, mut resize_parent: Local<Option<Entity>>, resize_parents: Query<&ResizeParent>, mut styles: Query<&mut Style>| {
                            if resize_parent.is_none() {
                                for parent in parents.iter_ancestors(entity) {
                                    if resize_parents.contains(parent) {
                                        *resize_parent = Some(parent);
                                    }
                                }
                            }
                            if let Some(resize_parent) = *resize_parent {
                                if let Ok(mut style) = styles.get_mut(resize_parent) {
                                    match corner {
                                        BoxCorner::TopLeft => {
                                            if let Val::Px(cur) = style.height {
                                                style.height = Val::Px(cur - drag.delta.y);
                                            }
                                            match style.top {
                                                Val::Auto => style.top = Val::Px(0.),
                                                Val::Px(cur) => style.top = Val::Px(cur + drag.delta.y),
                                                _ => (),
                                            }
                                            if let Val::Px(cur) = style.width {
                                                style.width = Val::Px(cur - drag.delta.x);
                                            }
                                            match style.left {
                                                Val::Auto => style.left = Val::Px(0.),
                                                Val::Px(cur) => style.left = Val::Px(cur + drag.delta.x),
                                                _ => (),
                                            }
                                        }
                                        BoxCorner::TopRight => {
                                            if let Val::Px(cur) = style.height {
                                                style.height = Val::Px(cur - drag.delta.y);
                                            }
                                            match style.top {
                                                Val::Auto => style.top = Val::Px(0.),
                                                Val::Px(cur) => style.top = Val::Px(cur + drag.delta.y),
                                                _ => (),
                                            }
                                            if let Val::Px(cur) = style.width {
                                                style.width = Val::Px(cur + drag.delta.x);
                                            }
                                        }
                                        BoxCorner::BottomLeft => {
                                            if let Val::Px(cur) = style.height {
                                                style.height = Val::Px(cur + drag.delta.y);
                                            }
                                            if let Val::Px(cur) = style.width {
                                                style.width = Val::Px(cur - drag.delta.x);
                                            }
                                            match style.left {
                                                Val::Auto => style.left = Val::Px(0.),
                                                Val::Px(cur) => style.left = Val::Px(cur + drag.delta.x),
                                                _ => (),
                                            }
                                        }
                                        BoxCorner::BottomRight => {
                                            if let Val::Px(cur) = style.height {
                                                style.height = Val::Px(cur + drag.delta.y);
                                            }
                                            if let Val::Px(cur) = style.width {
                                                style.width = Val::Px(cur + drag.delta.x);
                                            }
                                        }
                                    }
                                }
                            }
                        })
                    }))
                    .apply(square_style(resize_border_width.signal()))
                    .on_signal_with_style(border_width_slack.signal(), move |mut style, slack| {
                        match corner {
                            BoxCorner::TopLeft => {
                                style.top = -Val::Px(slack * 0.5);
                                style.left = -Val::Px(slack * 0.5);
                            }
                            BoxCorner::TopRight => {
                                style.top = -Val::Px(slack * 0.5);
                                style.right = -Val::Px(slack * 0.5);
                            }
                            BoxCorner::BottomLeft => {
                                style.bottom = -Val::Px(slack * 0.5);
                                style.left = -Val::Px(slack * 0.5);
                            }
                            BoxCorner::BottomRight => {
                                style.bottom = -Val::Px(slack * 0.5);
                                style.right = -Val::Px(slack * 0.5);
                            }
                        }
                    })
                    .hovered_sync(hovered.clone())
                    .cursor(match corner {
                        BoxCorner::TopLeft | BoxCorner::BottomRight => CursorIcon::NwseResize,
                        BoxCorner::TopRight | BoxCorner::BottomLeft => CursorIcon::NeswResize,
                    })
                    .align(match corner {
                        BoxCorner::TopLeft => Align::new().top().left(),
                        BoxCorner::TopRight => Align::new().top().right(),
                        BoxCorner::BottomLeft => Align::new().bottom().left(),
                        BoxCorner::BottomRight => Align::new().bottom().right(),
                    })
                    .background_color(BackgroundColor(Color::NONE))
                    // .background_color(BackgroundColor(Color::BLACK.with_alpha(0.3)))
                    ;
                el
            });
        }
        el
    }
}

#[derive(Event)]
struct OnPointerUpFlush;

#[derive(Resource, Default)]
pub struct OnPointerUpHandlers(pub Vec<Box<dyn FnMut() + Send + Sync + 'static>>);

fn on_pointer_up_handlers_pending(handlers: Res<OnPointerUpHandlers>) -> bool {
    !handlers.0.is_empty()
}

fn listen_for_release(mouse_inputs: Res<ButtonInput<MouseButton>>, mut commands: Commands) {
    if mouse_inputs.just_released(MouseButton::Left) {
        commands.trigger(OnPointerUpFlush);
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<OnPointerUpHandlers>()
        .add_systems(
            Update,
            listen_for_release.run_if(on_pointer_up_handlers_pending),
        )
        .observe(
            |_: Trigger<OnPointerUpFlush>, mut handlers: ResMut<OnPointerUpHandlers>| {
                for mut handler in handlers.0.drain(..) {
                    handler();
                }
            },
        );
}
