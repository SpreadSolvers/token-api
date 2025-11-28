mod db;
mod erc20;
mod evm_token;
mod schema;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, http::header::ContentType, web::Path,
};
use alloy::{
    primitives::{Address, address},
    providers::{Provider, ProviderBuilder},
};
use log::{error, info};
use tap_caip::{AccountId, ChainId};

use crate::{
    erc20::ERC20,
    evm_token::{EvmToken, get_token_data_from_chain},
    schema::evm_tokens,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    info!("Hello, world!");

    let port = 8080;
    let host = "localhost";
    let workers = 2;

    info!("Starting server on port {}", port);

    HttpServer::new(move || {
        App::new()
            .service(hello_world)
            .service(get_evm_token)
            .service(get_solana_token)
    })
    .bind((host, port))?
    .workers(workers)
    .run()
    .await
}

#[get("/tokens/evm/{chain_id}/{evm_address}")]
async fn get_evm_token(path: Path<(i32, String)>) -> impl Responder {
    let (chain_id, evm_address) = path.into_inner();

    info!(
        "Getting EVM token: {:?}, {:?}",
        chain_id.to_string(),
        evm_address.to_string()
    );

    let Ok(address) = evm_address.parse::<Address>() else {
        error!("Invalid EVM address: {}", evm_address);
        return HttpResponse::BadRequest().body("Invalid EVM address");
    };

    let token = get_token_data_from_chain(chain_id, address).await;

    return match token {
        Ok(token) => HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(serde_json::to_string(&token).expect("Failed to serialize token")),
        Err(e) => {
            error!("Error getting EVM token: {:?}", e);
            HttpResponse::InternalServerError().body("Error getting EVM token")
        }
    };
}

#[get("/tokens/solana/{address}")]
async fn get_solana_token(address: Path<String>) -> impl Responder {
    println!("Getting Solana token: {:?}", address.to_string());

    HttpResponse::Ok().body("Hello, world!")
}

#[get("/")]
async fn hello_world() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}
