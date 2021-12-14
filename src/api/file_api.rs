use crate::api::ReadApi;
use openapi_client::models;
use async_trait::async_trait;
use crate::error::Error;

pub struct FileApi;

impl FileApi {
    pub fn new(_path: &str) -> Self {
        FileApi
    }
}

#[async_trait]
impl ReadApi for FileApi {
    async fn get_monitors(&self) -> Result<Vec<models::Monitor>, Error> {
        unimplemented!()
    }

    async fn get_alerts(&self) -> Result<Vec<models::Alert>, Error> {
        unimplemented!()
    }
}
