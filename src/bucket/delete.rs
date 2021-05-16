use crate::{bucket::GridFSBucket, GridFSError};
use bson::{doc, oid::ObjectId};
use mongodb::options::DeleteOptions;

impl GridFSBucket {
    /**
    Given a @id, delete this stored file’s files collection document and
    associated chunks from a GridFS bucket.
     [Spec](https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst#file-deletion)


    ```rust
     # use mongodb::Client;
     # use mongodb::Database;
     # use mongodb_gridfs::{options::GridFSBucketOptions};
     use mongodb_gridfs::{GridFSBucket, GridFSError};
     # use uuid::Uuid;
     # fn db_name_new() -> String {
     #     "test_".to_owned()
     #         + Uuid::new_v4()
     #             .to_hyphenated()
     #             .encode_lower(&mut Uuid::encode_buffer())
     # }
     #
     # #[tokio::main]
     # async fn main() -> Result<(), GridFSError> {
     #     let client = Client::with_uri_str(
     #         &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
     #     )
     #     .await?;
     #     let dbname = db_name_new();
     #     let db: Database = client.database(&dbname);
     let bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
     #     let id = bucket
     #         .clone()
     #         .upload_from_stream("test.txt", "test data".as_bytes(), None)
     #         .await?;
     #
     bucket.delete(id).await?;
     #
     #     db.drop(None).await?;
     #     Ok(())
     # }
    ```
     # Errors

     Raise [`GridFSError::FileNotFound`] when the requested id doesn't exists.
    */
    pub async fn delete(&self, id: ObjectId) -> Result<(), GridFSError> {
        let dboptions = self.options.clone().unwrap_or_default();
        let bucket_name = dboptions.bucket_name;
        let file_collection = bucket_name.clone() + ".files";
        let files = self.db.collection(&file_collection);
        let chunk_collection = bucket_name + ".chunks";
        let chunks = self.db.collection(&chunk_collection);

        let mut delete_option = DeleteOptions::default();
        if let Some(write_concern) = dboptions.write_concern.clone() {
            delete_option.write_concern = Some(write_concern);
        }

        let delete_result = files
            .delete_one(doc! {"_id":id.clone()}, delete_option.clone())
            .await?;

        // If there is no such file listed in the files collection,
        // drivers MUST raise an error.
        if delete_result.deleted_count == 0 {
            return Err(GridFSError::FileNotFound());
        }

        chunks
            .delete_many(doc! {"files_id":id}, delete_option)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::GridFSBucket;
    use crate::{options::GridFSBucketOptions, GridFSError};
    use bson::doc;
    use bson::oid::ObjectId;
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
    async fn delete_a_file() -> Result<(), GridFSError> {
        let client = Client::with_uri_str(
            &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
        )
        .await?;
        let dbname = db_name_new();
        let db: Database = client.database(&dbname);
        let bucket = &GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
        let id = bucket
            .clone()
            .upload_from_stream("test.txt", "test data".as_bytes(), None)
            .await?;

        assert_eq!(id.to_hex(), id.to_hex());

        bucket.delete(id.clone()).await?;

        let count = db
            .collection("fs.files")
            .count_documents(doc! { "_id": id.clone() }, None)
            .await?;
        assert_eq!(count, 0, "File should be deleted");

        let count = db
            .collection("fs.chunks")
            .count_documents(doc! { "files_id": id }, None)
            .await?;
        assert_eq!(count, 0, "Chunks should be deleted");

        db.drop(None).await?;
        Ok(())
    }

    #[tokio::test]
    async fn delete_a_non_existant_file() -> Result<(), GridFSError> {
        let client = Client::with_uri_str(
            &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
        )
        .await?;
        let dbname = db_name_new();
        let db: Database = client.database(&dbname);
        let bucket = &GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
        let id = ObjectId::new();

        let result = bucket.delete(id.clone()).await;
        assert!(result.is_err());

        let count = db
            .collection("fs.files")
            .count_documents(doc! { "_id": id.clone() }, None)
            .await?;
        assert_eq!(count, 0, "File should be deleted");

        let count = db
            .collection("fs.chunks")
            .count_documents(doc! { "files_id": id }, None)
            .await?;
        assert_eq!(count, 0, "Chunks should be deleted");

        db.drop(None).await?;
        Ok(())
    }
}
