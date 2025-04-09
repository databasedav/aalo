# aalo [আলো](https://translate.google.com/?sl=bn&tl=en&text=%E0%A6%86%E0%A6%B2%E0%A7%8B&op=translate)

[![Crates.io Version](https://img.shields.io/crates/v/aalo?style=for-the-badge)](https://crates.io/crates/aalo)
[![Docs.rs](https://img.shields.io/docsrs/aalo?style=for-the-badge)](https://docs.rs/aalo)
[![Following released Bevy versions](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue?style=for-the-badge)](https://bevyengine.org/learn/quick-start/plugin-development/#main-branch-tracking)

```text
in bengali, aalo means "light" (i.e. photons), not to be confused with haalka !
```

[aalo](https://github.com/databasedav/aalo) is a [haalka](https://github.com/databasedav/haalka) port (in progress) of [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui).

## setup

```toml
[dependencies]
aalo = { version = "0.0", optional = true }

[features]
development = ["aalo"]
```

```rust
#[cfg(feature = "development")]
use aalo::prelude::*;

#[cfg(feature = "development")]
app.add_plugins(AaloPlugin::new().world());
```

***HIGHLY RECOMMENDED***, while not required, aalo is much snappier when compiled in release mode, you'll only need to do so once

```toml
[profile.dev.package.aalo]
opt-level = 3 
```

## registering custom frontends

use `register_frontend`, passing in a fully qualified type path and a function that returns an `impl Bundle`, e.g. `Node`, whose `Entity` also has a `FieldListener` `Component`; `FieldListener` is just a wrapper around a `SystemId<In<Box<dyn PartialReflect>>>`, which will be forwarded the corresponding field's value every frame it is visible in the inspector

```rust no_run
fn init_custom_bool_frontend(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
    let mut commands = world.commands();
    let text = commands.spawn_empty().id();
    let system = commands.register_system(
        move |In(reflect): In<Box<dyn PartialReflect>>, mut commands: Commands| {
            let cur_option = reflect.try_downcast_ref::<bool>().copied().or_else(|| {
                CustomBoolComponent::from_reflect(reflect.as_ref())
                    .map(|CustomBoolComponent(cur)| cur)
            });
            if let Some(cur) = cur_option {
                commands.entity(text).insert(Text(cur.to_string()));
            }
        },
    );
    commands
        .entity(entity)
        .add_child(text)
        .insert(FieldListener::new(system))
        .observe(
            move |click: Trigger<Pointer<Click>>, texts: Query<&Text>, mut field: TargetField| {
                if let Ok(Text(text)) = texts.get(text) {
                    let cur = match text.as_str() {
                        "true" => true,
                        "false" => false,
                        _ => return,
                    };
                    // one of these will silently error depending on if it's the field or component
                    // target, we just do both here for the convenience of using the same frontend
                    field.update(click.entity(), (!cur).clone_value());
                    field.update(click.entity(), CustomBoolComponent(!cur).clone_value());
                }
            },
        );
}

#[derive(Component)]
#[require(Node)]
#[component(on_add = init_custom_bool_frontend)]
struct CustomBoolFrontend;

fn custom_bool_frontend() -> impl Bundle {
    CustomBoolFrontend
}

#[derive(Component, Reflect, Default)]
struct CustomBoolComponent(bool);

register_frontend("bool", custom_bool_frontend);
register_frontend("custom::CustomBoolComponent", custom_bool_frontend);
```

see [custom frontend example](https://github.com/databasedav/aalo/blob/main/examples/custom.rs)

## hotkeys

**`/`**: open search

**`:`**: open targeting

**`tab/shift-tab`**: iterate up/down through search/targeting parts

**`left arrow/right arrow`**: iterate left/right through search/targeting roots

**`esc`**: close search/targeting

## examples

### on the web

All examples are compiled to wasm for both webgl2 and webgpu (check [compatibility](<https://github.com/gpuweb/gpuweb/wiki/Implementation-Status#implementation-status>)) and deployed to github pages.

- [**`world`**](https://github.com/databasedav/aalo/blob/main/examples/world.rs) [webgl2](https://databasedav.github.io/aalo/examples/webgl2/world/) [webgpu](https://databasedav.github.io/aalo/examples/webgpu/world/)

    minimal world inspector

- [**`custom`**](https://github.com/databasedav/aalo/blob/main/examples/custom.rs) [webgl2](https://databasedav.github.io/aalo/examples/webgl2/custom/) [webgpu](https://databasedav.github.io/aalo/examples/webgpu/custom/)

    custom frontend for a field and a component

## Bevy compatibility

|bevy|aalo|
|-|-|

## license
All code in this repository is dual-licensed under either:

- MIT License ([LICENSE-MIT](https://github.com/databasedav/aalo/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/databasedav/aalo/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.

### your contributions
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
