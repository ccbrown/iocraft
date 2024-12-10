use crate::{
    canvas::{Canvas, CanvasSubviewMut},
    component::{ComponentHelperExt, Components, InstantiatedComponent},
    context::{Context, ContextStack, SystemContext},
    element::{ElementExt, ElementKey},
    props::AnyProps,
    terminal::{MockTerminalConfig, MockTerminalOutputStream, Terminal, TerminalEvents},
};
use crossterm::{execute, terminal};
use futures::{
    future::{select, FutureExt, LocalBoxFuture},
    stream::{Stream, StreamExt},
};
use indexmap::IndexMap;
use std::{
    any::Any,
    cell::{Ref, RefMut},
    io, mem,
    pin::Pin,
    task::{self, Poll},
};
use taffy::{AvailableSpace, Layout, NodeId, Point, Size, Style, TaffyTree};
use uuid::Uuid;

pub(crate) struct UpdateContext<'a> {
    terminal: Option<&'a mut Terminal>,
    layout_engine: &'a mut LayoutEngine,
    did_clear_terminal_output: bool,
}

/// Provides information and operations that low level component implementations may need to
/// utilize during the update phase.
pub struct ComponentUpdater<'a, 'b: 'a, 'c: 'a> {
    node_id: NodeId,
    transparent_layout: bool,
    children: &'a mut Components,
    unattached_child_node_ids: Option<&'a mut Vec<NodeId>>,
    context: &'a mut UpdateContext<'b>,
    component_context_stack: &'a mut ContextStack<'c>,
}

impl<'a, 'b, 'c> ComponentUpdater<'a, 'b, 'c> {
    pub(crate) fn new(
        node_id: NodeId,
        children: &'a mut Components,
        unattached_child_node_ids: Option<&'a mut Vec<NodeId>>,
        context: &'a mut UpdateContext<'b>,
        component_context_stack: &'a mut ContextStack<'c>,
    ) -> Self {
        Self {
            node_id,
            transparent_layout: false,
            children,
            unattached_child_node_ids,
            context,
            component_context_stack,
        }
    }

    /// Puts the terminal into raw mode if it isn't already, and returns a stream of terminal
    /// events.
    pub fn terminal_events(&mut self) -> Option<TerminalEvents> {
        self.context.terminal.as_mut().and_then(|t| t.events().ok())
    }

    /// Returns whether the terminal is in raw mode.
    pub fn is_terminal_raw_mode_enabled(&self) -> bool {
        self.context
            .terminal
            .as_ref()
            .map(|t| t.is_raw_mode_enabled())
            .unwrap_or(false)
    }

    /// Removes the currently rendered output from the terminal, e.g. to allow for the printing of
    /// output above the component.
    pub fn clear_terminal_output(&mut self) {
        if !self.context.did_clear_terminal_output {
            if let Some(terminal) = self.context.terminal.as_mut() {
                terminal.clear_canvas().unwrap();
            }
            self.context.did_clear_terminal_output = true;
        }
    }

    #[doc(hidden)]
    pub fn component_context_stack(&self) -> &ContextStack<'c> {
        self.component_context_stack
    }

    /// Gets an immutable reference to context of the given type.
    pub fn get_context<T: Any>(&self) -> Option<Ref<T>> {
        self.component_context_stack.get_context()
    }

    /// Gets a mutable reference to context of the given type.
    pub fn get_context_mut<T: Any>(&self) -> Option<RefMut<T>> {
        self.component_context_stack.get_context_mut()
    }

    /// Sets the layout style of the current component.
    pub fn set_layout_style(&mut self, layout_style: taffy::style::Style) {
        self.context
            .layout_engine
            .set_style(self.node_id, layout_style)
            .expect("we should be able to set the style");
    }

    /// Sets the measure function of the current component, which is invoked to calculate the area
    /// that the component's content should occupy.
    pub fn set_measure_func(&mut self, measure_func: MeasureFunc) {
        self.context
            .layout_engine
            .get_node_context_mut(self.node_id)
            .expect("we should be able to get the node")
            .measure_func = Some(measure_func);
        self.context
            .layout_engine
            .mark_dirty(self.node_id)
            .expect("we should be able to mark the node as dirty");
    }

    /// If set to `true`, the layout of the current component will be transparent, meaning that
    /// children will effectively be direct descendants of the parent of the current component for
    /// layout purposes.
    pub fn set_transparent_layout(&mut self, transparent_layout: bool) {
        self.transparent_layout = transparent_layout;
    }

    pub(crate) fn has_transparent_layout(&self) -> bool {
        self.transparent_layout
    }

    /// Updates the children of the current component.
    pub fn update_children<I, T>(&mut self, children: I, context: Option<Context>)
    where
        I: IntoIterator<Item = T>,
        T: ElementExt,
    {
        self.component_context_stack
            .with_context(context, |component_context_stack| {
                let mut used_components = IndexMap::with_capacity(self.children.components.len());

                let mut direct_child_node_ids = Vec::new();
                let child_node_ids = if self.transparent_layout {
                    self.unattached_child_node_ids
                        .as_deref_mut()
                        .unwrap_or(&mut direct_child_node_ids)
                } else {
                    &mut direct_child_node_ids
                };

                for mut child in children {
                    let mut component: InstantiatedComponent =
                        match self.children.components.swap_remove(child.key()) {
                            Some(component)
                                if component.component().type_id()
                                    == child.helper().component_type_id() =>
                            {
                                child_node_ids.push(component.node_id());
                                component
                            }
                            _ => {
                                let new_node_id = self
                                    .context
                                    .layout_engine
                                    .new_leaf_with_context(
                                        Style::default(),
                                        LayoutEngineNodeContext::default(),
                                    )
                                    .expect("we should be able to add the node");
                                child_node_ids.push(new_node_id);
                                let h = child.helper();
                                InstantiatedComponent::new(new_node_id, child.props_mut(), h)
                            }
                        };
                    component.update(
                        self.context,
                        Some(child_node_ids),
                        component_context_stack,
                        child.props_mut(),
                    );

                    let mut child_key = child.key().clone();
                    while used_components.contains_key(&child_key) {
                        child_key = ElementKey::new(Uuid::new_v4().as_u128());
                    }
                    used_components.insert(child_key, component);
                }

                self.context
                    .layout_engine
                    .set_children(self.node_id, &direct_child_node_ids)
                    .expect("we should be able to set the children");

                for (_, component) in self.children.components.drain(..) {
                    self.context
                        .layout_engine
                        .remove(component.node_id())
                        .expect("we should be able to remove the node");
                }
                mem::swap(&mut self.children.components, &mut used_components);
            });
    }
}

struct DrawContext<'a> {
    layout_engine: &'a LayoutEngine,
    canvas: &'a mut Canvas,
}

/// Provides information and operations that low level component implementations may need to
/// utilize during the draw phase.
pub struct ComponentDrawer<'a> {
    node_id: NodeId,
    node_position: Point<u16>,
    node_size: Size<u16>,
    context: DrawContext<'a>,
}

impl<'a> ComponentDrawer<'a> {
    /// Gets the calculated layout of the current node.
    pub fn layout(&self) -> Layout {
        *self
            .context
            .layout_engine
            .layout(self.node_id)
            .expect("we should be able to get the layout")
    }

    /// Gets the size of the component.
    pub fn size(&self) -> Size<u16> {
        self.node_size
    }

    /// Gets the position of the component relative to the top left of the canvas.
    pub fn canvas_position(&self) -> Point<u16> {
        self.node_position
    }

    /// Gets the region of the canvas that the component should be drawn to.
    pub fn canvas(&mut self) -> CanvasSubviewMut {
        self.context.canvas.subview_mut(
            self.node_position.x as usize,
            self.node_position.y as usize,
            self.node_size.width as usize,
            self.node_size.height as usize,
            true,
        )
    }

    /// Prepares to begin drawing a node by moving to the node's position and invoking the given
    /// closure.
    pub(crate) fn for_child_node_layout<F>(&mut self, node_id: NodeId, f: F)
    where
        F: FnOnce(&mut Self),
    {
        let old_node_id = self.node_id;
        let old_node_position = self.node_position;
        let old_node_size = self.node_size;
        self.node_id = node_id;
        let layout = self.layout();
        self.node_position = Point {
            x: self.node_position.x + layout.location.x as u16,
            y: self.node_position.y + layout.location.y as u16,
        };
        self.node_size = Size {
            width: layout.size.width as u16,
            height: layout.size.height as u16,
        };
        f(self);
        self.node_id = old_node_id;
        self.node_position = old_node_position;
        self.node_size = old_node_size;
    }
}

type MeasureFunc = Box<dyn Fn(Size<Option<f32>>, Size<AvailableSpace>, &Style) -> Size<f32> + Send>;

#[derive(Default)]
pub(crate) struct LayoutEngineNodeContext {
    measure_func: Option<MeasureFunc>,
}

pub(crate) type LayoutEngine = TaffyTree<LayoutEngineNodeContext>;

struct Tree<'a> {
    layout_engine: LayoutEngine,
    wrapper_node_id: NodeId,
    root_component: InstantiatedComponent,
    root_component_props: AnyProps<'a>,
    system_context: SystemContext,
}

struct RenderOutput {
    canvas: Canvas,
    did_clear_terminal_output: bool,
}

impl<'a> Tree<'a> {
    fn new(mut props: AnyProps<'a>, helper: Box<dyn ComponentHelperExt>) -> Self {
        let mut layout_engine = TaffyTree::new();
        let root_node_id = layout_engine
            .new_leaf_with_context(Style::default(), LayoutEngineNodeContext::default())
            .expect("we should be able to add the root");
        let wrapper_node_id = layout_engine
            .new_with_children(Style::default(), &[root_node_id])
            .expect("we should be able to add the root");
        Self {
            layout_engine,
            wrapper_node_id,
            root_component: InstantiatedComponent::new(root_node_id, props.borrow(), helper),
            root_component_props: props,
            system_context: SystemContext::new(),
        }
    }

    fn render(
        &mut self,
        max_width: Option<usize>,
        terminal: Option<&mut Terminal>,
    ) -> RenderOutput {
        let did_clear_terminal_output = {
            let mut context = UpdateContext {
                terminal,
                layout_engine: &mut self.layout_engine,
                did_clear_terminal_output: false,
            };
            let mut component_context_stack = ContextStack::root(&mut self.system_context);
            self.root_component.update(
                &mut context,
                None,
                &mut component_context_stack,
                self.root_component_props.borrow(),
            );
            context.did_clear_terminal_output
        };

        self.layout_engine
            .compute_layout_with_measure(
                self.wrapper_node_id,
                Size {
                    width: max_width
                        .map(|w| AvailableSpace::Definite(w as _))
                        .unwrap_or(AvailableSpace::MaxContent),
                    height: AvailableSpace::MaxContent,
                },
                |known_dimensions, available_space, _node_id, node_context, style| {
                    match node_context.and_then(|cx| cx.measure_func.as_ref()) {
                        Some(f) => f(known_dimensions, available_space, style),
                        None => Size::ZERO,
                    }
                },
            )
            .expect("we should be able to compute the layout");

        let wrapper_layout = self
            .layout_engine
            .layout(self.wrapper_node_id)
            .expect("we should be able to get the wrapper layout");
        let mut canvas = Canvas::new(
            wrapper_layout.size.width as _,
            wrapper_layout.size.height as _,
        );
        let root_layout = self
            .layout_engine
            .layout(self.root_component.node_id())
            .expect("we should be able to get the root layout");
        let mut drawer = ComponentDrawer {
            node_id: self.root_component.node_id(),
            node_position: Point {
                x: root_layout.location.x as _,
                y: root_layout.location.y as _,
            },
            node_size: Size {
                width: root_layout.size.width as _,
                height: root_layout.size.height as _,
            },
            context: DrawContext {
                layout_engine: &self.layout_engine,
                canvas: &mut canvas,
            },
        };
        self.root_component.draw(&mut drawer);
        RenderOutput {
            canvas,
            did_clear_terminal_output,
        }
    }

    async fn terminal_render_loop(&mut self, mut term: Terminal) -> io::Result<()> {
        let mut prev_canvas: Option<Canvas> = None;
        loop {
            let width = term.width().map(|w| w as usize);
            execute!(term, terminal::BeginSynchronizedUpdate,)?;
            let output = self.render(width, Some(&mut term));
            if output.did_clear_terminal_output || prev_canvas.as_ref() != Some(&output.canvas) {
                if !output.did_clear_terminal_output {
                    term.clear_canvas()?;
                }
                term.write_canvas(&output.canvas)?;
            }
            prev_canvas = Some(output.canvas);
            execute!(term, terminal::EndSynchronizedUpdate)?;
            if self.system_context.should_exit() || term.received_ctrl_c() {
                break;
            }
            select(self.root_component.wait().boxed(), term.wait().boxed()).await;
            if term.received_ctrl_c() {
                break;
            }
        }
        Ok(())
    }
}

pub(crate) fn render<E: ElementExt>(mut e: E, max_width: Option<usize>) -> Canvas {
    let h = e.helper();
    let mut tree = Tree::new(e.props_mut(), h);
    tree.render(max_width, None).canvas
}

pub(crate) async fn terminal_render_loop<E>(mut e: E, term: Terminal) -> io::Result<()>
where
    E: ElementExt,
{
    let h = e.helper();
    let mut tree = Tree::new(e.props_mut(), h);
    tree.terminal_render_loop(term).await
}

pub(crate) struct MockTerminalRenderLoop<'a> {
    output: MockTerminalOutputStream,
    render_loop: LocalBoxFuture<'a, io::Result<()>>,
    render_loop_is_done: bool,
}

impl Stream for MockTerminalRenderLoop<'_> {
    type Item = Canvas;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut();

        if !this.render_loop_is_done && this.render_loop.poll_unpin(cx).is_ready() {
            this.render_loop_is_done = true;
        }

        this.output.poll_next_unpin(cx)
    }
}

pub(crate) fn mock_terminal_render_loop<'a, E>(
    e: E,
    config: MockTerminalConfig,
) -> MockTerminalRenderLoop<'a>
where
    E: ElementExt + 'a,
{
    let (term, output) = Terminal::mock(config);
    MockTerminalRenderLoop {
        render_loop: terminal_render_loop(e, term).boxed_local(),
        render_loop_is_done: false,
        output,
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use futures::stream::StreamExt;
    use macro_rules_attribute::apply;
    use smol_macros::test;
    use std::future::Future;

    #[derive(Default, Props)]
    struct MyInnerComponentProps {
        label: String,
    }

    #[component]
    fn MyInnerComponent(
        mut hooks: Hooks,
        props: &MyInnerComponentProps,
    ) -> impl Into<AnyElement<'static>> {
        let mut counter = hooks.use_state(|| 0);
        counter += 1;
        element! {
            Text(content: format!("render count ({}): {}", props.label, counter))
        }
    }

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut tick = hooks.use_state(|| 0);

        hooks.use_future(async move {
            tick += 1;
        });

        if tick == 1 {
            system.exit();
        }

        element! {
            Box(flex_direction: FlexDirection::Column) {
                Text(content: format!("tick: {}", tick))
                MyInnerComponent(label: "a")
                // without a key, these next elements may not be re-used across renders
                #((0..2).map(|i| element! { MyInnerComponent(label: format!("b{}", i)) }))
                // with a key, these next elements will definitely be re-used across renders
                #((0..2).map(|i| element! { MyInnerComponent(key: i, label: format!("c{}", i)) }))
            }
        }
    }

    #[apply(test!)]
    async fn test_terminal_render_loop() {
        let canvases: Vec<_> =
            mock_terminal_render_loop(element!(MyComponent), MockTerminalConfig::default())
                .collect()
                .await;
        let actual = canvases.iter().map(|c| c.to_string()).collect::<Vec<_>>();
        let expected = vec![
            "tick: 0\nrender count (a): 1\nrender count (b0): 1\nrender count (b1): 1\nrender count (c0): 1\nrender count (c1): 1\n",
            "tick: 1\nrender count (a): 2\nrender count (b0): 2\nrender count (b1): 1\nrender count (c0): 2\nrender count (c1): 2\n",
        ];
        assert_eq!(actual, expected);
    }

    async fn await_send_future<F: Future<Output = ()> + Send>(f: F) {
        f.await;
    }

    // Make sure terminal_render_loop can be sent across threads.
    #[apply(test!)]
    async fn test_terminal_render_loop_send() {
        let (term, _output) = Terminal::mock(MockTerminalConfig::default());
        await_send_future(async move {
            terminal_render_loop(element!(MyComponent), term)
                .await
                .unwrap();
        })
        .await;
    }

    #[component]
    fn FullWidthComponent() -> impl Into<AnyElement<'static>> {
        element! {
            Box(height: 2, width: 100pct, border_style: BorderStyle::Classic)
        }
    }

    #[test]
    fn test_transparent_layout() {
        // For layout purposes, components defined with #[component] should not introduce a new
        // node in between its parent and child.
        let actual = element! {
            Box(width: 10) {
                FullWidthComponent
            }
        }
        .to_string();
        assert_eq!(actual, "+--------+\n+--------+\n",);
    }
}
