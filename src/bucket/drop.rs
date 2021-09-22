use crate::bucket::GridFSBucket;
use mongodb::error::Result;
use bson::Document;

impl GridFSBucket {
    /**
    Drops the files and chunks collections associated with this
    bucket.
    [Spec](https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst#dropping-an-entire-gridfs-bucket)
     */
    pub async fn drop(&self) -> Result<()> {
        let dboptions = self.options.clone().unwrap_or_default();
        let bucket_name = dboptions.bucket_name;
        let file_collection = bucket_name.clone() + ".files";
        let files = self.db.collection::<Document>(&file_collection);

        // let drop_options = DropCollectionOptions::builder()
        //     .write_concern(dboptions.write_concern.clone())
        //     .build();

        // FIXME: MongoError(Error { kind: CommandError(CommandError { code: 14, code_name: "TypeMismatch", message: "\"writeConcern\" had the wrong type. Expected object, found null", labels: [] }), labels: [] })
        files.drop(None).await?;

        let chunk_collection = bucket_name + ".chunks";
        let chunks = self.db.collection::<Document>(&chunk_collection);

        //        let drop_options = DropCollectionOptions::default(); //  builder()
        // .write_concern(dboptions.write_concern)
        // .build();

        chunks.drop(None).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::GridFSBucket;
    use crate::{options::GridFSBucketOptions, GridFSError};

    use mongodb::Client;
    use mongodb::Database;
    use uuid::Uuid;
    fn db_name_new() -> String {
        "test_".to_owned()
            + Uuid::new_v4()
                .to_hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
    }

    #[tokio::test]
    async fn drop_bucket() -> Result<(), GridFSError> {
        let client = Client::with_uri_str(
            &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
        )
        .await?;
        let dbname = db_name_new();
        let db: Database = client.database(&dbname);
        let bucket = &GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
        bucket
            .clone()
            .upload_from_stream("test.txt", "test data".as_bytes(), None)
            .await?;

        let coll_list = db.list_collection_names(None).await?;
        assert!(coll_list.contains(&"fs.files".to_string()));
        assert!(coll_list.contains(&"fs.chunks".to_string()));

        bucket.drop().await?;

        let coll_list = db.list_collection_names(None).await?;
        assert!(coll_list.is_empty());

        db.drop(None).await?;
        Ok(())
    }
}
