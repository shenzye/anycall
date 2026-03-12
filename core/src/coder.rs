use serde::de::DeserializeOwned;
use serde::Serialize;

pub trait Coder: Default {
    type SerErr;
    type DesErr;
    fn encode(&self, input: impl Serialize) -> Result<Vec<u8>, Self::SerErr>;
    fn decode<O: DeserializeOwned>(&self, input: impl AsRef<[u8]>) -> Result<O, Self::DesErr>;
}

#[derive(Default, Clone, Copy)]
pub struct JsonCoder;

impl Coder for JsonCoder {
    type SerErr = serde_json::Error;
    type DesErr = serde_json::Error;
    fn encode(&self, input: impl Serialize) -> Result<Vec<u8>, Self::SerErr> {
        serde_json::to_vec(&input)
    }
    fn decode<O: DeserializeOwned>(&self, input: impl AsRef<[u8]>) -> Result<O, Self::DesErr> {
        serde_json::from_slice(input.as_ref())
    }
}

#[derive(Default, Clone, Copy)]
pub struct CborCoder;

impl Coder for CborCoder {
    type SerErr = ciborium::ser::Error<std::io::Error>;
    type DesErr = ciborium::de::Error<std::io::Error>;
    fn encode(&self, input: impl Serialize) -> Result<Vec<u8>, Self::SerErr> {
        let mut result = Vec::new();
        ciborium::into_writer(&input, &mut result)?;
        Ok(result)
    }
    fn decode<O: DeserializeOwned>(&self, input: impl AsRef<[u8]>) -> Result<O, Self::DesErr> {
        Ok(ciborium::from_reader(input.as_ref())?)
    }
}
