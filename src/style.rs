use bevy::{prelude::*, ui::widget};
use haalka::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

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

#[derive(Clone, Copy)]
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

const RESIZE_BORDER_SLACK_PERCENT: f32 = 90.;

pub fn resize_border<E: Element + Sizeable>(
    height: impl Signal<Item = f32> + Send + Sync + 'static,
    width: impl Signal<Item = f32> + Send + Sync + 'static,
    border_width: impl Signal<Item = f32> + Send + Sync + 'static,
    radius: impl Signal<Item = f32> + Send + Sync + 'static,
    unhighlighted_color: impl Signal<Item = Color> + Send + 'static,
    highlighted_color: impl Signal<Item = Color> + Send + 'static,
) -> impl FnOnce(E) -> Stack<NodeBundle> {
    move |mut el| {
        let hovereds = BoxEdge::iter()
            .map(|_| Mutable::new(false))
            .collect::<Vec<_>>();
        let height = height.dedupe().broadcast();
        let width = width.dedupe().broadcast();
        let border_width = border_width.dedupe().broadcast();
        let radius = radius.dedupe().broadcast();
        let mut el = Stack::<NodeBundle>::new()
            .height_signal(height.signal().map(Val::Px))
            .width_signal(width.signal().map(Val::Px))
            .layer({
                let mut el = El::<NodeBundle>::new()
                    .align(Align::center())
                    .height(Val::Percent(100.))
                    .width(Val::Percent(100.))
                    .apply(border_radius_style(BoxCorner::ALL, radius.signal()))
                    .apply(border_color_style(highlighted_color));
                for (edge, hovered) in BoxEdge::iter().zip(hovereds.iter()) {
                    el = el.apply(border_width_style(
                        [edge],
                        hovered
                            .signal()
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
                    .apply(border_color_style(unhighlighted_color));
                for (edge, hovered) in BoxEdge::iter().zip(hovereds.iter()) {
                    el = el.apply(border_width_style(
                        [edge],
                        hovered
                            .signal()
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
            .map(Val::Px)
            .broadcast();
        let hovereds = MutableVec::from(hovereds);
        let hovered_iter = hovereds.lock_ref().into_iter().cloned().collect::<Vec<_>>();
        for (edge, hovered) in BoxEdge::iter().zip(hovered_iter) {
            el = el.layer({
                let mut el = El::<NodeBundle>::new()
                    // .update_raw_el(|raw_el| {
                    //     raw_el.defer_update(DeferredUpdateAppendDirection::Back, |raw_el| {
                    //         raw_el.insert(Pickable {
                    //             should_block_lower: false,
                    //             ..default()
                    //         })
                    //     })
                    // })
                    .on_signal_with_style(
                        border_width_slack.signal().map(Val::Px),
                        move |mut style, slack| match edge {
                            BoxEdge::Top => {
                                style.top = -slack;
                                style.right = slack;
                            }
                            BoxEdge::Bottom => {
                                style.bottom = -slack;
                                style.right = slack;
                            }
                            BoxEdge::Left => {
                                style.left = -slack;
                                style.bottom = slack;
                            }
                            BoxEdge::Right => {
                                style.right = -slack;
                                style.bottom = slack;
                            }
                        },
                    )
                    .hovered_sync(hovered.clone())
                    .cursor_signal(
                        hovereds
                            .signal_vec_cloned()
                            .enumerate()
                            .map_signal(|(i, hovered)| {
                                map_ref! {
                                    let i_option = i.signal(),
                                    let hovered = hovered.signal() => 'block: {
                                        if let Some(i) = i_option {
                                            if *hovered {
                                                break 'block match i {
                                                    0 => Some(BoxEdge::Top),
                                                    1 => Some(BoxEdge::Bottom),
                                                    2 => Some(BoxEdge::Left),
                                                    3 => Some(BoxEdge::Right),
                                                    _ => None,
                                                }
                                            }
                                        }
                                        None
                                    }
                                }
                            })
                            .to_signal_map(move |edges| {
                                println!("{:?}", edges);
                                if edges.contains(&Some(edge)) {
                                    match edge {
                                        BoxEdge::Top => {
                                            if edges.contains(&Some(BoxEdge::Left)) {
                                                return Some(CursorIcon::NwResize);
                                            } else if edges.contains(&Some(BoxEdge::Right)) {
                                                return Some(CursorIcon::NeResize);
                                            } else {
                                                return Some(CursorIcon::NsResize);
                                            }
                                        }
                                        BoxEdge::Bottom => {
                                            if edges.contains(&Some(BoxEdge::Left)) {
                                                return Some(CursorIcon::SwResize);
                                            } else if edges.contains(&Some(BoxEdge::Right)) {
                                                return Some(CursorIcon::SeResize);
                                            } else {
                                                return Some(CursorIcon::NsResize);
                                            }
                                        }
                                        BoxEdge::Left => {
                                            if edges.contains(&Some(BoxEdge::Top)) {
                                                return Some(CursorIcon::NwResize);
                                            } else if edges.contains(&Some(BoxEdge::Bottom)) {
                                                return Some(CursorIcon::SwResize);
                                            } else {
                                                return Some(CursorIcon::EwResize);
                                            }
                                        }
                                        BoxEdge::Right => {
                                            if edges.contains(&Some(BoxEdge::Top)) {
                                                return Some(CursorIcon::NeResize);
                                            } else if edges.contains(&Some(BoxEdge::Bottom)) {
                                                return Some(CursorIcon::SeResize);
                                            } else {
                                                return Some(CursorIcon::EwResize);
                                            }
                                        }
                                    }
                                }
                                None
                            }),
                    )
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
                            .height_signal(
                                map_ref! {
                                    let slack = border_width_slack.signal(),
                                    let height = height.signal() => {
                                        height + slack * 2.
                                    }
                                }
                                .map(Val::Px),
                            )
                            .width_signal(resize_border_width.signal());
                    }
                    BoxEdge::Top | BoxEdge::Bottom => {
                        el = el.height_signal(resize_border_width.signal()).width_signal(
                            map_ref! {
                                let slack = border_width_slack.signal(),
                                let width = width.signal() => {
                                    width + slack * 2.
                                }
                            }
                            .map(Val::Px),
                        );
                    }
                }
                el
            });
        }
        el
    }
}
