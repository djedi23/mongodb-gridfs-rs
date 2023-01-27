use crate::{bucket::GridFSBucket, options::GridFSFindOptions};
use bson::Document;
use mongodb::error::Result;
use mongodb::options::FindOptions;
use mongodb::Cursor;

impl GridFSBucket {
    /**
    Find and return the files collection documents that match @filter.
    [Spec](https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst#generic-find-on-files-collection)

    # Examples

    ```rust
    use bson::doc;
    # #[cfg(feature = "async-std-runtime")]
    # use futures::stream::StreamExt;
    # #[cfg(any(feature = "default", feature = "tokio-runtime"))]
    use tokio_stream::StreamExt;
    # use mongodb::error::Result;
    # use mongodb::Client;
    # use mongodb::Database;
    use mongodb_gridfs::{bucket::GridFSBucket, options::GridFSFindOptions};
    # use mongodb_gridfs::options::GridFSBucketOptions;

    # #[tokio::main]
    # async fn main() -> Result<()> {
    #    let client = Client::with_uri_str(
    #        &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
    #    )
    #    .await?;
    #    let db: Database = client.database("test");
    #    let bucket = &GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
    let mut cursor = bucket
            .find(doc! {"filename":"test.txt"}, GridFSFindOptions::default())
            .await?;

        while let Some(_doc) = cursor.next().await {
            // ...
        }
    #    Ok(())
    # }
    ```
     */
    pub async fn find(
        &self,
        filter: Document,
        options: GridFSFindOptions,
    ) -> Result<Cursor<Document>> {
        let dboptions = self.options.clone().unwrap_or_default();
        let bucket_name = dboptions.bucket_name;
        let file_collection = bucket_name + ".files";
        let files = self.db.collection::<Document>(&file_collection);

        let find_options = FindOptions::builder()
            .allow_disk_use(options.allow_disk_use)
            .limit(options.limit)
            .max_time(options.max_time)
            .no_cursor_timeout(options.no_cursor_timeout)
            .skip(options.skip)
            .sort(options.sort)
            .read_concern(dboptions.read_concern)
            .build();

        files.find(filter, find_options).await
    }
}

#[cfg(test)]
mod tests {
    use super::GridFSBucket;
    use crate::{
        options::{GridFSBucketOptions, GridFSFindOptions},
        GridFSError,
    };
    use bson::doc;
    #[cfg(feature = "async-std-runtime")]
    use futures::stream::StreamExt;
    use mongodb::{Client, Database};
    #[cfg(any(feature = "default", feature = "tokio-runtime"))]
    use tokio_stream::StreamExt;
    use uuid::Uuid;

    fn db_name_new() -> String {
        "test_".to_owned()
            + Uuid::new_v4()
                .to_hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
    }

    #[tokio::test]
    async fn find_a_file() -> Result<(), GridFSError> {
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

        let mut cursor = bucket
            .find(doc! {"filename":"test.txt"}, GridFSFindOptions::default())
            .await?;

        while let Some(doc) = cursor.next().await {
            let doc = doc.unwrap();
            assert_eq!(doc.get_str("filename").unwrap(), "test.txt");
            assert_eq!(doc.get_i32("chunkSize").unwrap(), 261120);
            assert_eq!(doc.get_i64("length").unwrap(), 9);
            assert_eq!(
                doc.get_str("md5").unwrap(),
                "eb733a00c0c9d336e65691a37ab54293"
            );
        }
        db.drop(None).await?;
        Ok(())
    }

    #[tokio::test]
    async fn find_a_non_existing_file() -> Result<(), GridFSError> {
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

        let mut cursor = bucket
            .find(doc! {"filename":"null.txt"}, GridFSFindOptions::default())
            .await?;

        match cursor.next().await {
            None => assert!(true),
            Some(_) => assert!(false),
        }

        db.drop(None).await?;
        Ok(())
    }
}
