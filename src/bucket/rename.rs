use crate::bucket::GridFSBucket;
use bson::{doc, oid::ObjectId, Document};
use mongodb::{error::Result, options::UpdateOptions, results::UpdateResult};

impl GridFSBucket {
    /**
    Renames the stored file with the specified @id.
    [Spec](https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst#renaming-stored-files)

     */
    pub async fn rename(&self, id: ObjectId, new_filename: &str) -> Result<UpdateResult> {
        let dboptions = self.options.clone().unwrap_or_default();
        let bucket_name = dboptions.bucket_name;
        let file_collection = bucket_name + ".files";
        let files = self.db.collection::<Document>(&file_collection);

        let update_options = UpdateOptions::builder()
            .write_concern(dboptions.write_concern)
            .build();

        files
            .update_one(
                doc! {"_id":id},
                doc! {"$set":{"filename":new_filename}},
                update_options,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::GridFSBucket;
    use crate::{options::GridFSBucketOptions, GridFSError};
    use bson::doc;
    use bson::Document;
    use mongodb::Client;
    use mongodb::Database;
    use uuid::Uuid;
    fn db_name_new() -> String {
        "test_".to_owned()
            + Uuid::new_v4()
                .hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
    }

    #[tokio::test]
    async fn rename_a_file() -> Result<(), GridFSError> {
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

        bucket.rename(id, "renamed_file.txt").await?;

        let file = db
            .collection::<Document>("fs.files")
            .find_one(doc! { "_id": id }, None)
            .await?
            .unwrap();
        assert_eq!(file.get_str("filename").unwrap(), "renamed_file.txt");

        db.drop(None).await?;
        Ok(())
    }
}
