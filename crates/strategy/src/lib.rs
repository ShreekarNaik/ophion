pub mod noop;
pub mod taker;
pub mod traits;

pub use noop::NoopStrategy;
pub use taker::TakerStrategy;
pub use traits::{Action, Strategy};
