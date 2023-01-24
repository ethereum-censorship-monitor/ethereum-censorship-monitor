use actix_web::{web, App, HttpServer, Result};

use crate::{cli::Config, db};

mod errors;
use errors::*;

mod requests;
use requests::*;

mod handlers;
use handlers::*;

mod queries;
use queries::*;

mod responses;
use responses::*;

pub struct AppState {
    config: Config,
    pool: db::Pool,
}

pub async fn serve_api(config: Config) -> Result<(), std::io::Error> {
    let pool = db::connect(&config.api_db_connection).await.unwrap();
    let host_and_port = (config.api_host.clone(), config.api_port);
    HttpServer::new(move || {
        let state = AppState {
            config: config.clone(),
            pool: pool.clone(),
        };
        App::new()
            .app_data(web::Data::new(state))
            .service(handle_misses)
            .service(handle_txs)
            .service(handle_blocks)
    })
    .bind(host_and_port)?
    .run()
    .await
}
