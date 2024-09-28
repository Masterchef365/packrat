use std::collections::HashMap;

use framework::{Subservice, BiStream};

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
pub struct Job {
    pub name: String,
    pub description: String,
    pub is_archived: bool,
    pub replays: Vec<ReplayPack>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum FrontendWorkerStatusUpdate {
    Disconnected {
        /// When this worker was last online
        last_seen: String,
    },
    Online(BackendWorkerStatus),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkerSummary {
    pub address: String,
    pub data: FrontendWorkerStatusUpdate,
}

#[tarpc::service]
pub trait PackRatFrontend {
    // ---- Home page ----
    /// Returns jobs which should appear on the homepage
    async fn get_running_and_queued_jobs() -> Vec<Job>;
    /// Returns a chunk of archival data
    async fn get_archive(page: usize, num_per_page: usize) -> Vec<Job>;

    // ---- Worker control ----
    /// Returns the list of worker names
    async fn get_workers() -> HashMap<String, WorkerSummary>;
    /// Returns a stream of (worker name, status update)
    async fn get_worker_events() -> BiStream<(String, FrontendWorkerStatusUpdate), ()>;

    // ---- Login ----
    /// Returns the user's name
    async fn login(email: String) -> Option<Subservice<PackRatFrontendLoggedInClient>>;

    /// Creates a new user account with the given email and name
    async fn create_account(email: String, name: String);
}

#[tarpc::service]
pub trait PackRatFrontendLoggedIn {
    /// Changes the username of this account
    async fn change_name(new_name: String);

    /// Creates a proxy connection to the given worker
    async fn control_worker(name: String) -> Option<Subservice<PackRatWorkerClient>>;

    /// Returns un-archived jobs which are betrothed to this account
    async fn my_jobs() -> Vec<Job>;
}

#[tarpc::service]
pub trait PackRatWorker {
    /// Shut down this process on the machine
    async fn take_offline();

    /// If this worker is Ready, then the new replay is started
    async fn start_replay(replay: ReplayPack);

    /// Returns the current replay client
    async fn current_replay() -> Option<Subservice<PackRatWorkerProcessClient>>;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum BackendWorkerStatus {
    Replaying {
        current_board_index: usize,
        total_boards: usize,
    },
    Error {
        mins_to_timeout: u32,
        summary: String,
    },
}

pub enum ReplayStatus {
    Setup {
        message: String,
    },
    Running {
        current_board_index: usize,
        total_boards: usize,
    }
    //FinishingUp,
}

#[tarpc::service]
pub trait PackRatWorkerProcess {
    /// Cause this process to abort
    async fn abort();

    /// Get the parameters of this pack
    async fn parameters() -> ReplayPack;

    /// This method will immediately send the current state, followed by events.
    async fn follow() -> BiStream<BackendWorkerStatus, ()>;
}
