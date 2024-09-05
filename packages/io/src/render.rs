use crate::{
    traits::{AnyComponent, AnyComponentProps, ComponentProps},
    Element, ElementKey,
};
use crossterm::{
    cursor::{self},
    queue, terminal, Command, QueueableCommand,
};
use flashy_element::ElementType;
use futures::future::select_all;
use std::{
    any::Any,
    collections::HashMap,
    future::Future,
    io::{stdout, Write},
    mem,
};
pub use taffy::NodeId;
use taffy::{AvailableSpace, Layout, Point, Size, Style, TaffyTree};

struct InstantiatedComponent {
    node_id: NodeId,
    component: Box<dyn AnyComponent>,
}

pub struct Components {
    components: HashMap<ElementKey, InstantiatedComponent>,
}

impl Default for Components {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
}

pub struct ComponentsUpdater<'a> {
    updater: ComponentUpdater<'a>,
    components: &'a mut Components,
    used_components: HashMap<ElementKey, InstantiatedComponent>,
}

#[derive(Clone)]
pub struct AnyElement {
    key: ElementKey,
    props: Box<dyn AnyComponentProps>,
}

impl<T> From<Element<T>> for AnyElement
where
    T: ElementType + 'static,
    <T as ElementType>::Props: ComponentProps + Clone,
{
    fn from(e: Element<T>) -> Self {
        Self {
            key: e.key,
            props: Box::new(e.props),
        }
    }
}

impl<'a> ComponentsUpdater<'a> {
    pub fn update<E: Into<AnyElement>>(&mut self, e: E) {
        let e: AnyElement = e.into();
        let mut component: InstantiatedComponent = match self.components.components.remove(&e.key) {
            Some(mut component) if component.type_id() == e.props.component_type_id() => {
                e.props.update_component(&mut component.component);
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
                InstantiatedComponent {
                    node_id: new_node_id,
                    component: e.props.into_new_component(),
                }
            }
        };
        component.component.update(ComponentUpdater {
            node_id: component.node_id,
            layout_engine: self.updater.layout_engine,
        });
        if self
            .used_components
            .insert(e.key.clone(), component)
            .is_some()
        {
            panic!("duplicate key for sibling components: {}", e.key);
        }
    }
}

impl<'a> Drop for ComponentsUpdater<'a> {
    fn drop(&mut self) {
        for (_, component) in self.components.components.drain() {
            self.updater
                .layout_engine
                .remove(component.node_id)
                .expect("we should be able to remove the node");
        }
        mem::swap(&mut self.components.components, &mut self.used_components);
    }
}

impl Components {
    pub fn updater<'a>(&'a mut self, updater: ComponentUpdater<'a>) -> ComponentsUpdater<'a> {
        ComponentsUpdater {
            updater,
            used_components: HashMap::with_capacity(self.components.len()),
            components: self,
        }
    }

    pub fn render(&self, renderer: &mut ComponentRenderer<'_>) {
        for (_, component) in self.components.iter() {
            renderer.for_child_node(component.node_id, |renderer| {
                component.component.render(renderer);
            });
        }
    }

    pub async fn wait(&mut self) {
        select_all(
            self.components
                .iter_mut()
                .map(|(_, component)| component.component.wait()),
        )
        .await;
    }
}

pub struct ComponentUpdater<'a> {
    node_id: NodeId,
    layout_engine: &'a mut TaffyTree<LayoutEngineNodeContext>,
}

impl<'a> ComponentUpdater<'a> {
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
}

struct RenderContext<'a> {
    position: Point<u16>,
    layout_engine: &'a TaffyTree<LayoutEngineNodeContext>,
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
    fn for_child_node<F>(&mut self, node_id: NodeId, f: F)
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
struct LayoutEngineNodeContext {
    measure_func: Option<MeasureFunc>,
}

pub struct Tree {
    layout_engine: TaffyTree<LayoutEngineNodeContext>,
    root_component: Box<dyn AnyComponent>,
    root_node_id: NodeId,
}

impl Tree {
    fn new(e: AnyElement) -> Self {
        let root_component = e.props.into_new_component();
        let mut layout_engine = TaffyTree::new();
        let root_node_id = layout_engine
            .new_leaf_with_context(Style::default(), LayoutEngineNodeContext::default())
            .expect("we should be able to add the root");
        Self {
            layout_engine,
            root_component,
            root_node_id,
        }
    }

    fn render(&mut self) {
        self.root_component.update(ComponentUpdater {
            node_id: self.root_node_id,
            layout_engine: &mut self.layout_engine,
        });

        let (width, _) = terminal::size().expect("we should be able to get the terminal size");
        let (x, y) = cursor::position().expect("we should be able to get the cursor position");

        self.layout_engine
            .compute_layout_with_measure(
                self.root_node_id,
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
            node_id: self.root_node_id,
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

    async fn render_loop(&mut self) -> ! {
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

pub trait ElementExt {
    fn print(self);
    fn render(self) -> impl Future<Output = ()>;
}

impl<T: Into<AnyElement>> ElementExt for T {
    fn print(self) {
        let mut tree = Tree::new(self.into());
        tree.render();
        println!("");
    }

    async fn render(self) {
        let mut tree = Tree::new(self.into());
        tree.render_loop().await;
    }
}
