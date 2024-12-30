use iocraft::prelude::*;

#[derive(Clone)]
struct User {
    id: i32,
    name: String,
    email: String,
}

impl User {
    fn new(id: i32, name: &str, email: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            email: email.to_string(),
        }
    }
}

#[derive(Default, Props)]
struct UsersTableProps<'a> {
    users: Option<&'a Vec<User>>,
}

#[component]
fn UsersTable<'a>(props: &UsersTableProps<'a>) -> impl Into<AnyElement<'a>> {
    element! {
        View(
            margin_top: 1,
            margin_bottom: 1,
            flex_direction: FlexDirection::Column,
            width: 60,
            border_style: BorderStyle::Round,
            border_color: Color::Cyan,
        ) {
            View(border_style: BorderStyle::Single, border_edges: Edges::Bottom, border_color: Color::Grey) {
                View(width: 10pct, justify_content: JustifyContent::End, padding_right: 2) {
                    Text(content: "Id", weight: Weight::Bold, decoration: TextDecoration::Underline)
                }

                View(width: 40pct) {
                    Text(content: "Name", weight: Weight::Bold, decoration: TextDecoration::Underline)
                }

                View(width: 50pct) {
                    Text(content: "Email", weight: Weight::Bold, decoration: TextDecoration::Underline)
                }
            }

            #(props.users.map(|users| users.iter().enumerate().map(|(i, user)| element! {
                View(background_color: if i % 2 == 0 { None } else { Some(Color::DarkGrey) }) {
                    View(width: 10pct, justify_content: JustifyContent::End, padding_right: 2) {
                        Text(content: user.id.to_string())
                    }

                    View(width: 40pct) {
                        Text(content: user.name.clone())
                    }

                    View(width: 50pct) {
                        Text(content: user.email.clone())
                    }
                }
            })).into_iter().flatten())
        }
    }
}

fn main() {
    let users = vec![
        User::new(1, "Alice", "alice@example.com"),
        User::new(2, "Bob", "bob@example.com"),
        User::new(3, "Charlie", "charlie@example.com"),
        User::new(4, "David", "david@example.com"),
        User::new(5, "Eve", "eve@example.com"),
        User::new(6, "Frank", "frank@example.com"),
        User::new(7, "Grace", "grace@example.com"),
        User::new(8, "Heidi", "heidi@example.com"),
    ];

    element!(UsersTable(users: &users)).print();
}
