use crate::db;
use crate::fr;
use crate::state::{DbState, PlainchantState};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::{Router, routing};

use serde::Serialize;

#[derive(Serialize)]
struct ApiBoard {
    pub url:   String,
    pub title: String,
}

async fn boards<DB: db::Database>(
    State(DbState { db }): State<DbState<DB>>,
) -> (StatusCode, Json<Vec<ApiBoard>>) {
    let api_boards = match db.get_boards() {
        Ok(boards) => boards
            .into_iter()
            .map(|board| ApiBoard {
                url:   board.url,
                title: board.title,
            })
            .collect::<Vec<ApiBoard>>(),
        Err(_) => unimplemented!(),
    };

    (StatusCode::OK, Json(api_boards))
}

pub fn get_api_router<DB, FR>() -> Router<PlainchantState<DB, FR>>
where
    DB: db::Database,
    FR: fr::FileRack,
{
    Router::new().route("/boards", routing::get(boards))
}
