use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use async_graphql_parser::{parse_schema, types::ServiceDocument};
use error::{CannotLoadServiceDocumentFromPath, CannotLoadServiceDocumentFromString};

pub trait TryIntoServiceDocument {
    type Error: 'static;

    fn try_into_service_document(self) -> Result<ServiceDocument, Self::Error>;
}

impl TryIntoServiceDocument for &PathBuf {
    type Error = CannotLoadServiceDocumentFromPath;

    fn try_into_service_document(self) -> Result<ServiceDocument, Self::Error> {
        self.as_path().try_into_service_document()
    }
}

impl TryIntoServiceDocument for PathBuf {
    type Error = CannotLoadServiceDocumentFromPath;

    fn try_into_service_document(self) -> Result<ServiceDocument, Self::Error> {
        self.as_path().try_into_service_document()
    }
}

impl TryIntoServiceDocument for &Path {
    type Error = CannotLoadServiceDocumentFromPath;

    fn try_into_service_document(self) -> Result<ServiceDocument, Self::Error> {
        let mut file = File::open(self).map_err(|e| CannotLoadServiceDocumentFromPath {
            path: self.into(),
            error: e.into(),
        })?;

        let mut file_contents = String::new();
        file.read_to_string(&mut file_contents)
            .map_err(|e| CannotLoadServiceDocumentFromPath {
                path: self.into(),
                error: e.into(),
            })?;

        parse_schema(&file_contents).map_err(|e| CannotLoadServiceDocumentFromPath {
            path: self.into(),
            error: e.into(),
        })
    }
}

impl TryIntoServiceDocument for &str {
    type Error = CannotLoadServiceDocumentFromString;

    fn try_into_service_document(self) -> Result<ServiceDocument, Self::Error> {
        parse_schema(self).map_err(|e| CannotLoadServiceDocumentFromString(e.into()))
    }
}

impl TryIntoServiceDocument for &String {
    type Error = CannotLoadServiceDocumentFromString;

    fn try_into_service_document(self) -> Result<ServiceDocument, Self::Error> {
        parse_schema(self).map_err(|e| CannotLoadServiceDocumentFromString(e.into()))
    }
}

impl TryIntoServiceDocument for String {
    type Error = CannotLoadServiceDocumentFromString;

    fn try_into_service_document(self) -> Result<ServiceDocument, Self::Error> {
        parse_schema(self).map_err(|e| CannotLoadServiceDocumentFromString(e.into()))
    }
}

pub mod error {
    use std::path::PathBuf;

    #[derive(Debug, thiserror::Error)]
    #[error("CannotLoadServiceDocumentFromPath: path = '{path}', error = {error}")]
    pub struct CannotLoadServiceDocumentFromPath {
        pub path: PathBuf,
        // pub error_variant: CannotLoadServiceDocumentFromPathVariants,
        pub error: Box<dyn std::error::Error>,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("CannotLoadServiceDocumentFromString: error = {0}")]
    pub struct CannotLoadServiceDocumentFromString(
        #[from]
        #[source]
        pub Box<dyn std::error::Error>,
    );
}
