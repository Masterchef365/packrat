use std::{sync::Arc, time::Duration};

use anyhow::Result;
use common::{BackendWorkerStatus, PackRatFrontend, WorkerSummary};
use database::PackRatDatabase;
use framework::{
    futures::StreamExt,
    tarpc::server::{BaseChannel, Channel},
    ServerFramework,
};
use tokio::sync::Mutex as TokioMutex;

mod database;

#[tokio::main]
async fn main() -> Result<()> {
    let db = PackRatDatabase::new("data/".into())?;
    let db = Arc::new(TokioMutex::new(db));

    tokio::spawn(database::autosave(db.clone(), Duration::from_secs(60*60)));

    let endpoint = quic_session::server_endpoint(
        "0.0.0.0:9090".parse().unwrap(),
        include_bytes!("localhost.crt").to_vec(),
        include_bytes!("localhost.key").to_vec(),
    )
    .await?;

    while let Some(inc) = endpoint.accept().await {
        println!("new connection");
        let db = db.clone();
        tokio::spawn(async move {
            let sess = quic_session::server_connect(inc).await?;

            // Spawn the root service
            let (framework, channel) = ServerFramework::new(sess).await?;
            let transport = BaseChannel::with_defaults(channel);

            let server = PackRatFrontendServer { framework, db };
            let executor = transport.execute(PackRatFrontend::serve(server));

            tokio::spawn(executor.for_each(|response| async move {
                tokio::spawn(response);
            }));

            println!("connection ended");
            Ok::<_, anyhow::Error>(())
        });
    }

    Ok(())
}

#[derive(Clone)]
struct PackRatFrontendServer {
    framework: ServerFramework,
    db: Arc<TokioMutex<PackRatDatabase>>,
}

impl PackRatFrontend for PackRatFrontendServer {
    async fn get_archive(
        self,
        context: framework::tarpc::context::Context,
        page: usize,
        num_per_page: usize,
    ) -> Vec<common::Job> {
        todo!()
    }

    async fn get_running_and_queued_jobs(
        self,
        context: framework::tarpc::context::Context,
    ) -> Vec<common::Job> {
        todo!()
    }

    async fn get_worker_events(
        self,
        context: framework::tarpc::context::Context,
    ) -> framework::BiStream<(String, common::FrontendWorkerStatusUpdate), ()> {
        todo!()
    }

    async fn create_account(
        self,
        context: framework::tarpc::context::Context,
        email: String,
        name: String,
    ) -> () {
        todo!()
    }

    async fn get_workers(
        self,
        context: framework::tarpc::context::Context,
    ) -> std::collections::HashMap<String, common::WorkerSummary> {
        [
            (
                "101SIP02".to_string(),
                WorkerSummary {
                    address: "127.0.0.1".to_string(),
                    data: common::FrontendWorkerStatusUpdate::Online(BackendWorkerStatus::Ready),
                },
            ),
            (
                "101SIP00".to_string(),
                WorkerSummary {
                    address: "127.0.0.1".to_string(),
                    data: common::FrontendWorkerStatusUpdate::Online(
                        BackendWorkerStatus::Replaying(common::ReplayStatus::Setup {
                            message: "Downloading GeoContour".to_string(),
                        }),
                    ),
                },
            ),
        ]
        .into_iter()
        .collect()
    }

    async fn login(
        self,
        context: framework::tarpc::context::Context,
        email: String,
    ) -> Option<framework::Subservice<common::PackRatFrontendLoggedInClient>> {
        todo!()
    }

    /*
    async fn add(self, _context: framework::tarpc::context::Context, a: u32, b: u32) -> u32 {
        a + b
    }

    async fn get_sub(
        self,
        context: framework::tarpc::context::Context,
    ) -> framework::Subservice<common::MyOtherServiceClient> {
        println!("Getting sub, accepting");
        let (token, channelfuture) = self.framework.accept_subservice();
        println!("Accepted");

        tokio::spawn(async move {
            let transport = BaseChannel::with_defaults(channelfuture.await?);

            let server = MyOtherServiceServer;
            let executor = transport.execute(MyOtherService::serve(server));

            tokio::spawn(executor.for_each(|response| async move {
                tokio::spawn(response);
            }));

            Ok::<_, anyhow::Error>(())
        });

        token
    }
    */
}
