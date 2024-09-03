use crate::traits::{AnyComponent, AnyComponentProps, ComponentProps};
use crossterm::{cursor, queue, Command, QueueableCommand};
use flashy_element::{Element, ElementType};
use futures::future::select_all;
use std::{
    any::Any,
    collections::HashMap,
    future::Future,
    io::{stdout, Write},
    mem,
};
pub use taffy::NodeId;
use taffy::{AvailableSpace, Layout, Size, Style, TaffyTree};

struct InstantiatedComponent {
    node_id: NodeId,
    component: Box<dyn AnyComponent>,
}

pub struct Components {
    components: HashMap<String, InstantiatedComponent>,
}

impl Default for Components {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
}

pub struct ComponentsUpdater<'a> {
    tree_updater: ComponentUpdater<'a>,
    components: &'a mut Components,
    used_components: HashMap<String, InstantiatedComponent>,
}

#[derive(Clone)]
pub struct AnyElement {
    key: String,
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
                    .tree_updater
                    .layout_engine
                    .new_leaf_with_context(Style::default(), LayoutEngineNodeContext::default())
                    .expect("we should be able to add the node");
                self.tree_updater
                    .layout_engine
                    .add_child(self.tree_updater.node_id, new_node_id)
                    .expect("we should be able to add the child");
                InstantiatedComponent {
                    node_id: new_node_id,
                    component: e.props.into_new_component(),
                }
            }
        };
        component.component.update(ComponentUpdater {
            node_id: component.node_id,
            layout_engine: self.tree_updater.layout_engine,
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
            self.tree_updater
                .layout_engine
                .remove(component.node_id)
                .expect("we should be able to remove the node");
        }
        mem::swap(&mut self.components.components, &mut self.used_components);
    }
}

impl Components {
    pub fn updater<'a>(&'a mut self, tree_updater: ComponentUpdater<'a>) -> ComponentsUpdater<'a> {
        ComponentsUpdater {
            tree_updater,
            used_components: HashMap::with_capacity(self.components.len()),
            components: self,
        }
    }

    pub fn render(&self, tree_renderer: ComponentRenderer<'_>) {
        for (_, component) in self.components.iter() {
            component.component.render(ComponentRenderer {
                node_id: component.node_id,
                layout_engine: tree_renderer.layout_engine,
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

pub struct ComponentRenderer<'a> {
    node_id: NodeId,
    layout_engine: &'a TaffyTree<LayoutEngineNodeContext>,
}

impl<'a> ComponentRenderer<'a> {
    pub fn layout(&self) -> &Layout {
        self.layout_engine
            .layout(self.node_id)
            .expect("we should be able to get the layout")
    }

    pub fn queue(&self, command: impl Command) {
        stdout()
            .queue(command)
            .expect("we should be able to queue the command");
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

        self.layout_engine
            .compute_layout_with_measure(
                self.root_node_id,
                Size::max_content(),
                |known_dimensions, available_space, _node_id, node_context, style| {
                    match node_context.and_then(|cx| cx.measure_func.as_ref()) {
                        Some(f) => f(known_dimensions, available_space, style),
                        None => Size::ZERO,
                    }
                },
            )
            .expect("we should be able to compute the layout");

        self.root_component.render(ComponentRenderer {
            node_id: self.root_node_id,
            layout_engine: &self.layout_engine,
        });
    }

    async fn render_loop(&mut self) -> ! {
        let mut dest = stdout();
        queue!(dest, cursor::SavePosition, cursor::Hide)
            .expect("we should be able to queue commands");
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
