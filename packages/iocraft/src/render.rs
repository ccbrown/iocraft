use crate::{
    canvas::{Canvas, CanvasSubviewMut},
    component::{ComponentHelperExt, Components, InstantiatedComponent},
    context::{Context, ContextStack, SystemContext},
    element::{ElementExt, Output},
    multimap::AppendOnlyMultimap,
    props::AnyProps,
    terminal::{MockTerminalConfig, MockTerminalOutputStream, Terminal, TerminalEvents},
};
use core::{
    any::Any,
    cell::{Ref, RefMut},
    pin::Pin,
    task::{self, Poll},
};
use futures::{
    future::{select, FutureExt, LocalBoxFuture},
    stream::{Stream, StreamExt},
};
use std::io;
use std::sync::{Arc, Mutex};
use taffy::{
    AvailableSpace, Display, Layout, NodeId, Overflow, Point, Rect, Size, Style, TaffyTree,
};

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
    unattached_child_node_ids: &'a mut Vec<NodeId>,
    context: &'a mut UpdateContext<'b>,
    component_context_stack: &'a mut ContextStack<'c>,
}

impl<'a, 'b, 'c> ComponentUpdater<'a, 'b, 'c> {
    pub(crate) fn new(
        node_id: NodeId,
        children: &'a mut Components,
        unattached_child_node_ids: &'a mut Vec<NodeId>,
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

    /// Returns whether we're running in a terminal render loop.
    pub(crate) fn is_terminal_render_loop(&self) -> bool {
        self.context.terminal.is_some()
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
        if transparent_layout && !self.transparent_layout {
            self.context
                .layout_engine
                .set_style(
                    self.node_id,
                    Style {
                        display: Display::None,
                        ..Default::default()
                    },
                )
                .expect("we should be able to set the style");
        }
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
                let mut used_components = AppendOnlyMultimap::default();

                let mut direct_child_node_ids = Vec::new();
                let child_node_ids = if self.transparent_layout {
                    &mut self.unattached_child_node_ids
                } else {
                    &mut direct_child_node_ids
                };

                for mut child in children {
                    let mut component: InstantiatedComponent =
                        match self.children.components.pop_front(child.key()) {
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
                        child_node_ids,
                        component_context_stack,
                        child.props_mut(),
                    );

                    used_components.push_back(child.key().clone(), component);
                }

                self.context
                    .layout_engine
                    .set_children(self.node_id, &direct_child_node_ids)
                    .expect("we should be able to set the children");

                for component in self.children.components.iter() {
                    self.context
                        .layout_engine
                        .remove(component.node_id())
                        .expect("we should be able to remove the node");
                }
                self.children.components = used_components.into();
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
    node_position: Point<i16>,
    node_size: Size<u16>,
    clip_rect: Rect<u16>,
    context: DrawContext<'a>,
}

impl ComponentDrawer<'_> {
    /// Gets the calculated layout of the current node.
    pub fn layout(&self) -> Layout {
        *self
            .context
            .layout_engine
            .layout(self.node_id)
            .expect("we should be able to get the layout")
    }

    /// Gets the style of the current node.
    pub fn style(&self) -> &Style {
        self.context
            .layout_engine
            .style(self.node_id)
            .expect("we should be able to get the style")
    }

    /// Gets the size of the component.
    pub fn size(&self) -> Size<u16> {
        self.node_size
    }

    /// Gets the position of the component relative to the top left of the canvas.
    pub fn canvas_position(&self) -> Point<i16> {
        self.node_position
    }

    /// Gets the region of the canvas that the component should be drawn to.
    pub fn canvas(&mut self) -> CanvasSubviewMut {
        self.context.canvas.subview_mut(
            self.node_position.x as _,
            self.node_position.y as _,
            self.clip_rect.left as _,
            self.clip_rect.top as _,
            self.clip_rect.right.saturating_sub(self.clip_rect.left) as _,
            self.clip_rect.bottom.saturating_sub(self.clip_rect.top) as _,
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
            x: self.node_position.x + layout.location.x as i16,
            y: self.node_position.y + layout.location.y as i16,
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

    /// Prepares to begin drawing a node's children by shrinking the clipping rectangle if necessary.
    pub(crate) fn with_clip_rect_for_children<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        let overflow = self.style().overflow;
        if overflow.x == Overflow::Visible && overflow.y == Overflow::Visible {
            // No need to do anything.
            f(self);
            return;
        }

        let old_clip_rect = self.clip_rect;
        let layout = self.layout();
        if overflow.x != Overflow::Visible {
            self.clip_rect.left = self
                .clip_rect
                .left
                .max((self.node_position.x + layout.border.left as i16).max(0) as u16);
            self.clip_rect.right = self.clip_rect.right.min(
                (self.node_position.x + self.node_size.width as i16 - layout.border.right as i16)
                    .max(0) as u16,
            );
        }
        if overflow.y != Overflow::Visible {
            self.clip_rect.top = self
                .clip_rect
                .top
                .max((self.node_position.y + layout.border.top as i16).max(0) as u16);
            self.clip_rect.bottom = self.clip_rect.bottom.min(
                (self.node_position.y + self.node_size.height as i16 - layout.border.bottom as i16)
                    .max(0) as u16,
            );
        }
        f(self);
        self.clip_rect = old_clip_rect;
    }
}

/// The measure function of the current component, which is invoked to calculate the area that the
/// component's content should occupy.
pub type MeasureFunc =
    Box<dyn Fn(Size<Option<f32>>, Size<AvailableSpace>, &Style) -> Size<f32> + Send>;

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
    fn new(
        mut props: AnyProps<'a>,
        helper: Box<dyn ComponentHelperExt>,
        stdout: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
        stderr: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
        render_to: Output,
    ) -> Self {
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
            system_context: SystemContext::new(stdout, stderr, render_to),
        }
    }

    fn render(
        &mut self,
        max_width: Option<usize>,
        terminal: Option<&mut Terminal>,
    ) -> RenderOutput {
        let mut wrapper_child_node_ids = vec![self.root_component.node_id()];
        let did_clear_terminal_output = {
            let mut context = UpdateContext {
                terminal,
                layout_engine: &mut self.layout_engine,
                did_clear_terminal_output: false,
            };
            let mut component_context_stack = ContextStack::root(&mut self.system_context);
            self.root_component.update(
                &mut context,
                &mut wrapper_child_node_ids,
                &mut component_context_stack,
                self.root_component_props.borrow(),
            );
            context.did_clear_terminal_output
        };
        self.layout_engine
            .set_children(self.wrapper_node_id, &wrapper_child_node_ids)
            .expect("we should be able to set the children");

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
            clip_rect: Rect {
                left: 0,
                right: wrapper_layout.size.width as _,
                top: 0,
                bottom: wrapper_layout.size.height as _,
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
            term.refresh_size();
            let terminal_size = term.size();
            term.synchronized_update(|mut term| {
                let output = self.render(terminal_size.map(|(w, _)| w as usize), Some(&mut term));
                if output.did_clear_terminal_output || prev_canvas.as_ref() != Some(&output.canvas)
                {
                    if !output.did_clear_terminal_output {
                        term.clear_canvas()?;
                    }
                    term.write_canvas(&output.canvas)?;
                }
                prev_canvas = Some(output.canvas);
                Ok(())
            })?;
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
    let system_context = SystemContext::new_default();
    let mut tree = Tree::new(
        e.props_mut(),
        h,
        system_context.stdout(),
        system_context.stderr(),
        system_context.render_to(),
    );
    tree.render(max_width, None).canvas
}

pub(crate) async fn terminal_render_loop<E>(
    e: &mut E,
    term: Terminal,
    stdout: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    stderr: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    render_to: Output,
) -> io::Result<()>
where
    E: ElementExt,
{
    let h = e.helper();
    let mut tree = Tree::new(e.props_mut(), h, stdout, stderr, render_to);
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
    e: &'a mut E,
    config: MockTerminalConfig,
) -> MockTerminalRenderLoop<'a>
where
    E: ElementExt + 'a,
{
    let (term, output) = Terminal::mock(config);
    let system_context = SystemContext::new_default();
    MockTerminalRenderLoop {
        render_loop: terminal_render_loop(
            e,
            term,
            system_context.stdout(),
            system_context.stderr(),
            system_context.render_to(),
        )
        .boxed_local(),
        render_loop_is_done: false,
        output,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use core::future::Future;
    use macro_rules_attribute::apply;
    use smol_macros::test;

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
            View(flex_direction: FlexDirection::Column) {
                Text(content: format!("tick: {}", tick))
                MyInnerComponent(label: "a")
                #((0..2).map(|i| element! { MyInnerComponent(label: format!("b{}", i)) }))
                #((0..2).map(|i| element! { MyInnerComponent(key: i, label: format!("c{}", i)) }))
            }
        }
    }

    #[apply(test!)]
    async fn test_terminal_render_loop() {
        let canvases: Vec<_> =
            mock_terminal_render_loop(&mut element!(MyComponent), MockTerminalConfig::default())
                .collect()
                .await;
        let actual = canvases.iter().map(|c| c.to_string()).collect::<Vec<_>>();
        let expected = vec![
            "tick: 0\nrender count (a): 1\nrender count (b0): 1\nrender count (b1): 1\nrender count (c0): 1\nrender count (c1): 1\n",
            "tick: 1\nrender count (a): 2\nrender count (b0): 2\nrender count (b1): 2\nrender count (c0): 2\nrender count (c1): 2\n",
        ];
        assert_eq!(actual, expected);
    }

    async fn await_send_future<F: Future<Output = io::Result<()>> + Send>(f: F) {
        f.await.unwrap();
    }

    // Make sure terminal_render_loop can be sent across threads.
    #[apply(test!)]
    async fn test_terminal_render_loop_send() {
        let (term, _output) = Terminal::mock(MockTerminalConfig::default());
        await_send_future(terminal_render_loop(&mut element!(MyComponent), term)).await;
    }

    #[component]
    fn FullWidthComponent() -> impl Into<AnyElement<'static>> {
        element! {
            View(height: 2, width: 100pct, border_style: BorderStyle::Classic)
        }
    }

    #[test]
    fn test_transparent_layout() {
        // For layout purposes, components defined with #[component] should not introduce a new
        // node in between its parent and child.
        let actual = element! {
            View(width: 10) {
                FullWidthComponent
            }
        }
        .to_string();
        assert_eq!(actual, "+--------+\n+--------+\n",);
    }

    #[derive(Default, Props)]
    struct AsyncTickerProps {
        ticks: Option<State<i32>>,
    }

    #[component]
    fn AsyncTicker<'a>(
        props: &mut AsyncTickerProps,
        mut hooks: Hooks,
    ) -> impl Into<AnyElement<'a>> {
        let mut ticks = props.ticks.unwrap();
        hooks.use_future(async move {
            ticks += 1;
        });
        element!(View)
    }

    #[component]
    fn AsyncTickerContainer(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let child_ticks = hooks.use_state(|| 0);
        let mut tick = hooks.use_state(|| 0);

        hooks.use_future(async move {
            tick += 1;
        });

        if tick == 5 {
            // make sure our children have all ticked exactly 10 times
            assert_eq!(child_ticks, 10);
            system.exit();
        } else {
            // do a few more render passes
            tick += 1;
        }

        element! {
            View {
                #((0..10).map(|_| {
                    element! {
                        AsyncTicker(ticks: child_ticks)
                    }
                }))
            }
        }
    }

    // This is a regression test for an issue where elements added via iterator without keys would
    // be re-created on every render instead of being recycled.
    #[apply(test!)]
    async fn test_async_ticker_container() {
        let canvases: Vec<_> = mock_terminal_render_loop(
            &mut element!(AsyncTickerContainer),
            MockTerminalConfig::default(),
        )
        .collect()
        .await;
        assert!(!canvases.is_empty());
    }

    #[test]
    fn test_negative_dimensions() {
        let actual = element! {
            View(width: 10, height: 5, position: Position::Relative) {
                View(position: Position::Absolute, left: 10, top: 10, right: 10, bottom: 10, overflow: Overflow::Hidden) {
                    Text(content: "Hello!")
                }
            }
        }
        .to_string();
        assert_eq!(actual, "\n\n\n\n\n",);
    }
}
