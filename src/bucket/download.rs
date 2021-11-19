use crate::{bucket::GridFSBucket, GridFSError};
use bson::{doc, oid::ObjectId, Document};
use futures::{Stream, StreamExt, TryFutureExt};
use mongodb::options::{FindOneOptions, FindOptions, SelectionCriteria};

impl GridFSBucket {
    /// Opens a Stream from which the application can read the contents of the stored file
    /// specified by @id.
    /// [Spec](https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst#file-download)
    /// 
    /// Returns a [`Stream`].
    /// 
    /// # Examples
    /// 
    ///  ```rust
    ///  use futures::stream::StreamExt;
    ///  # use mongodb::Client;
    ///  # use mongodb::Database;
    ///  use mongodb_gridfs::{options::GridFSBucketOptions, GridFSBucket, GridFSError};
    ///  # use uuid::Uuid;
    ///  # fn db_name_new() -> String {
    ///  #     "test_".to_owned()
    ///  #         + Uuid::new_v4()
    ///  #             .to_hyphenated()
    ///  #             .encode_lower(&mut Uuid::encode_buffer())
    ///  # }
    ///  #
    ///  # #[tokio::main]
    ///  # async fn main() -> Result<(), GridFSError> {
    ///  #     let client = Client::with_uri_str(
    ///  #         &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
    ///  #     )
    ///  #     .await?;
    ///  #     let dbname = db_name_new();
    ///  #     let db: Database = client.database(&dbname);
    ///  let bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
    ///  #     let id = bucket
    ///  #         .clone()
    ///  #         .upload_from_stream("test.txt", "test data".as_bytes(), None)
    ///  #         .await?;
    ///  #     println!("{}", id);
    ///  #
    ///  let (mut cursor, filename) = bucket.open_download_stream_with_filename(id).await?;
    ///  assert_eq!(filename, "test.txt");
    ///  let buffer = cursor.next().await.unwrap();
    ///  #     println!("{:?}", buffer);
    ///  #
    ///  #     db.drop(None).await?;
    ///  #     Ok(())
    ///  # }
    ///  ```
    /// 
    ///  # Errors
    /// 
    ///  Raise [`GridFSError::FileNotFound`] when the requested id doesn't exists.
    ///
    pub async fn open_download_stream_with_filename(
        &self,
        id: ObjectId,
    ) -> Result<(impl Stream<Item = Vec<u8>>, String), GridFSError> {
        let dboptions = self.options.clone().unwrap_or_default();
        let bucket_name = dboptions.bucket_name;
        let file_collection = bucket_name.clone() + ".files";
        let files = self.db.collection::<Document>(&file_collection);
        let chunk_collection = bucket_name + ".chunks";
        let chunks = self.db.collection::<Document>(&chunk_collection);

        let mut find_one_options = FindOneOptions::default();
        let mut find_options = FindOptions::builder().sort(doc! {"n":1}).build();

        if let Some(read_concern) = dboptions.read_concern {
            find_one_options.read_concern = Some(read_concern.clone());
            find_options.read_concern = Some(read_concern);
        }
        if let Some(read_preference) = dboptions.read_preference {
            find_one_options.selection_criteria =
                Some(SelectionCriteria::ReadPreference(read_preference.clone()));
            find_options.selection_criteria =
                Some(SelectionCriteria::ReadPreference(read_preference));
        }

        /*
        Drivers must first retrieve the files collection document for this
        file. If there is no files collection document, the file either never
        existed, is in the process of being deleted, or has been corrupted,
        and the driver MUST raise an error.
        */
        let file = files
            .find_one(doc! {"_id":id.clone()}, find_one_options)
            .await?;

        if let Some(file) = file {
            let filename = file.get_str("filename").unwrap().to_string();
            let stream =
            chunks
                .find(doc! {"files_id":id}, find_options.clone())
                .await
                .unwrap()
                .map(|item| {
                    let i = item.unwrap();
                    i.get_binary_generic("data").unwrap().clone()
                });
            Ok((stream, filename))
        } else {
            Err(GridFSError::FileNotFound())
        }
    }

    /**
     Opens a Stream from which the application can read the contents of the stored file
     specified by @id.
     [Spec](https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst#file-download)

     Returns a [`Stream`].

     # Examples

     ```rust
     use futures::stream::StreamExt;
     # use mongodb::Client;
     # use mongodb::Database;
     use mongodb_gridfs::{options::GridFSBucketOptions, GridFSBucket, GridFSError};
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
     #     println!("{}", id);
     #
     let mut cursor = bucket.open_download_stream(id).await?;
     let buffer = cursor.next().await.unwrap();
     #     println!("{:?}", buffer);
     #
     #     db.drop(None).await?;
     #     Ok(())
     # }
     ```

     # Errors

     Raise [`GridFSError::FileNotFound`] when the requested id doesn't exists.
    */
    pub async fn open_download_stream(
        &self,
        id: ObjectId,
    ) -> Result<impl Stream<Item = Vec<u8>>, GridFSError> {
        self.open_download_stream_with_filename(id).map_ok(|(stream, _)| stream).await
    }
}

#[cfg(test)]
mod tests {
    use super::GridFSBucket;
    use crate::{options::GridFSBucketOptions, GridFSError};
    use bson::oid::ObjectId;
    use futures::stream::StreamExt;
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
    async fn open_download_stream() -> Result<(), GridFSError> {
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

        let mut cursor = bucket.open_download_stream(id).await?;
        let buffer = cursor.next().await.unwrap();
        assert_eq!(buffer, [116, 101, 115, 116, 32, 100, 97, 116, 97]);
        db.drop(None).await?;
        Ok(())
    }
    #[tokio::test]
    async fn open_download_stream_chunk_size() -> Result<(), GridFSError> {
        let client = Client::with_uri_str(
            &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
        )
        .await?;
        let dbname = db_name_new();
        let db: Database = client.database(&dbname);
        let bucket = &GridFSBucket::new(
            db.clone(),
            Some(GridFSBucketOptions::builder().chunk_size_bytes(4).build()),
        );
        let id = bucket
            .clone()
            .upload_from_stream("test.txt", "test data".as_bytes(), None)
            .await?;

        assert_eq!(id.to_hex(), id.to_hex());

        let mut cursor = bucket.open_download_stream(id).await?;
        let buffer = cursor.next().await.unwrap();
        assert_eq!(buffer, [116, 101, 115, 116]);

        let buffer = cursor.next().await.unwrap();
        assert_eq!(buffer, [32, 100, 97, 116]);

        let buffer = cursor.next().await.unwrap();
        assert_eq!(buffer, [97]);

        let buffer = cursor.next().await;
        assert_eq!(buffer, None);

        db.drop(None).await?;
        Ok(())
    }

    #[tokio::test]
    async fn open_download_stream_not_existing_file() -> Result<(), GridFSError> {
        let client = Client::with_uri_str(
            &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
        )
        .await?;
        let dbname = db_name_new();
        let db: Database = client.database(&dbname);
        let bucket = &GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
        let id = ObjectId::new();

        let cursor = bucket.open_download_stream(id).await;
        assert!(cursor.is_err());

        db.drop(None).await?;
        Ok(())
    }
}
