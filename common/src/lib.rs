//use framework::Subservice;

/// TLS certificate (self-signed for debug purposes)
pub const CERTIFICATE: &[u8] = include_bytes!("localhost.crt");

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReplayPack {
    pub name: String,
    pub path: String,
    pub project: String,
    pub ip_config: String,
    pub ip_version_manifest: String,
    pub ps_key: String,
    pub ps_tag: String,
    pub generate_training_files: bool,
    pub zip_training_tiffs: bool,
    pub skip_required_run_check: bool,
    /// Inserted as-is into the XML
    pub custom_keys: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ReplayState {
    Ready,
    Running,
    Finished,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Replay {
    pub xml: ReplayPack,
    pub state: ReplayState,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Job {
    pub name: String,
    pub description: String,
    pub replays: Vec<ReplayPack>,
}

#[tarpc::service]
pub trait PackRat {
    /// Returns jobs which should appear on the homepage
    async fn get_running_and_queued_jobs() -> Vec<Job>;

    async fn login(email: String) -> Option<usize>;
}
