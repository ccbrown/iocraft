# iocraft [![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/ccbrown/iocraft/commit.yaml)](https://github.com/ccbrown/iocraft/actions) [![docs.rs](https://img.shields.io/docsrs/iocraft)](https://docs.rs/iocraft/) [![Codecov](https://img.shields.io/codecov/c/github/ccbrown/iocraft)](https://app.codecov.io/github/ccbrown/iocraft)

`iocraft` is a library for crafting beautiful text output and interfaces for the terminal or
logs. It allows you to easily build complex layouts and interactive elements using a
declarative API.

## Features

- Define your UI using a clean, highly readable syntax.
- Organize your UI using flexbox layouts powered by [`taffy`](https://docs.rs/taffy/).
- Output colored and styled UIs to the terminal or ASCII output anywhere else.
- Create animated or interactive elements with event handling and hooks.
- Build fullscreen terminal applications with ease.
- Pass props and context by reference to avoid unnecessary cloning.

## Getting Started

If you're familiar with React, you'll feel right at home with `iocraft`. It uses all the same
concepts, but is text-focused and made for Rust.

Here's your first `iocraft` program:

```rust
use iocraft::prelude::*;

fn main() {
    element! {
        Box(
            border_style: BorderStyle::Round,
            border_color: Color::Blue,
        ) {
            Text(content: "Hello, world!")
        }
    }
    .print();
}
```

Your UI is composed primarily via the `element!` macro, which allows you to
declare your UI elements in a SwiftUI-like syntax.

`iocraft` provides a few built-in components, such as `Box`, `Text`, and
`TextInput`, but you can also create your own using the `#[component]` macro.

For example, here's a custom component that uses a hook to display a counter
which increments every 100ms:

```rust
use iocraft::prelude::*;
use std::time::Duration;

#[component]
fn Counter(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut count = hooks.use_state(|| 0);

    hooks.use_future(async move {
        loop {
            smol::Timer::after(Duration::from_millis(100)).await;
            count += 1;
        }
    });

    element! {
        Text(color: Color::Blue, content: format!("counter: {}", count))
    }
}

fn main() {
    smol::block_on(element!(Counter).render_loop()).unwrap();
}
```

## More Examples

There are many [examples here on GitHub](https://github.com/ccbrown/iocraft/tree/main/examples)
which demonstrate various concepts and how to use all of `iocraft`'s features.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
