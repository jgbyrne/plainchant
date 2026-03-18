use crate::actions::Actions;
use crate::db;
use crate::fr;
use crate::site;
use crate::state::{DbState, PlainchantState};
use crate::util::{ErrOrigin, PlainchantErr};

use axum::Json;
use axum::extract;
use axum::extract::State;
use axum::http::StatusCode;
use axum::{Router, routing};

use std::sync::Arc;

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

impl From<site::Board> for ApiBoard {
    fn from(board: site::Board) -> Self {
        ApiBoard {
            url:           board.url,
            title:         board.title,
            post_cap:      board.post_cap,
            archive_cap:   board.archive_cap,
            bump_limit:    board.bump_limit,
            next_post_num: board.next_post_num,
        }
    }
}

async fn boards<DB: db::Database>(
    State(DbState { db }): State<DbState<DB>>,
) -> ApiResult<Vec<ApiBoard>> {
    let api_boards = db
        .get_boards()?
        .into_iter()
        .map(|b| b.into())
        .collect::<Vec<ApiBoard>>();

    api_ok(api_boards)
}

async fn board<DB: db::Database>(
    State(actions): State<Arc<Actions>>,
    State(DbState { db }): State<DbState<DB>>,
    extract::Path(board): extract::Path<String>,
) -> ApiResult<ApiBoard> {
    let board_id = actions.board_url_to_id(&board)?;
    let board = db.get_board(board_id)?.into();
    api_ok(board)
}

#[derive(Serialize)]
struct ApiOriginal {
    board_url:    String,
    post_num:     u64,
    time:         u64,
    poster:       Option<String>,
    title:        Option<String>,
    body:         String,
    is_moderator: bool,
    is_admin:     bool,
    trip:         Option<String>,
    file_id:      Option<String>,
    is_approved:  bool,
    is_flagged:   bool,
    bump_time:    u64,
    replies:      u16,
    img_replies:  u16,
    archived:     bool,
}

fn original_to_api(
    actions: &Arc<Actions>,
    orig: site::Original,
) -> Result<ApiOriginal, PlainchantErr> {
    Ok(ApiOriginal {
        board_url:    actions.board_id_to_url(orig.board_id)?,
        post_num:     orig.post_num,
        time:         orig.time,
        poster:       orig.poster,
        title:        orig.title,
        body:         orig.body,
        is_moderator: matches!(orig.feather, site::Feather::Moderator),
        is_admin:     matches!(orig.feather, site::Feather::Admin),
        trip:         match orig.feather {
            site::Feather::Trip(s) => Some(s),
            _ => None,
        },
        file_id:      orig.file_id,
        is_approved:  matches!(orig.approval, site::Approval::Approved),
        is_flagged:   matches!(orig.approval, site::Approval::Flagged),
        bump_time:    orig.bump_time,
        replies:      orig.replies,
        img_replies:  orig.img_replies,
        archived:     orig.archived,
    })
}

async fn threads<DB: db::Database>(
    State(actions): State<Arc<Actions>>,
    State(DbState { db }): State<DbState<DB>>,
    extract::Path(board_url): extract::Path<String>,
) -> ApiResult<Vec<ApiOriginal>> {
    let board_id = actions.board_url_to_id(&board_url)?;
    let threads = db
        .get_catalog(board_id)?
        .originals
        .into_iter()
        .map(|orig| original_to_api(&actions, orig))
        .collect::<Result<Vec<ApiOriginal>, PlainchantErr>>()?;
    api_ok(threads)
}

#[derive(Serialize)]
struct ApiReply {
    board_url:    String,
    orig_num:     u64,
    post_num:     u64,
    time:         u64,
    poster:       Option<String>,
    body:         String,
    is_moderator: bool,
    is_admin:     bool,
    trip:         Option<String>,
    file_id:      Option<String>,
    is_approved:  bool,
    is_flagged:   bool,
}

fn reply_to_api(actions: &Arc<Actions>, reply: site::Reply) -> Result<ApiReply, PlainchantErr> {
    Ok(ApiReply {
        board_url:    actions.board_id_to_url(reply.board_id)?,
        orig_num:     reply.orig_num,
        post_num:     reply.post_num,
        time:         reply.time,
        poster:       reply.poster,
        body:         reply.body,
        is_moderator: matches!(reply.feather, site::Feather::Moderator),
        is_admin:     matches!(reply.feather, site::Feather::Admin),
        trip:         match reply.feather {
            site::Feather::Trip(s) => Some(s),
            _ => None,
        },
        file_id:      reply.file_id,
        is_approved:  matches!(reply.approval, site::Approval::Approved),
        is_flagged:   matches!(reply.approval, site::Approval::Flagged),
    })
}

#[derive(Serialize)]
struct ApiThread {
    original: ApiOriginal,
    replies:  Vec<ApiReply>,
}

async fn thread<DB: db::Database>(
    State(actions): State<Arc<Actions>>,
    State(DbState { db }): State<DbState<DB>>,
    extract::Path((board_url, post_num)): extract::Path<(String, u64)>,
) -> ApiResult<ApiThread> {
    let thread = db.get_thread(actions.board_url_to_id(&board_url)?, post_num)?;
    api_ok(ApiThread {
        original: original_to_api(&actions, thread.original)?,
        replies:  thread
            .replies
            .into_iter()
            .map(|reply| reply_to_api(&actions, reply))
            .collect::<Result<Vec<ApiReply>, PlainchantErr>>()?,
    })
}

pub fn get_api_router<DB, FR>() -> Router<PlainchantState<DB, FR>>
where
    DB: db::Database,
    FR: fr::FileRack,
{
    Router::new()
        .route("/site", routing::get(site))
        .route("/boards", routing::get(boards))
        .route("/board/{board_url}", routing::get(board))
        .route("/board/{board_url}/threads", routing::get(threads))
        .route("/board/{board_url}/thread/{post_num}", routing::get(thread))
}
