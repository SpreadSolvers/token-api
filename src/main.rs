use std::{env, time::Duration};

use actix_cors::Cors;
use actix_web::{App, HttpServer, web::Data};
use dotenv::dotenv;
use jsonrpc_v2::Server;
use log::info;

use token_api::{
    handlers::{get_evm_token_metadata, get_evm_token_metadata_with_rpc_url, hello_world},
    repositories::sqlite::evm_token::SqliteEvmTokenRepository,
    services::{chainlist::ChainlistService, evm::EvmTokenService, provider::ProviderService},
};

const CHAINLIST_TTL: Duration = Duration::from_hours(24);
/// How long to reuse the same Fallback [`RpcClient`] (keeps Alloy transport rankings; refresh picks up new Chainlist URLs).
const PROVIDER_CACHE_TTL: Duration = Duration::from_secs(15 * 60);

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    dotenv().ok();

    info!("Hello, world full of tokens!");

    let port = env::var("PORT")
        .expect("PORT must be set")
        .parse::<u16>()
        .expect("PORT must be a number");

    let host = env::var("HOST").expect("HOST must be set");

    let workers = env::var("WORKERS")
        .expect("WORKERS must be set")
        .parse::<usize>()
        .expect("WORKERS must be a number");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let dev_cors = matches!(
        env::var("APP_ENV").as_deref(),
        Ok("development") | Ok("dev")
    );
    if dev_cors {
        info!("APP_ENV=development: permissive CORS enabled");
    }

    let evm_token_repository = SqliteEvmTokenRepository::new(database_url);

    let evm_token_service = EvmTokenService::new(evm_token_repository);

    let chainlist_service = ChainlistService::new(CHAINLIST_TTL);
    let provider_service = ProviderService::new(chainlist_service.clone(), PROVIDER_CACHE_TTL);

    let rpc = Server::new()
        .with_data(jsonrpc_v2::Data::new(evm_token_service.clone()))
        .with_data(jsonrpc_v2::Data::new(provider_service.clone()))
        .with_method(
            "eth_getTokenMetadataWithRpc",
            get_evm_token_metadata_with_rpc_url,
        )
        .with_method("eth_getTokenMetadata", get_evm_token_metadata)
        .finish();

    info!("Starting server on port {}", port);

    HttpServer::new(move || {
        let rpc = rpc.clone();
        let cors = if dev_cors {
            Cors::permissive()
        } else {
            Cors::default()
        };
        App::new()
            .wrap(cors)
            .app_data(Data::new(evm_token_service.clone()))
            .service(hello_world)
            .service(
                actix_web::web::service("/rpc")
                    .guard(actix_web::guard::Post())
                    .finish(rpc.into_web_service()),
            )
    })
    .bind((host, port))?
    .workers(workers)
    .run()
    .await
}
