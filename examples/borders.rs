use flashy_cli::prelude::*;

struct BordersProps {}

impl ComponentProps for BordersProps {
    type Component = Borders;
}

struct Borders {
    node_id: NodeId,
    children: Components,
}

impl Component for Borders {
    type Props = BordersProps;
    type State = ();

    fn new(node_id: NodeId, _props: Self::Props) -> Self {
        Self {
            node_id,
            children: Components::default(),
        }
    }

    fn set_props(&mut self, _props: Self::Props) {}

    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn update(&mut self, updater: ComponentUpdater<'_>) {
        let mut updater = self.children.updater(updater);
        updater.update(ComponentDeclaration::new(
            "wrapper",
            BoxProps {
                children: vec![ComponentDeclaration::new(
                    "text",
                    TextProps {
                        value: "hi!".to_string(),
                    },
                )],
            },
        ));
    }

    fn render(&self, renderer: ComponentRenderer<'_>) {
        self.children.render(renderer)
    }
}

fn main() {
    render_static(BordersProps {});
}
