use std::net::SocketAddr;

use clap::Parser;
use phnxbackend::{ds::Ds, infra_service::InfraService};
use phnxtypes::identifiers::Fqdn;
use protos::{
    auth_service::v1::auth_service_server::AuthServiceServer,
    delivery_service::v1::delivery_service_server::DeliveryServiceServer,
    queue_service::v1::queue_service_server::QueueServiceServer,
};

use auth_service::AuthService;
use queue_service::QueueService;
use sqlx::PgPool;
use tower_http::trace::TraceLayer;
use url::Url;

mod auth_service;
mod queue_service;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:50051")]
    listen: SocketAddr,

    #[arg(long, default_value = "localhost")]
    domain: Fqdn,

    #[arg(long, default_value = "postgres://postgres:postgres@localhost:5432")]
    database: Url,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let auth_service = AuthService::new();
    let queue_service = QueueService::new();

    let pool = PgPool::connect(args.database.as_str()).await?;
    let delivery_service = Ds::new_from_pool(pool, args.domain).await?;

    tonic::transport::Server::builder()
        .layer(TraceLayer::new_for_grpc())
        .add_service(AuthServiceServer::new(auth_service))
        .add_service(QueueServiceServer::new(queue_service))
        .add_service(DeliveryServiceServer::new(delivery_service))
        .serve("0.0.0.0:50051".parse()?)
        .await?;

    Ok(())
}
