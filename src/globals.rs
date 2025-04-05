use super::defaults::*;
use bevy_color::prelude::*;
use haalka::prelude::*;

pub static GLOBAL_FONT_SIZE: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(DEFAULT_FONT_SIZE));
pub static GLOBAL_ROW_GAP: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(DEFAULT_ROW_GAP));
pub static GLOBAL_COLUMN_GAP: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(DEFAULT_COLUMN_GAP));
pub static GLOBAL_PADDING: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(DEFAULT_PADDING));
pub static GLOBAL_BORDER_RADIUS: Lazy<Mutable<f32>> =
    Lazy::new(|| Mutable::new(DEFAULT_BORDER_RADIUS));
pub static GLOBAL_BORDER_WIDTH: Lazy<Mutable<f32>> =
    Lazy::new(|| Mutable::new(DEFAULT_BORDER_WIDTH));
pub static GLOBAL_PRIMARY_BACKGROUND_COLOR: Lazy<Mutable<Color>> =
    Lazy::new(|| Mutable::new(DEFAULT_PRIMARY_BACKGROUND_COLOR));
pub static GLOBAL_SECONDARY_BACKGROUND_COLOR: Lazy<Mutable<Color>> =
    Lazy::new(|| Mutable::new(DEFAULT_SECONDARY_BACKGROUND_COLOR));
pub static GLOBAL_TERTIARY_BACKGROUND_COLOR: Lazy<Mutable<Color>> =
    Lazy::new(|| Mutable::new(DEFAULT_TERTIARY_BACKGROUND_COLOR));
pub static GLOBAL_HIGHLIGHTED_COLOR: Lazy<Mutable<Color>> =
    Lazy::new(|| Mutable::new(DEFAULT_HIGHLIGHTED_COLOR));
pub static GLOBAL_UNHIGHLIGHTED_COLOR: Lazy<Mutable<Color>> =
    Lazy::new(|| Mutable::new(DEFAULT_UNHIGHLIGHTED_COLOR));
pub static GLOBAL_BORDER_COLOR: Lazy<Mutable<Color>> =
    Lazy::new(|| Mutable::new(DEFAULT_BORDER_COLOR));
pub static GLOBAL_ERROR_COLOR: Lazy<Mutable<Color>> =
    Lazy::new(|| Mutable::new(DEFAULT_ERROR_COLOR));
pub static GLOBAL_SCROLL_PIXELS: Lazy<Mutable<f32>> =
    Lazy::new(|| Mutable::new(DEFAULT_SCROLL_PIXELS));
