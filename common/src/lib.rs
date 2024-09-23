use framework::Subservice;

/// TLS certificate (self-signed for debug purposes)
pub const CERTIFICATE: &[u8] = include_bytes!("localhost.crt");

#[tarpc::service]
pub trait MyService {
    /// Returns a greeting for name.
    async fn add(a: u32, b: u32) -> u32;

    /// Returns a sub-service
    async fn get_sub() -> Subservice<MyOtherServiceClient>;
}

#[tarpc::service]
pub trait MyOtherService {
    /// Returns a greeting for name.
    async fn subtract(a: u32, b: u32) -> u32;
}
