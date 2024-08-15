use bevy::prelude::*;
use haalka::prelude::*;

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

#[derive(Clone, Copy)]
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
