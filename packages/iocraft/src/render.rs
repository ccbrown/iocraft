use crate::{
    canvas::{Canvas, CanvasSubviewMut},
    component::{ComponentHelperExt, Components, InstantiatedComponent},
    context::{Context, ContextStack, SystemContext},
    element::ElementExt,
    props::AnyProps,
    terminal::{Terminal, TerminalEvents},
};
use crossterm::{execute, terminal};
use futures::future::{select, FutureExt};
use std::{
    any::Any,
    cell::{Ref, RefMut},
    collections::HashMap,
    io::{self, Write},
    mem,
};
use taffy::{AvailableSpace, Layout, NodeId, Point, Size, Style, TaffyTree};

pub(crate) struct UpdateContext<'a> {
    terminal: Option<&'a mut Terminal>,
    layout_engine: &'a mut LayoutEngine,
    lines_to_rewind_to_clear: usize,
    did_clear_terminal_output: bool,
}

/// Provides information and operations that low level component implementations may need to
/// utilize during the update phase.
pub struct ComponentUpdater<'a, 'b: 'a, 'c: 'a> {
    node_id: NodeId,
    children: &'a mut Components,
    context: &'a mut UpdateContext<'b>,
    component_context_stack: &'a mut ContextStack<'c>,
}

impl<'a, 'b, 'c> ComponentUpdater<'a, 'b, 'c> {
    pub(crate) fn new(
        node_id: NodeId,
        children: &'a mut Components,
        context: &'a mut UpdateContext<'b>,
        component_context_stack: &'a mut ContextStack<'c>,
    ) -> Self {
        Self {
            node_id,
            children,
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
                terminal
                    .rewind_lines(self.context.lines_to_rewind_to_clear as _)
                    .unwrap();
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

    /// Updates the children of the current component.
    pub fn update_children<I, T>(&mut self, children: I, context: Option<Context>)
    where
        I: IntoIterator<Item = T>,
        T: ElementExt,
    {
        self.component_context_stack
            .with_context(context, |component_context_stack| {
                let mut used_components = HashMap::with_capacity(self.children.components.len());

                for mut child in children {
                    let mut component: InstantiatedComponent =
                        match self.children.components.remove(child.key()) {
                            Some(component)
                                if component.component().type_id()
                                    == child.helper().component_type_id() =>
                            {
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
                                self.context
                                    .layout_engine
                                    .add_child(self.node_id, new_node_id)
                                    .expect("we should be able to add the child");
                                let h = child.helper();
                                InstantiatedComponent::new(new_node_id, child.props_mut(), h)
                            }
                        };
                    component.update(self.context, component_context_stack, child.props_mut());
                    if used_components
                        .insert(child.key().clone(), component)
                        .is_some()
                    {
                        panic!("duplicate key for sibling components: {}", child.key());
                    }
                }

                for (_, component) in self.children.components.drain() {
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
    pub(crate) fn for_child_node<F>(&mut self, node_id: NodeId, f: F)
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

type MeasureFunc = Box<dyn Fn(Size<Option<f32>>, Size<AvailableSpace>, &Style) -> Size<f32>>;

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
        lines_to_rewind_to_clear: usize,
    ) -> RenderOutput {
        let did_clear_terminal_output = {
            let mut context = UpdateContext {
                terminal,
                layout_engine: &mut self.layout_engine,
                did_clear_terminal_output: false,
                lines_to_rewind_to_clear,
            };
            let mut component_context_stack = ContextStack::root(&mut self.system_context);
            self.root_component.update(
                &mut context,
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

    async fn terminal_render_loop<W>(&mut self, mut w: W) -> io::Result<()>
    where
        W: Write,
    {
        let mut terminal = Terminal::new()?;
        let mut prev_canvas: Option<Canvas> = None;
        loop {
            let width = terminal.width().ok().map(|w| w as usize);
            execute!(w, terminal::BeginSynchronizedUpdate,)?;
            let lines_to_rewind_to_clear = prev_canvas.as_ref().map_or(0, |c| c.height());
            let output = self.render(width, Some(&mut terminal), lines_to_rewind_to_clear);
            if output.did_clear_terminal_output || prev_canvas.as_ref() != Some(&output.canvas) {
                if !output.did_clear_terminal_output {
                    terminal.rewind_lines(lines_to_rewind_to_clear as _)?;
                }
                output.canvas.write_ansi(&mut w)?;
            }
            prev_canvas = Some(output.canvas);
            execute!(w, terminal::EndSynchronizedUpdate)?;
            if self.system_context.should_exit() || terminal.received_ctrl_c() {
                break;
            }
            select(
                self.root_component.wait().boxed_local(),
                terminal.wait().boxed_local(),
            )
            .await;
            if terminal.received_ctrl_c() {
                break;
            }
        }
        mem::drop(terminal);
        Ok(())
    }
}

pub(crate) fn render<E: ElementExt>(mut e: E, max_width: Option<usize>) -> Canvas {
    let h = e.helper();
    let mut tree = Tree::new(e.props_mut(), h);
    tree.render(max_width, None, 0).canvas
}

pub(crate) async fn terminal_render_loop<E, W>(mut e: E, dest: W) -> io::Result<()>
where
    E: ElementExt,
    W: Write,
{
    let h = e.helper();
    let mut tree = Tree::new(e.props_mut(), h);
    tree.terminal_render_loop(dest).await
}

#[cfg(test)]
mod tests {
    use crate::{hooks::UseFuture, prelude::*};
    use macro_rules_attribute::apply;
    use smol_macros::test;

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let mut counter = hooks.use_state(|| 0);

        hooks.use_future(async move {
            counter += 1;
        });

        if counter == 1 {
            system.exit();
        }

        element! {
            Text(content: format!("count: {}", counter))
        }
    }

    #[apply(test!)]
    async fn test_terminal_render_loop() {
        let mut buf = Vec::new();
        terminal_render_loop(element!(MyComponent), &mut buf)
            .await
            .unwrap();
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("count: 0"));
        assert!(output.contains("count: 1"));
    }
}
