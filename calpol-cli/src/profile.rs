use crate::ClientError;
use calpol_model::api_v1::UserSummary;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use url::Url;

lazy_static! {
    static ref CONFIG_DIR: PathBuf = dirs::config_dir().unwrap().join("calpol");
    static ref PROFILE_PATH: PathBuf = CONFIG_DIR.join("profile").with_extension("json");
}

#[derive(Serialize, Deserialize)]
pub struct Profile {
    pub token: String,
    pub user: UserSummary,
    pub url: Url,
}

impl Profile {
    pub fn load_profile(path_override: Option<&PathBuf>) -> Result<Self, ClientError> {
        let file = fs::read_to_string(path_override.unwrap_or(&PROFILE_PATH))
            .map_err(ClientError::FailedToReadProfileFile)?;
        serde_json::from_str(&file).map_err(ClientError::FailedToParseProfile)
    }

    pub fn save_profile(&self, path_override: Option<&PathBuf>) -> Result<(), ClientError> {
        let json = serde_json::to_string_pretty(&self).unwrap();
        let path = path_override.unwrap_or(&PROFILE_PATH);
        fs::create_dir_all(path.parent().unwrap())
            .map_err(ClientError::FailedToCreateAppDirectory)?;
        fs::write(path, json).map_err(ClientError::FailedToWriteProfileFile)
    }

    pub fn exists(path_override: Option<&PathBuf>) -> bool {
        path_override.unwrap_or(&PROFILE_PATH).exists()
    }

    pub fn delete(path_override: Option<&PathBuf>) -> Result<(), ClientError> {
        fs::remove_file(path_override.unwrap_or(&PROFILE_PATH))
            .map_err(ClientError::FailedToDeleteProfileFile)
    }

    pub fn route_url(&self, route: &'static str) -> Url {
        self.url.join(route).unwrap()
    }

    pub fn route_url_with_id<I: ToString>(&self, route: &'static str, id: &I) -> Url {
        assert!(route.ends_with('/'));
        self.url
            .join(route)
            .unwrap()
            .join(id.to_string().as_str())
            .unwrap()
    }

    pub fn route_url_with_id_and<I: ToString>(
        &self,
        route: &'static str,
        id: &I,
        and: &'static str,
    ) -> Url {
        assert!(route.ends_with('/'));
        self.url
            .join(route)
            .unwrap()
            .join(format!("{}/", id.to_string()).as_str())
            .unwrap()
            .join(and)
            .unwrap()
    }
}
