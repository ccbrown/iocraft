<div align="center">
  <h1><code>iocraft</code></h1>

  <p>
    <strong>✨ A Rust crate for beautiful, artisanally crafted CLIs and text-based IO. ✨</strong>
  </p>

  <p>
    <a href="https://github.com/ccbrown/iocraft/actions"><img src="https://img.shields.io/github/actions/workflow/status/ccbrown/iocraft/commit.yaml" alt="GitHub Actions Workflow Status" /></a>
    <a href="https://docs.rs/iocraft/"><img src="https://img.shields.io/docsrs/iocraft" alt="docs.rs" /></a>
    <a href="https://crates.io/crates/iocraft"><img src="https://img.shields.io/crates/v/iocraft" alt="crates.io" /></a>
    <a href="https://app.codecov.io/github/ccbrown/iocraft"><img src="https://img.shields.io/codecov/c/github/ccbrown/iocraft" alt="Codecov" /></a>
  </p>
</div>

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
- Broad support for both Unix and Windows terminals so your UIs look great everywhere.

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

<img src="https://raw.githubusercontent.com/ccbrown/iocraft/refs/heads/main/examples/images/hello-world.png" height=237 />

Your UI is composed primarily via the `element!` macro, which allows you to
declare your UI elements in a React/SwiftUI-like syntax.

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

<img src="https://raw.githubusercontent.com/ccbrown/iocraft/refs/heads/main/examples/images/counter.svg" />

## More Examples

There are many [examples on GitHub](https://github.com/ccbrown/iocraft/tree/main/examples) which
demonstrate various concepts such as tables, progress bars, fullscreen apps,
forms, and more!

<img src="https://raw.githubusercontent.com/ccbrown/iocraft/refs/heads/main/examples/images/table.png" height=402 />
<img src="https://raw.githubusercontent.com/ccbrown/iocraft/refs/heads/main/examples/images/form.png" height=387 />
<img src="https://raw.githubusercontent.com/ccbrown/iocraft/refs/heads/main/examples/images/overlap.png" height=450 />
<img src="https://raw.githubusercontent.com/ccbrown/iocraft/refs/heads/main/examples/images/weather-powershell.png" height=350 />

## Shoutouts

`iocraft` was inspired by [Dioxus](https://github.com/DioxusLabs/dioxus) and
[Ink](https://github.com/vadimdemedes/ink), which you should also check out,
especially if you're building graphical interfaces or interested in using
JavaScript/TypeScript.

You may also want to check out [Ratatui](https://github.com/ratatui/ratatui),
which serves a similar purpose with a less declarative API.

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
