# aalo [আলো](https://translate.google.com/?sl=bn&tl=en&text=%E0%A6%86%E0%A6%B2%E0%A7%8B&op=translate)

```text
in bengali, aalo means "light" (i.e. photons), not to be confused with haalka !
```

[aalo](https://github.com/databasedav/aalo) is a [haalka](https://github.com/databasedav/haalka) port (in progress) of [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui).

## registering custom frontends

use `register_frontend`, passing in a fully qualified type path and a function that returns an `impl Bundle` e.g. `Node`

```rust
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
