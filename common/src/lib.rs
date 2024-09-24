use framework::Subservice;

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

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ReplayStateData {
    #[default]
    Ready,
    Running {
        board_count: usize,
        current_board_index: usize,
    },
    Finished,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Replay {
    pub xml: ReplayPack,
    pub state: ReplayStateData,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Job {
    pub name: String,
    pub description: String,
    pub replays: Vec<ReplayPack>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum IpStatusState {
    #[default]
    Ready,
    Running {
        job: Job,
        replay: Replay,
    },
    Error {
        timeout_in: usize,
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct IpStatus {
    pub name: String,
    pub address: String,
    pub state: IpStatusState,
    pub lockout_username: Option<String>,
    pub last_replay_parameters: Replay,
}

#[tarpc::service]
pub trait PackRat {
    /// API interface for workers
    async fn worker_login(designation: String) -> Option<Subservice<PackRatWorkerClient>>;

    /// Returns the user's name, and 
    async fn frontend_login(email: String) -> Option<(String, Subservice<PackRatFrontendClient>)>;

    /// Creates a new user account with the given email and name
    async fn create_account(email: String, name: String);

}

#[tarpc::service]
pub trait PackRatWorker {
    async fn get_replay() -> Replay;

    async fn get_abort() -> bool;
}


#[tarpc::service]
pub trait PackRatFrontend {
    // Home page

    /// Returns jobs which should appear on the homepage
    async fn get_running_and_queued_jobs() -> Vec<Job>;
    async fn get_ip_farm_status() -> Vec<IpStatus>;

    // Login

    /// Changes the username of this account
    async fn change_name(new_name: String);
}
