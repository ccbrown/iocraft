use std::io;

use iocraft::prelude::*;
use smol::process::{Command, Stdio};
use which::which;

#[derive(Clone, Debug)]
struct ManPage {
    key: String,
    title: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
enum ManLayout {
    #[default]
    Vertical,
    Horizontal,
}

fn parse_man_output(output: &str) -> Vec<ManPage> {
    let mut man_pages = Vec::new();

    for line in output.lines().map(|x| x.trim()).filter(|x| !x.is_empty()) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        // The key is the first part (e.g., "arandr")
        //let key = parts[0].trim_end_matches('(').to_string();
        let key = parts[0].split('(').next().unwrap().to_string();

        // The title is the rest of the line after the key and section (e.g., "visual front end for XRandR 1.2")
        let title_start = line.find("  - ").map_or(line.len(), |pos| pos + 3);
        let title = line[title_start..].trim().to_string();

        if title.is_empty() {
            continue;
        }

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
        elms.push(
            MixedTextContent::new(&key[last + pos..last + pos + query.len()])
                .color(Color::Red)
                .weight(Weight::Bold),
        );

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
    let output = std::process::Command::new("man")
        .args(["-k", ".", "-s", "1"])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    let output_str = String::from_utf8(output.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(parse_man_output(&output_str))
}

#[derive(Props, Default)]
struct PromptProps {
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
        View(flex_direction: FlexDirection::Column, border_style: BorderStyle::Round, height: 3) {
            View(height: 1, margin_top: -1, justify_content: JustifyContent::Center) {
                Text(content: "Prompt")
            }
            View(flex_direction: FlexDirection::Row) {
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

fn find_sgr<I: Iterator<Item = char>>(it: &mut std::iter::Peekable<I>, e: char) -> Option<usize> {
    let Some(c) = it.peek() else {
        return None;
    };

    if *c != '\u{1b}' {
        return None;
    }

    it.next().unwrap();

    let Some(c) = it.next() else {
        return None;
    };

    if c != '[' {
        return None;
    };

    if it.peek().map(|x| *x == e).unwrap_or(true) {
        return Some(0);
    }

    let digit = it.take_while(|x| *x != e).collect::<String>();

    return Some(digit.parse().unwrap());
}

fn escape_chars_to_styling(content: &str) -> Vec<MixedTextContent> {
    let (mut text, mut bold, mut underline) = (String::new(), false, false);
    let mut elms = Vec::new();
    let mut push = |text, bold, underline| {
        elms.push(
            MixedTextContent::new(text)
                .weight(if bold { Weight::Bold } else { Weight::Normal })
                .decoration(if underline {
                    TextDecoration::Underline
                } else {
                    TextDecoration::None
                }),
        );
    };

    let mut it = content.chars().take(4096).peekable();
    loop {
        let mut is_text = false;
        let sgr = find_sgr(&mut it, 'm');

        match sgr {
            Some(x) => {
                push(text, bold, underline);
                text = String::new();

                match x {
                    0 | 22 => {
                        bold = false;
                        underline = false;
                    }
                    1 => bold = true,
                    4 => underline = true,
                    24 => underline = false,
                    _ => {
                        //dbg!(x);
                        //panic!("");
                    }
                }
            }
            None => is_text = true,
        };

        if is_text {
            if let Some(c) = it.next() {
                text.push(c);
            } else {
                break;
            }
        }
    }

    if text.len() > 0 {
        push(text, bold, underline);
    }

    elms
}

#[derive(Props, Default)]
struct PreviewProps {
    current: String,
}

#[component]
fn Preview<'a>(props: &'a PreviewProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut contents = hooks.use_state_default();
    let width = hooks
        .use_component_rect()
        .get()
        .map(|rect| rect.right - rect.left - 2);

    let update_page = hooks.use_async_handler(move |current: String| async move {
        // do not render if width is not known yet
        let Some(width) = width else {
            return;
        };

        let res = Command::new("man")
            .args(&[&current.to_string()])
            .env("MANWIDTH", width.to_string())
            .env("MAN_KEEP_FORMATTING", "1")
            .env("GROFF_SGR", "1")
            .stdout(Stdio::piped())
            .output()
            .await
            .unwrap();

        contents.set(escape_chars_to_styling(
            str::from_utf8(&res.stdout).unwrap(),
        ));
    });

    // update content when page key or width changed
    hooks.use_memo(
        || update_page(props.current.clone()),
        (&props.current, width),
    );

    element! {
        View(flex_grow: 1.0, flex_direction: FlexDirection::Column, border_style: BorderStyle::Round) {
            View(height: 1, margin_top: -1, justify_content: JustifyContent::Center) {
                Text(content: "Preview")
            }
            View(height: 100pct,  overflow: Some(Overflow::Hidden)) {
                MixedText(contents: contents.read().clone())
            }
        }
    }
}

#[derive(Props, Default)]
struct ResultsProps {
    elms: Vec<(String, Vec<MixedTextContent>)>,
    current_idx: Option<State<usize>>,
}

#[component]
fn Results<'a>(props: &'a ResultsProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let Some(mut current_idx) = props.current_idx else {
        panic!("value is required");
    };

    let (width, height) = match hooks.use_component_rect().get() {
        Some(rect) => (rect.right - rect.left - 2, rect.bottom - rect.top - 2),
        _ => (30, 20),
    };

    let mut beginning = hooks.use_state(|| 0);

    if beginning > current_idx.get() {
        beginning.set(current_idx.get());
    } else if current_idx.get() > beginning + height as usize - 1 {
        beginning.set(current_idx.get() - height as usize + 1);
    }

    let max_len = hooks.use_memo(
        || props.elms.iter().skip(beginning.get() as usize).take(height as usize).map(|x| x.0.chars().count()).max().unwrap_or(0) as u32,
        (&props.elms.iter().map(|x| x.0.clone()).collect::<String>(), &beginning)
    );

    let max_len = u32::min(max_len, width as u32);
    let header_len = u32::max(width as u32 - max_len - 1, 4);
    let key_len = width as u32 - header_len;

    let nprops = props.elms.len();
    let current_key = match props.elms.len() {
        0 => None,
        _ => {
            if current_idx.get() >= props.elms.len() {
                current_idx.set(props.elms.len() - 1);
            }

            Some(props.elms[current_idx.get()].0.clone())
        }
    };

    //let (stdout, stderr) = hooks.use_output();
    hooks.use_terminal_events({
        move |event| match event {
            TerminalEvent::Key(KeyEvent { code, kind, modifiers, .. }) if kind != KeyEventKind::Release => {
                match code {
                    KeyCode::Up =>
                        current_idx.set((current_idx.get() as isize - 1).rem_euclid(nprops as isize) as usize),
                    KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => current_idx.set((current_idx.get() as isize - 10).rem_euclid(nprops as isize) as usize),
                    KeyCode::Down => current_idx.set((current_idx.get() + 1) % nprops ),
                    KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => current_idx.set((current_idx.get() + 10) % nprops),
                    KeyCode::Enter => {
                        current_key.as_ref().map(|current_key| {
                            let _ = std::process::Command::new("man")
                                .arg(&current_key)
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .unwrap();
                        });
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });

    element! {
        View(flex_grow: 1.0, flex_direction: FlexDirection::Column, border_style: BorderStyle::Round) {
            View(height: 1, margin_top: -1, justify_content: JustifyContent::Center) {
                Text(content: "Results")
            }
            View(flex_direction: FlexDirection::Column, overflow: Some(Overflow::Hidden)) {
                #(props.elms.iter().enumerate().skip(beginning.get() as usize)
                .map(|(idx, mat)| if current_idx.get() == idx {
                    (Color::DarkGrey, mat)
                } else {
                    (Color::Reset, mat)
                })
                .map(|(color, mat)| element! {
                    View(flex_direction: FlexDirection::Row, background_color: Some(color)) {
                        View(width: key_len, height: 1) { Text(content: mat.0.clone(), color: Some(Color::Cyan), weight: Weight::Bold) }
                        View(width: header_len) {MixedText(contents: mat.1.clone(), wrap: TextWrap::NoWrap) }
                    }
                }).take(height as usize))
            }
        }
    }
}

#[derive(Props, Default)]
struct ManPicker;

#[component]
fn Picker<'a>(_props: &'a ManPicker, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // global view of all pages
    let pages = hooks.use_const(|| get_man_pages().unwrap());

    // query of the prompt element and selected element in the results lists
    // are shared between two components
    let prompt: State<String> = hooks.use_state_default();
    let current_idx = hooks.use_state(|| 0usize);

    // layout can be changed during runtime
    let mut layout = hooks.use_state(|| ManLayout::Vertical);

    // cache preview elements based on current prompt
    let elms = hooks.use_memo(
        || {
            pages
                .iter()
                .filter_map(|page| {
                    matches(&page.title, &prompt.read().as_str()).map(|x| (page.key.clone(), x))
                })
                .collect::<Vec<_>>()
        },
        &prompt,
    );

    let nelms = (elms.len(), pages.len());

    let key = hooks.use_memo(|| {
        if current_idx.get() >= elms.len() {
            elms.last().map(|x| x.0.clone()).unwrap_or(String::new())
        } else {
            elms[current_idx.get()].0.clone()
        }
    }, (&current_idx, &prompt));

    hooks.use_terminal_events({
        move |event| match event {
            TerminalEvent::Key(KeyEvent {
                code,
                kind,
                modifiers,
                ..
            }) if kind != KeyEventKind::Release && modifiers.contains(KeyModifiers::ALT) => {
                match code {
                    KeyCode::Char('V') => layout.set(ManLayout::Vertical),
                    KeyCode::Char('H') => layout.set(ManLayout::Horizontal),
                    _ => {}
                }
            }
            _ => {}
        }
    });

    if *layout.read() == ManLayout::Vertical {
        element! {
            View(flex_direction: FlexDirection::Row, width: 100pct) {
                View(flex_direction: FlexDirection::Column, width: 50pct) {
                    Results(elms: elms, current_idx)
                    Prompt(prompt: prompt, show_carrot: true, nelms)
                }
                Preview(current: key)
            }
        }
    } else {
        element! {
            View(flex_direction: FlexDirection::Column, width: 100pct) {
                View(flex_direction: FlexDirection::Column, height: 50pct) {
                    Results(elms: elms, current_idx)
                    Prompt(prompt: prompt, show_carrot: true, nelms)
                }
                View(height: 50pct) {
                    Preview(current: key)
                }
            }
        }
    }
}

fn main() {
    if which("man").is_err() {
        println!("System interface manual `man` not available!");
        return;
    }
    if which("ul").is_err() {
        println!("Formatter `ul` not available!");
        return;
    }

    smol::block_on(
        element! {
            View(width: 160, height: 30) {
                Picker()
            }
        }
        .render_loop(),
    )
    .unwrap();
}
