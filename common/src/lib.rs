#[tarpc::service]
pub trait PackRat {
    /// Returns a greeting for name.
    async fn hello(name: String) -> String;
}
