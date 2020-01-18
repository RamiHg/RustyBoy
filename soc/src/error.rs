#[derive(Debug)]
pub enum Type {
    InvalidOperation(String),
    InvalidAddress(i32),
    TODOMemoryBus,
}

pub type Result<T> = core::result::Result<T, Type>;
