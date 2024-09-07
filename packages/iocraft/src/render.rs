use crate::{
    canvas::{Canvas, CanvasSubviewMut},
    component::{Components, InstantiatedComponent},
    AnyElement,
};
use crossterm::{cursor, queue, terminal, QueueableCommand};
use std::{
    collections::HashMap,
    io::{stdout, Write},
    mem,
};
pub use taffy::NodeId;
use taffy::{AvailableSpace, Layout, Point, Size, Style, TaffyTree};

pub struct ComponentUpdater<'a> {
    node_id: NodeId,
    children: &'a mut Components,
    layout_engine: &'a mut LayoutEngine,
}

impl<'a> ComponentUpdater<'a> {
    pub(crate) fn new(
        node_id: NodeId,
        children: &'a mut Components,
        layout_engine: &'a mut LayoutEngine,
    ) -> Self {
        Self {
            node_id,
            children,
            layout_engine,
        }
    }

    pub fn set_layout_style(&mut self, layout_style: taffy::style::Style) {
        self.layout_engine
            .set_style(self.node_id, layout_style)
            .expect("we should be able to set the style");
    }

    pub fn set_measure_func(&mut self, measure_func: MeasureFunc) {
        self.layout_engine
            .get_node_context_mut(self.node_id)
            .expect("we should be able to get the node")
            .measure_func = Some(measure_func);
        self.layout_engine
            .mark_dirty(self.node_id)
            .expect("we should be able to mark the node as dirty");
    }

    pub fn update_children<I, T>(&mut self, children: I)
    where
        I: IntoIterator<Item = T>,
        T: Into<AnyElement>,
    {
        let mut used_components = HashMap::with_capacity(self.children.components.len());

        for child in children {
            let e: AnyElement = child.into();
            let (key, props) = e.into_key_and_props();
            let mut component: InstantiatedComponent = match self.children.components.remove(&key) {
                Some(mut component)
                    if component.component().type_id() == props.component_type_id() =>
                {
                    component.set_props(props);
                    component
                }
                _ => {
                    let new_node_id = self
                        .layout_engine
                        .new_leaf_with_context(Style::default(), LayoutEngineNodeContext::default())
                        .expect("we should be able to add the node");
                    self.layout_engine
                        .add_child(self.node_id, new_node_id)
                        .expect("we should be able to add the child");
                    InstantiatedComponent::new(new_node_id, props)
                }
            };
            component.update(self.layout_engine);
            if used_components.insert(key.clone(), component).is_some() {
                panic!("duplicate key for sibling components: {}", key);
            }
        }

        for (_, component) in self.children.components.drain() {
            self.layout_engine
                .remove(component.node_id())
                .expect("we should be able to remove the node");
        }
        mem::swap(&mut self.children.components, &mut used_components);
    }
}

struct RenderContext<'a> {
    layout_engine: &'a LayoutEngine,
    canvas: &'a mut Canvas,
}

pub struct ComponentRenderer<'a> {
    node_id: NodeId,
    node_position: Point<u16>,
    node_size: Size<u16>,
    context: RenderContext<'a>,
}

impl<'a> ComponentRenderer<'a> {
    /// Gets the calculated layout of the current node.
    pub fn layout(&self) -> Layout {
        self.context
            .layout_engine
            .layout(self.node_id)
            .expect("we should be able to get the layout")
            .clone()
    }

    pub fn canvas(&mut self) -> CanvasSubviewMut {
        self.context.canvas.subview_mut(
            self.node_position.x as usize,
            self.node_position.y as usize,
            self.node_size.width as usize,
            self.node_size.height as usize,
            true,
        )
    }

    /// Prepares to begin rendering a node by moving to the node's position and invoking the given
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

struct Tree {
    layout_engine: LayoutEngine,
    root_component: InstantiatedComponent,
}

impl Tree {
    fn new(e: AnyElement) -> Self {
        let (_, props) = e.into_key_and_props();
        let mut layout_engine = TaffyTree::new();
        let root_node_id = layout_engine
            .new_leaf_with_context(Style::default(), LayoutEngineNodeContext::default())
            .expect("we should be able to add the root");
        Self {
            layout_engine,
            root_component: InstantiatedComponent::new(root_node_id, props),
        }
    }

    fn render(&mut self, max_width: Option<usize>) -> Canvas {
        self.root_component.update(&mut self.layout_engine);

        self.layout_engine
            .compute_layout_with_measure(
                self.root_component.node_id(),
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

        let root_layout = self
            .layout_engine
            .layout(self.root_component.node_id())
            .expect("we should be able to get the root layout");
        let mut canvas = Canvas::new(root_layout.size.width as _);
        let mut renderer = ComponentRenderer {
            node_id: self.root_component.node_id(),
            node_position: Point { x: 0, y: 0 },
            node_size: Size {
                width: root_layout.size.width as _,
                height: root_layout.size.height as _,
            },
            context: RenderContext {
                layout_engine: &self.layout_engine,
                canvas: &mut canvas,
            },
        };
        self.root_component.render(&mut renderer);
        canvas
    }

    async fn terminal_render_loop(&mut self) -> ! {
        let mut dest = stdout();
        queue!(dest, cursor::SavePosition).expect("we should be able to queue commands");
        loop {
            dest.queue(cursor::RestorePosition)
                .expect("we should be able to queue commands");
            dest.flush().expect("we should be able to flush the output");
            let (width, _) = terminal::size().expect("we should be able to get the terminal size");
            let canvas = self.render(Some(width as _));
            canvas
                .write_ansi(stdout())
                .expect("we should be able to write to stdout");
            self.root_component.wait().await;
        }
    }
}

pub fn render<E: Into<AnyElement>>(e: E, max_width: Option<usize>) -> Canvas {
    let mut tree = Tree::new(e.into());
    tree.render(max_width)
}

pub(crate) async fn terminal_render_loop<E: Into<AnyElement>>(e: E) -> ! {
    let mut tree = Tree::new(e.into());
    tree.terminal_render_loop().await
}
