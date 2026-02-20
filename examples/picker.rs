use iocraft::prelude::*;

use std::process::{Command, Stdio};
use std::io;

#[derive(Clone, Debug)]
struct ManPage {
    key: String,
    title: String,
}

fn parse_man_output(output: &str) -> Vec<ManPage> {
    let mut man_pages = Vec::new();

    for line in output.lines().map(|x| x.trim()).filter(|x| !x.is_empty()) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue; // malformed
        }

        // The key is the first part (e.g., "arandr")
        let key = parts[0].trim_end_matches('(').to_string();

        // The title is the rest of the line after the key and section (e.g., "visual front end for XRandR 1.2")
        let title_start = line.find("  - ").map_or(line.len(), |pos| pos + 3);
        let title = line[title_start..].trim().to_string();

        man_pages.push(ManPage { title, key });
    }

    man_pages
}

fn matches(key: &str, query: &str) -> Option<Vec<MixedTextContent>> {
    if query == "" {
        return Some(vec![MixedTextContent::new(key.to_owned())]);
    }

    let mut elms = vec![];
    let mut last = 0;

    while let Some(pos) = key[last..].find(query) {
        elms.push(MixedTextContent::new(&key[last..last + pos]));
        elms.push(MixedTextContent::new(&key[last + pos..last + pos + query.len()]).color(Color::Red).weight(Weight::Bold));

        last += pos + query.len();
    }
    if last < key.len() {
        elms.push(MixedTextContent::new(&key[last..]));
    }

    if elms.len() > 1 {
        Some(elms)
    } else {
        None
    }
}

fn get_man_pages() -> io::Result<Vec<ManPage>> {
    let output = Command::new("man")
        .args(["-k", ".", "-s", "1"])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Command failed: {}", String::from_utf8_lossy(&output.stderr)),
        ));
    }

    let output_str = String::from_utf8(output.stdout).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, e)
    })?;

    Ok(parse_man_output(&output_str))
}

#[derive(Props, Default)]
struct PromptProps {
    title: String,
    show_carrot: bool,
    prompt: Option<State<String>>,
    nelms: (usize, usize),
}

#[component]
fn Prompt<'a>(props: &'a PromptProps, _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let Some(mut value) = props.prompt else {
        panic!("value is required");
    };

    element! {
        View(border_style: BorderStyle::Round, flex_grow: 1.0, height: 3, border_title: Some(BorderTitle { title: props.title.clone(), pos: BorderTitlePos::Bottom })) {
            Fragment {
                #( if props.show_carrot { Some(
                        element! { Text(content: ">", color: Some(Color::Red)) }) 
                   } else { None }
                )
                View(flex_grow: 1.0, background_color: Color::DarkGrey) {
                    TextInput(has_focus: true, value: value.to_string(), on_change: move |new_value| value.set(new_value))
                }
                Text(content: format!(" {}/{}", props.nelms.0, props.nelms.1), color: Some(Color::DarkGrey))
            }
        }
    }
}

#[derive(Props, Default)]
struct ResultsProps {
    elms: Vec<(String, Vec<MixedTextContent>)>,
}

#[component]
fn Results<'a>(props: &'a ResultsProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let max_len = hooks.use_memo(|| 
        props.elms.iter().map(|x| x.0.len()).max().unwrap_or(0) as u32, 
        &props.elms.iter().map(|x| x.0.clone()).collect::<String>());

    let max_len = u32::min(max_len, 17);
    let mut current_idx = hooks.use_state(|| 0isize);
    let mut beginning = hooks.use_state(|| 0);

    if current_idx.get() < 0 {
        current_idx.set(props.elms.len() as isize - 1);
    } else if current_idx.get() >= props.elms.len() as isize && props.elms.len() > 0 {
        current_idx.set(0);
    }

    if beginning > current_idx.get() {
        beginning.set(current_idx.get());
    } else if current_idx.get() > beginning + 17 {
        beginning.set(current_idx.get() - 17);
    }

    let nprops = props.elms.len();
    let current_key = match props.elms.len() {
        0 => None,
        _ => Some(props.elms[current_idx.get() as usize].0.clone())
    };

    hooks.use_terminal_events({
        move |event| match event {
            TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
                match code {
                    KeyCode::Up => current_idx.set(current_idx.get() - 1),
                    KeyCode::Down => current_idx.set(isize::min(current_idx.get() + 1, nprops as isize)),
                    KeyCode::Enter => {
                        current_key.as_ref().map(|current_key| {
                            let _ = Command::new("man").arg(&current_key)
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output().unwrap();
                        });
                    },
                    _ => {}
                }
            }
            _ => {}
        }
    });

    element! { 
        View(flex_grow: 1.0, flex_direction: FlexDirection::Column, height: 20, border_style: BorderStyle::Round, overflow: Some(Overflow::Hidden), border_title: Some(BorderTitle { title: "Results".to_owned(), pos: BorderTitlePos::Top })) {
            #(props.elms.iter().enumerate().skip(beginning.get() as usize)
            .map(|(idx, mat)| if current_idx.get() as usize == idx {
                (Color::DarkGrey, mat)
            } else {
                (Color::Reset, mat)
            })
            .map(|(color, mat)| element! { 
                View(flex_direction: FlexDirection::Row, background_color: Some(color)) {
                    View(width: max_len + 2, height: 1) { Text(content: mat.0.clone(), color: Some(Color::Cyan), weight: Weight::Bold) }
                    View(width: 80 - max_len - 2) {MixedText(contents: mat.1.clone(), wrap: TextWrap::NoWrap) }
                }
            }).take(18))
        }
    }
}

#[derive(Props, Default)]
struct ManPicker;

#[component]
fn Picker<'a>(_props: &'a ManPicker, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let pages = hooks.use_const(|| get_man_pages().unwrap());
    let prompt: State<String> = hooks.use_state_default();

    let elms = hooks.use_memo(|| 
        pages.iter().filter_map(|page| matches(&page.title, &prompt.read().as_str()).map(|x| (page.key.clone(), x))).collect::<Vec<_>>(), &prompt);

    let nelms = (elms.len(), pages.len());
    element! {
        View(flex_direction: FlexDirection::Column, width: Size::Percent(100.0)) {
            Results(elms: elms)
            Prompt(prompt: prompt, show_carrot: true, nelms, title: "Man Pages".to_owned())
        }
    }
        
}

fn main() {
    smol::block_on(
        element! {
            View(width: Size::Length(80)) {
                Picker()
            }
        }
        .render_loop()
    )
    .unwrap();
}
