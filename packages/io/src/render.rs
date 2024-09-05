use crate::{
    component::{Components, InstantiatedComponent},
    AnyElement, ElementKey,
};
use crossterm::{cursor, queue, terminal, Command, QueueableCommand};
use std::{
    collections::HashMap,
    io::{stdout, Write},
    mem,
};
pub use taffy::NodeId;
use taffy::{AvailableSpace, Layout, Point, Size, Style, TaffyTree};

struct ComponentsUpdater<'a> {
    updater: ComponentUpdater<'a>,
    used_components: HashMap<ElementKey, InstantiatedComponent>,
}

impl<'a> ComponentsUpdater<'a> {
    pub fn update<E: Into<AnyElement>>(&mut self, e: E) {
        let e: AnyElement = e.into();
        let (key, props) = e.into_key_and_props();
        let mut component: InstantiatedComponent =
            match self.updater.children.components.remove(&key) {
                Some(mut component)
                    if component.component().type_id() == props.component_type_id() =>
                {
                    component.set_props(props);
                    component
                }
                _ => {
                    let new_node_id = self
                        .updater
                        .layout_engine
                        .new_leaf_with_context(Style::default(), LayoutEngineNodeContext::default())
                        .expect("we should be able to add the node");
                    self.updater
                        .layout_engine
                        .add_child(self.updater.node_id, new_node_id)
                        .expect("we should be able to add the child");
                    InstantiatedComponent::new(new_node_id, props.into_new_component())
                }
            };
        component.update(self.updater.layout_engine);
        if self
            .used_components
            .insert(key.clone(), component)
            .is_some()
        {
            panic!("duplicate key for sibling components: {}", key);
        }
    }
}

impl<'a> Drop for ComponentsUpdater<'a> {
    fn drop(&mut self) {
        for (_, component) in self.updater.children.components.drain() {
            self.updater
                .layout_engine
                .remove(component.node_id())
                .expect("we should be able to remove the node");
        }
        mem::swap(
            &mut self.updater.children.components,
            &mut self.used_components,
        );
    }
}

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

    pub fn update_children<I, T>(self, children: I)
    where
        I: IntoIterator<Item = T>,
        T: Into<AnyElement>,
    {
        let mut updater = ComponentsUpdater {
            used_components: HashMap::with_capacity(self.children.components.len()),
            updater: self,
        };
        for child in children {
            updater.update(child);
        }
    }
}

struct RenderContext<'a> {
    position: Point<u16>,
    layout_engine: &'a LayoutEngine,
}

pub struct ComponentRenderer<'a> {
    node_id: NodeId,
    node_position: Point<u16>,
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

    /// Moves the cursor to the given position relative to the current node's position.
    pub fn move_cursor(&mut self, x: u16, y: u16) {
        self.context.position = Point {
            x: self.node_position.x + x,
            y: self.node_position.y + y,
        };
        self.queue(cursor::MoveTo(
            self.context.position.x,
            self.context.position.y,
        ));
    }

    /// Queues a command to be executed.
    pub fn queue(&self, command: impl Command) {
        stdout()
            .queue(command)
            .expect("we should be able to queue the command");
    }

    /// Prepares to begin rendering a node by moving to the node's position and invoking the given
    /// closure.
    pub(crate) fn for_child_node<F>(&mut self, node_id: NodeId, f: F)
    where
        F: FnOnce(&mut Self),
    {
        let old_node_id = self.node_id;
        let old_node_position = self.node_position;
        self.node_id = node_id;
        let layout = self.layout();
        self.node_position = Point {
            x: self.node_position.x + layout.location.x as u16,
            y: self.node_position.y + layout.location.y as u16,
        };
        self.context.position = self.node_position;
        self.queue(cursor::MoveTo(
            self.context.position.x,
            self.context.position.y,
        ));
        f(self);
        self.node_id = old_node_id;
        self.node_position = old_node_position;
    }
}

type MeasureFunc = Box<dyn Fn(Size<Option<f32>>, Size<AvailableSpace>, &Style) -> Size<f32>>;

#[derive(Default)]
pub(crate) struct LayoutEngineNodeContext {
    measure_func: Option<MeasureFunc>,
}

pub(crate) type LayoutEngine = TaffyTree<LayoutEngineNodeContext>;

pub(crate) struct Tree {
    layout_engine: LayoutEngine,
    root_component: InstantiatedComponent,
}

impl Tree {
    pub fn new(e: AnyElement) -> Self {
        let root_component = e.into_key_and_props().1.into_new_component();
        let mut layout_engine = TaffyTree::new();
        let root_node_id = layout_engine
            .new_leaf_with_context(Style::default(), LayoutEngineNodeContext::default())
            .expect("we should be able to add the root");
        Self {
            layout_engine,
            root_component: InstantiatedComponent::new(root_node_id, root_component),
        }
    }

    pub fn render(&mut self) {
        self.root_component.update(&mut self.layout_engine);

        let (width, _) = terminal::size().expect("we should be able to get the terminal size");
        let (x, y) = cursor::position().expect("we should be able to get the cursor position");

        self.layout_engine
            .compute_layout_with_measure(
                self.root_component.node_id(),
                Size {
                    width: AvailableSpace::Definite(width as _),
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

        let mut renderer = ComponentRenderer {
            node_id: self.root_component.node_id(),
            node_position: Point { x, y },
            context: RenderContext {
                position: Point { x, y },
                layout_engine: &self.layout_engine,
            },
        };
        self.root_component.render(&mut renderer);
        let root_layout = renderer.layout();
        renderer.move_cursor(
            root_layout.size.width as _,
            root_layout.size.height as u16 - 1,
        );
    }

    pub async fn render_loop(&mut self) -> ! {
        let mut dest = stdout();
        queue!(dest, cursor::SavePosition).expect("we should be able to queue commands");
        loop {
            dest.queue(cursor::RestorePosition)
                .expect("we should be able to queue commands");
            self.render();
            dest.flush().expect("we should be able to flush the output");
            self.root_component.wait().await;
        }
    }
}
