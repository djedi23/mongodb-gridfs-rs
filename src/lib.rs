//! This crate provides an implementation of Mongo GridFS on the top of mongodb's crate.
//! This implementation only use the _async/await_ version of mongodb.
//!
//! From https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst
//! > GridFS is a convention drivers use to store and retrieve BSON binary data (type “\x05”) that exceeds MongoDB’s BSON-document size limit of 16 MiB. When this data, called a user file, is written to the system, GridFS divides the file into chunks that are stored as distinct documents in a chunks collection. To retrieve a stored file, GridFS locates and returns all of its component chunks. Internally, GridFS creates a files collection document for each stored file. Files collection documents hold information about stored files, and they are stored in a files collection.
//! # Examples
//! Uploading a document:
//!  ```rust
//!  # use mongodb::Client;
//!  # use mongodb::{error::Error, Database};
//!  use mongodb_gridfs::{options::GridFSBucketOptions, GridFSBucket};
//!  # use uuid::Uuid;
//!  
//!  # fn db_name_new() -> String {
//!  #     "test_".to_owned()
//!  #         + Uuid::new_v4()
//!  #             .to_hyphenated()
//!  #             .encode_lower(&mut Uuid::encode_buffer())
//!  # }
//!  #
//!  # #[tokio::main]
//!  # async fn main() -> Result<(), Error> {
//!  #    let client = Client::with_uri_str(&std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string())).await?;
//!  #    let dbname = db_name_new();
//!  #    let db: Database = client.database(&dbname);
//!  let bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
//!  let id = bucket
//!      .upload_from_stream("test.txt", "stream your data here".as_bytes(), None)
//!      .await?;
//!  #     println!("{}", id);
//!  #     db.drop(None).await
//!  # }
//!  ```
//!  Downloading a document:
//!  ```rust
//! use futures::stream::StreamExt;
//! # use mongodb::Client;
//! # use mongodb::Database;
//! use mongodb_gridfs::{options::GridFSBucketOptions, GridFSBucket, GridFSError};
//! # use uuid::Uuid;
//!
//! # fn db_name_new() -> String {
//! #     "test_".to_owned()
//! #         + Uuid::new_v4()
//! #             .to_hyphenated()
//! #             .encode_lower(&mut Uuid::encode_buffer())
//! # }
//! #
//! # #[tokio::main]
//! # async fn main() -> Result<(), GridFSError> {
//! #     let client = Client::with_uri_str(
//! #         &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
//! #     )
//! #     .await?;
//! #     let dbname = db_name_new();
//! #     let db: Database = client.database(&dbname);
//! let bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
//! #     let id = bucket
//! #         .clone()
//! #         .upload_from_stream("test.txt", "test data".as_bytes(), None)
//! #         .await?;
//! #     println!("{}", id);
//! #
//! let mut cursor = bucket.open_download_stream(id).await?;
//! let buffer = cursor.next().await.unwrap();
//! #     println!("{:?}", buffer);
//! #
//! #     db.drop(None).await?;
//! #     Ok(())
//! # }
//!  ```
//! # Features
//! The following features are propagated to mongodb:
//! - default
//! - async-std-runtime
//! - tokio-runtime
//! # Code Status
//! | Feature                                     | Status  | Notes                                           |
//! | ------------------------------------------- | ------- | ----------------------------------------------- |
//! | GridFSUploadOptions                         | DONE    | `contentType` and `aliases` are not implemented |
//! | GridFSBucketOption                          | DONE    | concerns not used when ensuring indexes         |
//! | GridFSFindOptions                           | DONE    |                                                 |
//! | GridFSDownloadByNameOptions                 | TODO    |                                                 |
//! | GridFSBucket                                | DONE    |                                                 |
//! | GridFSBucket . open_upload_stream           | DONE    |                                                 |
//! | GridFSBucket . open_upload_stream_with_id   |         |                                                 |
//! | GridFSBucket . upload_from_stream           | NO      | No Implementation planned                         |
//! | GridFSBucket . upload_from_stream_with_id   | NO      | No Implementation planned                         |
//! | GridFSBucket . open_download_stream         | DONE    |                                                 |
//! | GridFSBucket . download_to_stream           | NO      | No Implementation planned                         |
//! | GridFSBucket . delete                       | DONE    |                                                 |
//! | GridFSBucket . find                         | DONE    |                                                 |
//! | GridFSBucket . rename                       | DONE    |                                                 |
//! | GridFSBucket . drop                         | DONE    |                                                 |
//! | GridFSBucket . open_download_stream_by_name |         |                                                 |
//! | GridFSBucket . download_to_stream_by_name   |         |                                                 |
//! | indexes                                     | DONE   |                                                 |

pub mod bucket;
pub mod options;
use std::{
    error::Error,
    fmt::{Display, Formatter, Result},
};

pub use bucket::GridFSBucket;

#[derive(Debug)]
pub enum GridFSError {
    MongoError(mongodb::error::Error),
    FileNotFound(),
}

impl From<mongodb::error::Error> for GridFSError {
    fn from(err: mongodb::error::Error) -> GridFSError {
        GridFSError::MongoError(err)
    }
}

impl Error for GridFSError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            GridFSError::MongoError(e) => Some(e),
            GridFSError::FileNotFound() => None,
        }
    }

    // fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
    //     None
    // }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl Display for GridFSError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            GridFSError::MongoError(me) => write!(f, "{}", me),
            GridFSError::FileNotFound() => write!(f, "File not found"),
        }
    }
}
