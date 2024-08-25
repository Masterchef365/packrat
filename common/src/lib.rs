use serde::{de::DeserializeOwned, Serialize};

#[tarpc::service]
pub trait PackRat {
    /// Returns a greeting for name.
    async fn hello(name: String) -> String;
}

pub fn encode<T: Serialize>(value: &T) -> bincode::Result<Vec<u8>> {
    bincode::serialize(value)
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> bincode::Result<T> {
    bincode::deserialize(bytes)
}
