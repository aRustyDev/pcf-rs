pub mod database;
pub mod spicedb;

// Import the main types we need while avoiding conflicts
// Note: Users should prefer direct module access for clarity
pub use database::{DatabaseHealth, VersionCompatibility, DatabaseError};
pub use spicedb::{SpiceDBClient, SpiceDBClientTrait, SpiceDBConfig, SpiceDBError};

// Re-export health modules with prefixes to avoid conflicts
pub use database::health as database_health;
pub use spicedb::health as spicedb_health;