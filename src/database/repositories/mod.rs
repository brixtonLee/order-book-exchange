/// Repository pattern implementations adhering to SOLID principles
///
/// - **Single Responsibility**: Each repository handles one entity type
/// - **Open/Closed**: Easy to extend with new implementations
/// - **Liskov Substitution**: Traits define contracts
/// - **Interface Segregation**: Focused repository interfaces
/// - **Dependency Inversion**: Depend on traits, not concrete types

pub mod ohlc_repository;
pub mod symbol_repository;
pub mod tick_repository;

pub use ohlc_repository::{OhlcRepository, OhlcRepositoryImpl};
pub use symbol_repository::{SymbolRepository, SymbolRepositoryImpl};
pub use tick_repository::{TickRepository, TickRepositoryImpl};
