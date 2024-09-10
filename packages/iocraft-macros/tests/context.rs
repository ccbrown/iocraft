use iocraft::context;

#[allow(dead_code)]
#[derive(Default)]
struct NumberOfTheDay(i32);

#[allow(dead_code)]
#[context]
pub struct MyContext<'a> {
    optional_number: Option<&'a NumberOfTheDay>,
    number: &'a NumberOfTheDay,
}
