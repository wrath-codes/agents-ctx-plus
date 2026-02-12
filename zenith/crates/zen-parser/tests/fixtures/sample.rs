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

// ── Extended fixture: additional Rust constructs ───────────────────

// 1. const fn
pub const fn const_add(a: u32, b: u32) -> u32 {
    a + b
}

// 2. extern "C" fn
pub extern "C" fn c_callback(x: i32) -> i32 {
    x
}

// 3. unsafe trait
pub unsafe trait ThreadSafe {
    fn verify(&self) -> bool;
}

// 4. unsafe impl
unsafe impl ThreadSafe for Config {
    fn verify(&self) -> bool {
        true
    }
}

// 5. Trait with supertraits
pub trait Validator: Clone + Send {
    fn validate(&self) -> bool;
}

// 6. Trait with constants, associated types, and default method
pub trait Configurable {
    const MAX_ITEMS: usize;
    const DEFAULT_NAME: &'static str;
    type Item;
    fn configure(&mut self);
    fn name(&self) -> &str {
        "default"
    }
}

// 7. GATs (Generic Associated Types)
pub trait Lending {
    type Item<'a> where Self: 'a;
    fn lend(&self) -> Self::Item<'_>;
}

// 8. Tuple struct
pub struct Point(pub f64, pub f64);

// 9. Unit struct
pub struct Marker;

// 10. Enum with documented payloads
pub enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    Color(u8, u8, u8),
}

// 11. #[repr(C)] struct
#[repr(C)]
pub struct FfiPoint {
    pub x: f64,
    pub y: f64,
}

// 12. Const generics struct
pub struct Buffer<const N: usize> {
    data: [u8; N],
}

// 13. extern block (foreign_mod_item)
extern "C" {
    fn external_func(x: i32) -> i32;
    static EXTERNAL_VAR: i32;
}

// 14. pub use re-export
pub use std::collections::HashMap;

// 15. extern crate
extern crate alloc;

// 16. Item-position macro invocation
thread_local! {
    static LOCAL_VALUE: std::cell::RefCell<u32> = std::cell::RefCell::new(0);
}

// 17. cfg-gated function
#[cfg(feature = "serde")]
pub fn serde_only() -> bool {
    true
}

// 18. deprecated function
#[deprecated(since = "1.0.0", note = "use new_api instead")]
pub fn old_api() {}

// 19. must_use function
#[must_use]
pub fn important_result() -> bool {
    true
}

// 20. doc(hidden) function
#[doc(hidden)]
pub fn internal_only() {}

// 21. Block doc comment function
/** Block documented function.
 *
 * With multiple lines.
 */
pub fn block_documented() -> bool {
    true
}

// 22. impl Trait return
pub fn make_iterator() -> impl Iterator<Item = u32> {
    vec![1u32, 2, 3].into_iter()
}

// 23. dyn Trait parameter
pub fn process_dyn(handler: &dyn std::fmt::Debug) -> String {
    format!("{:?}", handler)
}

// 24. HRTB (Higher-Ranked Trait Bounds)
pub fn apply_fn<F>(f: F) -> i32
where
    F: for<'a> Fn(&'a i32) -> i32,
{
    f(&42)
}

// 25. pub(super) visibility
pub(super) fn super_visible() {}

// 26. pub(in path) visibility
pub(in crate::internal) fn path_visible() -> bool {
    true
}

// 27. Impl with associated consts
impl Config {
    pub const DEFAULT_COUNT: usize = 10;
    pub const VERSION: &'static str = "1.0";
}

// 28. Negative impl
impl !Send for RawValue {}

// 29. Self receiver forms
pub struct Receiver {
    value: i32,
}

impl Receiver {
    pub fn take(self) -> i32 {
        self.value
    }
    pub fn borrow(&self) -> i32 {
        self.value
    }
    pub fn mutate(&mut self) {
        self.value += 1;
    }
}

// 30. #[macro_export] macro
#[macro_export]
macro_rules! exported_macro {
    ($val:expr) => {
        $val + 1
    };
}

// 31. Impl-level associated type (via trait impl)
impl Configurable for Receiver {
    const MAX_ITEMS: usize = 100;
    const DEFAULT_NAME: &'static str = "receiver";
    type Item = i32;
    fn configure(&mut self) {
        self.value = 0;
    }
}

// 32. impl for reference type
impl std::fmt::Display for &RawValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RawValue")
    }
}
