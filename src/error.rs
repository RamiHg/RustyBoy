use core::fmt;

pub enum Type {
    InvalidOperation(String),
}

pub type Result<T> = core::result::Result<T, Type>;

impl fmt::Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { <Type as fmt::Display>::fmt(self, f) }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
        }
    }
}
