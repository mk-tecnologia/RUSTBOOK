pub mod ab;
pub mod audit;
pub mod auth;
pub mod devices;
pub mod frontend;
pub mod groups;
pub mod peers;
pub mod system;
pub mod tags;
pub mod users;

use crate::state::AppState;
use axum::Router;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .merge(auth::routes())
        .merge(audit::routes())
        .merge(devices::routes())
        .merge(ab::routes())
        .merge(peers::routes())
        .merge(tags::routes())
        .merge(system::routes())
        .merge(users::routes())
        .merge(groups::routes())
}
