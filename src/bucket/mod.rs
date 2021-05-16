mod delete;
mod download;
mod drop;
mod find;
mod rename;
mod upload;
use crate::options::GridFSBucketOptions;
use mongodb::Database;

/// GridFS bucket. A prefix under which a GridFS system’s collections are stored.
/// [Spec](https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst#configurable-gridfsbucket-class)
#[derive(Clone, Debug)]
pub struct GridFSBucket {
    pub(crate) db: Database,
    pub(crate) options: Option<GridFSBucketOptions>,
    // internal: when true should check the indexes
    pub(crate) never_write: bool,
}

impl GridFSBucket {
    /**
     * Create a new GridFSBucket object on @db with the given @options.
     */
    pub fn new(db: Database, options: Option<GridFSBucketOptions>) -> GridFSBucket {
        GridFSBucket {
            db,
            options,
            never_write: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GridFSBucket, GridFSBucketOptions};
    use mongodb::Client;
    use mongodb::{error::Error, Database};
    use uuid::Uuid;
    fn db_name_new() -> String {
        "test_".to_owned()
            + Uuid::new_v4()
                .to_hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
    }

    #[tokio::test]
    async fn grid_f_s_bucket_new() -> Result<(), Error> {
        let client = Client::with_uri_str(
            &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
        )
        .await?;
        let dbname = db_name_new();
        let db: Database = client.database(&dbname);
        let bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));

        if let Some(options) = bucket.options {
            assert_eq!(
                options.bucket_name,
                GridFSBucketOptions::default().bucket_name
            );
        }
        assert_eq!(bucket.db.name(), db.name());

        Ok(())
    }
}
