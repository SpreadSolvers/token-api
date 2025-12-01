use std::env;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get,
    http::header::ContentType,
    web::{Data, Json, Path},
};
use alloy::primitives::Address;
use dotenv::dotenv;
use log::{error, info};
use serde::Deserialize;

use token_api::{
    repositories::sqlite::evm_token::SqliteEvmTokenRepository,
    services::evm::{EvmTokenService, error::EvmTokenServiceError},
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    dotenv().ok();

    info!("Hello, world full of tokens!");

    let port = 8080;
    let host = "localhost";
    let workers = 2;

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let evm_token_repository = SqliteEvmTokenRepository::new(database_url);

    let evm_token_service = EvmTokenService::new(evm_token_repository);

    info!("Starting server on port {}", port);

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(evm_token_service.clone()))
            .service(hello_world)
            .service(get_evm_token)
            .service(get_solana_token)
    })
    .bind((host, port))?
    .workers(workers)
    .run()
    .await
}

#[derive(Deserialize)]
struct RpcUrl {
    rpc_url: String,
}

#[get("/tokens/evm/{chain_id}/{evm_address}")]
async fn get_evm_token(
    path: Path<(i32, String)>,
    data: Json<RpcUrl>,
    evm_token_service: Data<EvmTokenService>,
) -> impl Responder {
    let (chain_id, evm_address) = path.into_inner();
    let rpc_url = data.into_inner().rpc_url;

    info!("RPC URL: {:?}", rpc_url);

    info!(
        "Getting EVM token: {:?}, {:?}",
        chain_id.to_string(),
        evm_address.to_string()
    );

    let Ok(checked_address) = evm_address.parse::<Address>() else {
        error!("Invalid EVM address: {}", evm_address);
        return HttpResponse::BadRequest().body("Invalid EVM address");
    };

    let token = evm_token_service
        .get_or_fetch_token(chain_id, checked_address, rpc_url)
        .await;

    return match token {
        Ok(token) => HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(serde_json::to_string(&token).expect("Failed to serialize token")),

        Err(e) => {
            error!("Error getting EVM token: {:?}", e);
            match e {
                EvmTokenServiceError::Repository(e) => {
                    error!("Repository error: {:?}", e);
                    HttpResponse::InternalServerError().body("Repository error")
                }
                EvmTokenServiceError::Chain(e) => {
                    error!("Chain error: {:?}", e);
                    HttpResponse::InternalServerError().body("Chain error")
                }
                EvmTokenServiceError::Multicall(e) => {
                    error!("Multicall error: {:?}", e);
                    HttpResponse::InternalServerError().body("Multicall error")
                }
                EvmTokenServiceError::ChainIdMismatch(expected, actual) => {
                    error!(
                        "Chain ID mismatch: expected {:?}, got {:?}",
                        expected, actual
                    );
                    HttpResponse::BadRequest()
                        .body("Provided chain ID argument and chain id from RPC mismatch")
                }
                EvmTokenServiceError::CaipIdBuildFailed(e) => {
                    error!("CAIP ID build failed: {:?}", e);
                    HttpResponse::BadRequest().body(format!(
                        "Failed to build CAIP ID for token with provided address and chain ID: {}",
                        e
                    ))
                }
                EvmTokenServiceError::BlockingError(e) => {
                    error!("Blocking error: {:?}", e);
                    HttpResponse::InternalServerError().body("Blocking error: failed to get token")
                }
            }
        }
    };
}

#[get("/tokens/solana/{address}")]
async fn get_solana_token(address: Path<String>) -> impl Responder {
    println!("Getting Solana token: {:?}", address.to_string());

    HttpResponse::Ok().body("Hello, Solana!")
}

#[get("/")]
async fn hello_world() -> impl Responder {
    HttpResponse::Ok().body("Hello, TokenAPI!")
}
