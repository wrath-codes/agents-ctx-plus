/// A documented async function with generics.
/// Second line of docs.
pub async fn process<T: Clone>(items: Vec<T>, count: usize) -> Result<Vec<T>, MyError> {
    todo!()
}

unsafe fn dangerous() {
    // unsafe operations here
}

/// A function with lifetimes and where clause.
///
/// # Errors
/// Returns an error if the input is invalid.
pub fn transform<'a, 'b, T>(input: &'a T, output: &'b mut T) -> Result<(), MyError>
where
    T: Clone + Send,
{
    todo!()
}

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub count: usize,
    enabled: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Parse error at line {line}")]
    Parse { line: u32 },
    #[error("Not found")]
    NotFound,
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

impl From<std::io::Error> for MyError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<String> for MyError {
    fn from(msg: String) -> Self {
        Self::Parse { line: 0 }
    }
}

pub const MAX_SIZE: usize = 1024;

pub static GLOBAL_NAME: &str = "zenith";

pub type MyResult<T> = std::result::Result<T, MyError>;

mod internal {
    pub fn helper() -> bool {
        true
    }
}

macro_rules! make_getter {
    ($name:ident, $ty:ty) => {
        pub fn $name(&self) -> &$ty {
            &self.$name
        }
    };
}

union RawValue {
    int_val: i32,
    float_val: f32,
}

#[pyfunction]
pub fn py_add(a: i32, b: i32) -> i32 {
    a + b
}
