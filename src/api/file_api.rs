use crate::api::{ApiFuture, ReadApi};
use crate::openapi_client::models;

pub struct FileApi;

impl FileApi {
    //pub fn new(_path: &str) -> Self {
    //    FileApi
    //}
}

impl ReadApi for FileApi {
    fn get_monitors(&self) -> ApiFuture<Vec<models::Monitor>> {
        unimplemented!()
    }

    fn get_alerts(&self) -> ApiFuture<Vec<models::Alert>> {
        unimplemented!()
    }
}
