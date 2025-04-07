#![allow(dead_code)]

use bevy::prelude::*;

pub(crate) fn example_window_plugin() -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            #[cfg(feature = "deployed_wasm_example")]
            canvas: Some("#bevy".to_string()),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: true,
            ..default()
        }),
        ..default()
    }
}
