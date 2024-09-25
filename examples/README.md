# Examples

In this directory, you'll find many examples for various concepts and features of [iocraft](https://github.com/ccbrown/iocraft/).

To run any of the examples, use `cargo run --example NAME`. For example, to run the table example: `cargo run --example table`

## Table of Contents

|Example|Preview|
|---|:---:|
|[borders.rs](./borders.rs)<br />Showcases various border styles.|![preview](./images/borders.png)|
|[context.rs](./context.rs)<br />Demonstrates using a custom context via `ContextProvider` and `use_context`.|![preview](./images/context.png)|
|[counter.rs](./counter.rs)<br />Renders a dynamic component which spawns a future to increment a counter every 100ms.|![preview](./images/counter.png)|
|[form.rs](./form.rs)<br />Displays a form prompting the user for input into multiple text fields. Uses mutable reference props to surface the user's input to the caller once the form is submitted.|![preview](./images/form.png)|
|[fullscreen.rs](./fullscreen.rs)<br />Takes over the full terminal, rendering to an alternate buffer and preventing the user from scrolling.|![preview](./images/fullscreen.png)|
|[hello_world.rs](./hello_world.rs)<br />Hello, world!|![preview](./images/hello-world.png)|
|[progress_bar.rs](./progress_bar.rs)<br />Renders a dynamic progress bar which fills up and then exits.|![preview](./images/progress_bar.png)|
|[table.rs](./table.rs)<br />Displays a list of users provided by reference via properties.|![preview](./images/table.png)|
|[use_input.rs](./use_input.rs)<br />Demonstrates using keyboard input to move a 👾.|![preview](./images/use_input.png)|
|[use_output.rs](./use_output.rs)<br />Continuously logs text output above the rendered component.|![preview](./images/use_output.png)|
|[weather.rs](./weather.rs)<br />Demonstrates asynchronous loading of data from remote APIs in response to user input.|![preview](./images/weather.png)|
