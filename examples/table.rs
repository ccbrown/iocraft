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

#[derive(Clone, Default)]
struct UsersTableProps {
    users: Vec<User>,
}

#[component]
fn UsersTable(props: &UsersTableProps) -> impl Into<AnyElement> {
    element! {
        Box(flex_direction: FlexDirection::Column, width: 60, border_style: BorderStyle::Round) {
            Box {
                Box(width: 10pct) {
                    Text(content: "Id")
                }

                Box(width: 40pct) {
                    Text(content: "Name")
                }

                Box(width: 50pct) {
                    Text(content: "Email")
                }
            }

            {props.users.iter().map(|user| element! {
                Box {
                    Box(width: 10pct) {
                        Text(content: user.id.to_string())
                    }

                    Box(width: 40pct) {
                        Text(content: user.name.clone())
                    }

                    Box(width: 50pct) {
                        Text(content: user.email.clone())
                    }
                }
            })}
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

    element!(UsersTable(users: users)).print();
}
