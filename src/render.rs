use crossterm::{cursor, queue, Command, QueueableCommand};
use futures::future::{select_all, BoxFuture, FutureExt};
use std::io::{stdout, Write};
pub use taffy::NodeId;
use taffy::{AvailableSpace, Layout, Size, Style, TaffyTree};

pub struct Components<C> {
    components: Vec<C>,
}

impl<C> Default for Components<C> {
    fn default() -> Self {
        Self {
            components: Vec::new(),
        }
    }
}

pub struct ComponentsUpdater<'a, C: Component> {
    tree_updater: TreeUpdater<'a>,
    components: &'a mut Components<C>,
    next_index: usize,
}

impl<'a, C: Component> ComponentsUpdater<'a, C> {
    pub fn update(&mut self, props: C::Props) {
        if self.components.components.len() > self.next_index {
            self.components.components[self.next_index].set_props(props);
        } else {
            let new_node_id = self
                .tree_updater
                .layout_engine
                .new_leaf_with_context(Style::default(), LayoutEngineNodeContext::default())
                .expect("we should be able to add the node");
            self.tree_updater
                .layout_engine
                .add_child(self.tree_updater.node_id, new_node_id)
                .expect("we should be able to add the child");
            let component = C::new(new_node_id, props);
            self.components.components.push(component);
        }
        self.next_index += 1;
        let component = &mut self.components.components[self.next_index - 1];
        component.update(TreeUpdater {
            node_id: component.node_id(),
            layout_engine: self.tree_updater.layout_engine,
        });
    }
}

impl<'a, C: Component> Drop for ComponentsUpdater<'a, C> {
    fn drop(&mut self) {
        for component in self.components.components.drain(self.next_index..) {
            self.tree_updater
                .layout_engine
                .remove(component.node_id())
                .expect("we should be able to remove the node");
        }
    }
}

impl<C: Component> Components<C> {
    pub fn updater<'a>(&'a mut self, tree_updater: TreeUpdater<'a>) -> ComponentsUpdater<'a, C> {
        ComponentsUpdater {
            tree_updater,
            components: self,
            next_index: 0,
        }
    }

    pub fn render(&self, tree_renderer: TreeRenderer<'_>) {
        for component in self.components.iter() {
            component.render(TreeRenderer {
                node_id: component.node_id(),
                layout_engine: tree_renderer.layout_engine,
            });
        }
    }

    pub async fn wait(&mut self) {
        select_all(self.components.iter_mut().map(|component| component.wait())).await;
    }
}

pub struct TreeUpdater<'a> {
    node_id: NodeId,
    layout_engine: &'a mut TaffyTree<LayoutEngineNodeContext>,
}

impl<'a> TreeUpdater<'a> {
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

pub struct TreeRenderer<'a> {
    node_id: NodeId,
    layout_engine: &'a TaffyTree<LayoutEngineNodeContext>,
}

impl<'a> TreeRenderer<'a> {
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

pub trait Renderable {
    fn update(&mut self, updater: TreeUpdater<'_>);
    fn render(&self, renderer: TreeRenderer<'_>);
    fn wait(&mut self) -> BoxFuture<()> {
        std::future::pending::<()>().boxed()
    }
}

pub trait Component: Renderable {
    type Props;
    type State;

    fn new(node_id: NodeId, props: Self::Props) -> Self;
    fn set_props(&mut self, props: Self::Props);
    fn node_id(&self) -> NodeId;
}

type MeasureFunc = Box<dyn Fn(Size<Option<f32>>, Size<AvailableSpace>, &Style) -> Size<f32>>;

#[derive(Default)]
struct LayoutEngineNodeContext {
    measure_func: Option<MeasureFunc>,
}

pub struct Tree {
    layout_engine: TaffyTree<LayoutEngineNodeContext>,
    root_component: Box<dyn Renderable>,
    root_node_id: NodeId,
}

impl Tree {
    fn new<C: Component + 'static>(props: C::Props) -> Self {
        let mut layout_engine = TaffyTree::new();
        let root_node_id = layout_engine
            .new_leaf_with_context(Style::default(), LayoutEngineNodeContext::default())
            .expect("we should be able to add the root");
        let component = C::new(root_node_id, props);
        Self {
            layout_engine,
            root_component: Box::new(component),
            root_node_id,
        }
    }

    async fn render_loop(&mut self) -> ! {
        loop {
            self.root_component.update(TreeUpdater {
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

            let mut dest = stdout();
            queue!(dest, cursor::SavePosition, cursor::Hide)
                .expect("we should be able to queue commands");
            self.root_component.render(TreeRenderer {
                node_id: self.root_node_id,
                layout_engine: &self.layout_engine,
            });
            dest.queue(cursor::RestorePosition)
                .expect("we should be able to queue commands");
            dest.flush().expect("we should be able to flush the output");

            self.root_component.wait().await;
        }
    }
}

pub async fn render<C: Component + 'static>(props: C::Props) {
    let mut tree = Tree::new::<C>(props);
    tree.render_loop().await;
}
