use iocraft::context;

#[allow(dead_code)]
#[derive(Default)]
struct NumberOfTheDay(i32);

#[allow(dead_code)]
#[context]
pub struct MyBasicContext<'a> {
    number: Option<&'a NumberOfTheDay>,
}
