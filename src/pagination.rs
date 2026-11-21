use crate::error::ApiError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Pagination {
    #[serde(default = "default_page")]
    pub current: i64,
    #[serde(default = "default_size", rename = "pageSize")]
    pub page_size: i64,
}
pub(crate) fn default_page() -> i64 {
    1
}
pub(crate) fn default_size() -> i64 {
    100
}

impl Pagination {
    pub fn offset(&self) -> Result<i64, ApiError> {
        if self.current < 1 || !(1..=100).contains(&self.page_size) {
            return Err(ApiError::BadRequest(
                "current must be >= 1 and pageSize between 1 and 100".into(),
            ));
        }
        (self.current - 1)
            .checked_mul(self.page_size)
            .ok_or_else(|| ApiError::BadRequest("Page is too large".into()))
    }
}
