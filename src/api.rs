use crate::db;
use crate::fr;
use crate::state::{DbState, PlainchantState};
use crate::util::{ErrOrigin, PlainchantErr};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::{Router, routing};

use serde::Serialize;

#[derive(Serialize)]
struct ApiError {
    message: String,
}

type ApiResponse<T> = (StatusCode, Json<T>);
type ApiErrorResponse = (StatusCode, Json<ApiError>);

type ApiResult<T> = Result<ApiResponse<T>, ApiErrorResponse>;

fn api_ok<T>(inner: T) -> ApiResult<T> {
    Ok((StatusCode::OK, Json(inner)))
}

impl From<PlainchantErr> for ApiErrorResponse {
    fn from(err: PlainchantErr) -> Self {
        let code = match err.origin {
            ErrOrigin::Web(c) => {
                StatusCode::from_u16(c).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            },
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (code, Json(ApiError { message: err.msg }))
    }
}

#[derive(Serialize)]
struct ApiSite {
    pub name:        String,
    pub description: String,
    pub contact:     Option<String>,
    pub url:         Option<String>,
}

async fn site<DB: db::Database>(State(DbState { db }): State<DbState<DB>>) -> ApiResult<ApiSite> {
    let site = db.get_site()?;
    let api_site = ApiSite {
        name:        site.name,
        description: site.description,
        contact:     site.contact,
        url:         site.url,
    };
    api_ok(api_site)
}

#[derive(Serialize)]
struct ApiBoard {
    pub url:           String,
    pub title:         String,
    pub post_cap:      u16,
    pub archive_cap:   u16,
    pub bump_limit:    u16,
    pub next_post_num: u64,
}

async fn boards<DB: db::Database>(
    State(DbState { db }): State<DbState<DB>>,
) -> ApiResult<Vec<ApiBoard>> {
    let api_boards = db
        .get_boards()?
        .into_iter()
        .map(|board| ApiBoard {
            url:           board.url,
            title:         board.title,
            post_cap:      board.post_cap,
            archive_cap:   board.archive_cap,
            bump_limit:    board.bump_limit,
            next_post_num: board.next_post_num,
        })
        .collect::<Vec<ApiBoard>>();

    api_ok(api_boards)
}

pub fn get_api_router<DB, FR>() -> Router<PlainchantState<DB, FR>>
where
    DB: db::Database,
    FR: fr::FileRack,
{
    Router::new()
        .route("/site", routing::get(site))
        .route("/boards", routing::get(boards))
}
