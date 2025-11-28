mod db;
mod erc20;
mod evm_token;
mod schema;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, http::header::ContentType, web::Path,
};
use alloy::primitives::Address;
use log::{error, info, warn};
use tap_caip::{AccountId, ChainId};

use crate::{
    db::establish_connection,
    evm_token::{find_token_by_id, get_token_data_from_chain, save_evm_token},
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

    let mut connection = establish_connection();

    let Ok(checked_address) = evm_address.to_string().parse::<Address>() else {
        error!("Invalid EVM address: {}", evm_address);
        return HttpResponse::BadRequest().body("Invalid EVM address");
    };

    let caip_account_id = match AccountId::new(
        ChainId::new("eip155", &chain_id.to_string()).unwrap(),
        &checked_address.to_checksum(None),
    ) {
        Ok(account_id) => account_id,
        Err(e) => {
            error!("Error creating CAIP account id: {:?}", e);
            return HttpResponse::BadRequest().body("Error creating CAIP account id");
        }
    };

    match find_token_by_id(&mut connection, caip_account_id) {
        Ok(token) => {
            info!("Found token by id: {:?}", token.id);
            return HttpResponse::Ok()
                .content_type(ContentType::json())
                .body(serde_json::to_string(&token).expect("Failed to serialize token"));
        }
        Err(e) => {
            dbg!(&e);
            warn!("Not found token by id: {:?}", e);
        }
    }

    let Ok(address) = evm_address.parse::<Address>() else {
        error!("Invalid EVM address: {}", evm_address);
        return HttpResponse::BadRequest().body("Invalid EVM address");
    };

    let token = get_token_data_from_chain(chain_id, address).await;

    return match token {
        Ok(token) => {
            let _ = save_evm_token(&mut connection, &token).map_err(|e| {
                error!("Error saving EVM token: {:?}", e);
                HttpResponse::InternalServerError().body("Error saving EVM token")
            });

            HttpResponse::Ok()
                .content_type(ContentType::json())
                .body(serde_json::to_string(&token).expect("Failed to serialize token"))
        }
        Err(e) => {
            error!("Error getting EVM token: {:?}", e);
            HttpResponse::InternalServerError().body("Error getting EVM token")
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
