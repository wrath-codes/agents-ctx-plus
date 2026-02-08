/// A documented async function with generics.
/// Second line of docs.
pub async fn process<T: Clone>(items: Vec<T>, count: usize) -> Result<Vec<T>, MyError> {
    todo!()
}

unsafe fn dangerous() {
    // unsafe operations here
}

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub count: usize,
    enabled: bool,
}

#[derive(Debug)]
pub enum Status {
    Active,
    Inactive(String),
    Error { code: u32 },
}

pub trait Handler {
    type Output;
    fn handle(&self) -> Self::Output;
}

impl Handler for Config {
    type Output = String;
    fn handle(&self) -> Self::Output {
        self.name.clone()
    }
}

impl Config {
    pub fn new(name: String) -> Self {
        Self { name, count: 0, enabled: true }
    }
}

pub const MAX_SIZE: usize = 1024;

pub type MyResult<T> = std::result::Result<T, MyError>;
