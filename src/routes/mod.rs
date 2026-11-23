pub mod ab;
pub mod audit;
pub mod auth;
pub mod connection_settings;
pub mod devices;
pub mod frontend;
pub mod groups;
pub mod peer_groups;
pub mod peers;
pub mod presence_settings;
pub mod server_settings;
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
        .merge(connection_settings::routes())
        .merge(ab::routes())
        .merge(peers::routes())
        .merge(presence_settings::routes())
        .merge(peer_groups::routes())
        .merge(tags::routes())
        .merge(system::routes())
        .merge(server_settings::routes())
        .merge(users::routes())
        .merge(groups::routes())
}
