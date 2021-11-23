use crate::bucket::GridFSBucket;
use crate::options::GridFSUploadOptions;
use bson::{doc, oid::ObjectId, Document};
use chrono::Utc;
use md5::{Digest, Md5};
use mongodb::{
    error::Error,
    options::{FindOneOptions, InsertOneOptions, UpdateOptions},
    Collection,
};
use futures::io::{AsyncRead, AsyncReadExt};

impl GridFSBucket {
    async fn create_files_index(&self, collection_name: &str) -> Result<Document, Error> {
        self.db
            .run_command(
                doc! {
                    "createIndexes": collection_name,
                    "indexes": [
                        {
                            "key": {
                                "filename":1,
                                "uploadDate":1.0
                            },
                            "name": collection_name.to_owned()+"_index",
                    }]},
                None,
            )
            .await
    }

    async fn create_chunks_index(&self, collection_name: &str) -> Result<Document, Error> {
        self.db
            .run_command(
                doc! {
                "createIndexes": collection_name,
                "indexes": [
                    {
                        "key": {
                             "files_id":1,
                             "n":1
                        },
                        "name": collection_name.to_owned()+"_index",
                }]},
                None,
            )
            .await
    }

    /// Ensure the index of fs.files collection is created before first write operation.
    /// [Spec](https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst#before-write-operations)
    async fn ensure_file_index(
        &mut self,
        files: &Collection<Document>,
        file_collection: &str,
        chunk_collection: &str,
    ) -> Result<(), Error> {
        if self.never_write {
            if files
                .find_one(
                    doc! {},
                    FindOneOptions::builder()
                        .projection(doc! { "_id": 1 })
                        .build(),
                )
                .await
                .ok()
                == Some(None)
            {
                {
                    #![allow(clippy::clone_double_ref)]
                    let is_collection_exists = self
                        .db
                        .list_collection_names(doc! {"name":file_collection.clone()})
                        .await?;
                    if is_collection_exists.is_empty() {
                        self.db
                            .create_collection(&file_collection.clone(), None)
                            .await?
                    }

                    let indexes = self
                        .db
                        .run_command(doc! {"listIndexes":file_collection.clone()}, None)
                        .await?;
                    let mut have_index = false;
                    for index in indexes
                        .get_document("cursor")
                        .unwrap()
                        .get_array("firstBatch")
                        .unwrap()
                    {
                        let key = index.as_document().unwrap().get_document("key").unwrap();
                        let filename = key.get_i32("filename");
                        let upload_date = key.get_i32("uploadDate");
                        let filename_f = key.get_f64("filename");
                        let upload_date_f = key.get_f64("uploadDate");

                        match (filename, upload_date, filename_f, upload_date_f) {
                            (Ok(1), Ok(1), _, _) => {
                                have_index = true;
                            }
                            (_, _, Ok(x), Ok(y))
                                if (x - 1.0).abs() < 0.0001 && (y - 1.0).abs() < 0.0001 =>
                            {
                                have_index = true;
                            }
                            (Ok(1), _, _, Ok(x)) if (x - 1.0).abs() < 0.0001 => {
                                have_index = true;
                            }
                            (_, Ok(1), Ok(x), _) if (x - 1.0).abs() < 0.0001 => {
                                have_index = true;
                            }
                            _ => {}
                        }
                    }
                    if !have_index {
                        self.create_files_index(&file_collection).await?;
                    }
                }
                {
                    #![allow(clippy::clone_double_ref)]
                    let is_collection_exists = self
                        .db
                        .list_collection_names(doc! {"name":chunk_collection.clone()})
                        .await?;
                    if is_collection_exists.is_empty() {
                        self.db
                            .create_collection(&chunk_collection.clone(), None)
                            .await?
                    }

                    let indexes = self
                        .db
                        .run_command(doc! {"listIndexes":chunk_collection.clone()}, None)
                        .await?;
                    let mut have_index = false;
                    for index in indexes
                        .get_document("cursor")
                        .unwrap()
                        .get_array("firstBatch")
                        .unwrap()
                    {
                        let key = index.as_document().unwrap().get_document("key").unwrap();
                        let files_id = key.get_i32("files_id");
                        let n = key.get_i32("n");
                        let files_id_f = key.get_f64("files_id");
                        let n_f = key.get_f64("n");

                        match (files_id, n, files_id_f, n_f) {
                            (Ok(1), Ok(1), _, _) => {
                                have_index = true;
                            }
                            (_, _, Ok(x), Ok(y))
                                if (x - 1.0).abs() < 0.0001 && (y - 1.0).abs() < 0.0001 =>
                            {
                                have_index = true;
                            }
                            (Ok(1), _, _, Ok(x)) if (x - 1.0).abs() < 0.0001 => {
                                have_index = true;
                            }
                            (_, Ok(1), Ok(x), _) if (x - 1.0).abs() < 0.0001 => {
                                have_index = true;
                            }
                            _ => {}
                        }
                    }
                    if !have_index {
                        self.create_chunks_index(&chunk_collection).await?;
                    }
                }
            }
            self.never_write = false;
        }
        Ok(())
    }

    /**
      Uploads a user file to a GridFS bucket. The driver generates the file id.

      Reads the contents of the user file from the @source Stream and uploads it
      as chunks in the chunks collection. After all the chunks have been uploaded,
      it creates a files collection document for @filename in the files collection.

      Returns the id of the uploaded file.

      Note: this method is provided for backward compatibility. In languages
      that use generic type parameters, this method may be omitted since
      the TFileId type might not be an ObjectId.
      # Examples
       ```
       # use mongodb::Client;
       # use mongodb::{error::Error, Database};
       use mongodb_gridfs::{options::GridFSBucketOptions, GridFSBucket};
       # use uuid::Uuid;
       #
       # fn db_name_new() -> String {
       #     "test_".to_owned()
       #         + Uuid::new_v4()
       #             .to_hyphenated()
       #             .encode_lower(&mut Uuid::encode_buffer())
       # }
       #
       # #[tokio::main]
       # async fn main() -> Result<(), Error> {
       #    let client = Client::with_uri_str(&std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string())).await?;
       #    let dbname = db_name_new();
       #    let db: Database = client.database(&dbname);
       let mut bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
       let id = bucket
           .upload_from_stream("test.txt", "stream your data here".as_bytes(), None)
           .await?;
       #     println!("{}", id);
       #     db.drop(None).await
       # }
       ```
    */
    pub async fn upload_from_stream<'a> (
        &mut self,
        filename: &str,
        mut source: impl AsyncRead + Unpin,
        options: Option<GridFSUploadOptions>,
    ) -> Result<ObjectId, Error> {
        let dboptions = self.options.clone().unwrap_or_default();
        let mut chunk_size: u32 = dboptions.chunk_size_bytes;
        let bucket_name = dboptions.bucket_name;
        let file_collection = bucket_name.clone() + ".files";
        let disable_md5 = dboptions.disable_md5;
        let chunk_collection = bucket_name + ".chunks";
        let mut progress_tick = None;
        if let Some(options) = options.clone() {
            if let Some(chunk_size_bytes) = options.chunk_size_bytes {
                chunk_size = chunk_size_bytes;
            }
            progress_tick = options.progress_tick;
        }
        let files = self.db.collection(&file_collection);

        self.ensure_file_index(&files, &file_collection, &chunk_collection)
            .await?;

        let mut file_document = doc! {"filename":filename,
        "chunkSize":chunk_size};
        if let Some(options) = options {
            if let Some(metadata) = options.metadata {
                file_document.insert("metadata", metadata);
            }
        }
        let mut insert_option = InsertOneOptions::default();
        if let Some(write_concern) = dboptions.write_concern.clone() {
            insert_option.write_concern = Some(write_concern);
        }
        let insert_file_result = files
            .insert_one(file_document, Some(insert_option.clone()))
            .await?;

        let files_id = insert_file_result.inserted_id.as_object_id().unwrap();

        let mut md5 = Md5::default();
        let chunks = self.db.collection(&chunk_collection);
        let mut vecbuf: Vec<u8> = vec![0; chunk_size as usize];
        let mut length: usize = 0;
        let mut n: u32 = 0;
        loop {
            let buffer = vecbuf.as_mut_slice();
            let read_size = source.read(buffer).await?;
            if read_size == 0 {
                break;
            }
            let mut bin: Vec<u8> = Vec::from(buffer);
            bin.resize(read_size, 0);
            md5.update(&bin);
            chunks
                .insert_one(
                    doc! {"files_id":files_id,
                    "n":n,
                    "data": bson::Binary{subtype: bson::spec::BinarySubtype::Generic, bytes:bin}},
                    Some(insert_option.clone()),
                )
                .await?;
            length += read_size;
            n += 1;
            if let Some(ref progress_tick) = progress_tick {
                progress_tick.update(length);
            };
        }

        let mut update = doc! { "length": length as i64, "uploadDate": Utc::now() };
        if !disable_md5 {
            update.insert("md5", format!("{:02x}", md5.finalize()));
        }
        let mut update_option = UpdateOptions::default();
        if let Some(write_concern) = dboptions.write_concern {
            update_option.write_concern = Some(write_concern);
        }
        files
            .update_one(
                doc! {"_id":files_id},
                doc! {"$set":update},
                Some(update_option),
            )
            .await?;

        Ok(files_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::GridFSBucket;
    use crate::options::GridFSBucketOptions;
    use bson::{doc, Document};
    use mongodb::Client;
    use mongodb::{error::Error, Database};
    use futures::StreamExt;
    use uuid::Uuid;
    fn db_name_new() -> String {
        "test_".to_owned()
            + Uuid::new_v4()
                .to_hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
    }

    #[tokio::test]
    async fn upload_from_stream() -> Result<(), Error> {
        let client = Client::with_uri_str(
            &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
        )
        .await?;
        let dbname = db_name_new();
        let db: Database = client.database(&dbname);
        let mut bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
        let id = bucket
            .upload_from_stream("test.txt", "test data".as_bytes(), None)
            .await?;

        assert_eq!(id.to_hex(), id.to_hex());
        let file = db
            .collection::<Document>("fs.files")
            .find_one(doc! { "_id": id.clone() }, None)
            .await?
            .unwrap();
        assert_eq!(file.get_str("filename").unwrap(), "test.txt");
        assert_eq!(file.get_i32("chunkSize").unwrap(), 261120);
        assert_eq!(file.get_i64("length").unwrap(), 9);
        assert_eq!(
            file.get_str("md5").unwrap(),
            "eb733a00c0c9d336e65691a37ab54293"
        );

        let chunks: Vec<Result<Document, Error>> = db
            .collection("fs.chunks")
            .find(doc! { "files_id": id }, None)
            .await?
            .collect()
            .await;

        assert_eq!(chunks[0].as_ref().unwrap().get_i32("n").unwrap(), 0);
        assert_eq!(
            chunks[0]
                .as_ref()
                .unwrap()
                .get_binary_generic("data")
                .unwrap(),
            &vec![116 as u8, 101, 115, 116, 32, 100, 97, 116, 97]
        );

        db.drop(None).await
        //Ok(())
    }

    #[tokio::test]
    async fn upload_from_stream_chunk_size() -> Result<(), Error> {
        let client = Client::with_uri_str(
            &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
        )
        .await?;
        let dbname = db_name_new();
        let db: Database = client.database(&dbname);
        let mut bucket = GridFSBucket::new(
            db.clone(),
            Some(GridFSBucketOptions::builder().chunk_size_bytes(8).build()),
        );
        let id = bucket
            .upload_from_stream("test.txt", "test data 1234567890".as_bytes(), None)
            .await?;

        assert_eq!(id.to_hex(), id.to_hex());
        let file = db
            .collection::<Document>("fs.files")
            .find_one(doc! { "_id": id.clone() }, None)
            .await?
            .unwrap();
        assert_eq!(file.get_str("filename").unwrap(), "test.txt");
        assert_eq!(file.get_i32("chunkSize").unwrap(), 8);
        assert_eq!(file.get_i64("length").unwrap(), 20);
        assert_eq!(
            file.get_str("md5").unwrap(),
            "5e75d6271a7cfc3d9b79116be261eb21"
        );

        let chunks: Vec<Result<Document, Error>> = db
            .collection::<Document>("fs.chunks")
            .find(doc! { "files_id": id }, None)
            .await?
            .collect()
            .await;

        assert_eq!(chunks[0].as_ref().unwrap().get_i32("n").unwrap(), 0);
        assert_eq!(
            chunks[0]
                .as_ref()
                .unwrap()
                .get_binary_generic("data")
                .unwrap(),
            &vec![116 as u8, 101, 115, 116, 32, 100, 97, 116]
        );

        assert_eq!(chunks[1].as_ref().unwrap().get_i32("n").unwrap(), 1);
        assert_eq!(
            chunks[1]
                .as_ref()
                .unwrap()
                .get_binary_generic("data")
                .unwrap(),
            &vec![97 as u8, 32, 49, 50, 51, 52, 53, 54]
        );

        assert_eq!(chunks[2].as_ref().unwrap().get_i32("n").unwrap(), 2);
        assert_eq!(
            chunks[2]
                .as_ref()
                .unwrap()
                .get_binary_generic("data")
                .unwrap(),
            &vec![55 as u8, 56, 57, 48]
        );

        db.drop(None).await
        // Ok(())
    }

    #[tokio::test]
    async fn ensure_files_index_before_write() -> Result<(), Error> {
        let client = Client::with_uri_str(
            &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
        )
        .await?;
        let dbname = db_name_new();
        let db: Database = client.database(&dbname);
        let mut bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));

        let indexes = db
            .run_command(doc! {"listIndexes":"fs.files"}, None)
            .await
            .ok();

        assert_eq!(indexes, None, "No index expected");

        bucket
            .upload_from_stream("test.txt", "test data".as_bytes(), None)
            .await?;

        let indexes = db
            .run_command(doc! {"listIndexes":"fs.files"}, None)
            .await?;

        let mut have_index = false;
        for index in indexes
            .get_document("cursor")
            .unwrap()
            .get_array("firstBatch")
            .unwrap()
        {
            let key = index.as_document().unwrap().get_document("key").unwrap();
            let filename = key.get_i32("filename");
            let upload_date = key.get_i32("uploadDate");
            let filename_f = key.get_f64("filename");
            let upload_date_f = key.get_f64("uploadDate");

            match (filename, upload_date, filename_f, upload_date_f) {
                (Ok(1), Ok(1), _, _) => {
                    have_index = true;
                }
                (_, _, Ok(x), Ok(y)) if x == 1.0 && y == 1.0 => {
                    have_index = true;
                }
                (Ok(1), _, _, Ok(x)) if x == 1.0 => {
                    have_index = true;
                }
                (_, Ok(1), Ok(x), _) if x == 1.0 => {
                    have_index = true;
                }
                _ => {}
            }
        }

        assert_eq!(have_index, true, "should found a file index");

        db.drop(None).await
        // Ok(())
    }

    #[tokio::test]
    async fn ensure_chunks_index_before_write() -> Result<(), Error> {
        let client = Client::with_uri_str(
            &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
        )
        .await?;
        let dbname = db_name_new();
        let db: Database = client.database(&dbname);
        let mut bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));

        let indexes = db
            .run_command(doc! {"listIndexes":"fs.chunks"}, None)
            .await
            .ok();

        assert_eq!(indexes, None, "No index expected");

        bucket
            .upload_from_stream("test.txt", "test data".as_bytes(), None)
            .await?;

        let indexes = db
            .run_command(doc! {"listIndexes":"fs.chunks"}, None)
            .await?;

        let mut have_chunks_index = false;
        for index in indexes
            .get_document("cursor")
            .unwrap()
            .get_array("firstBatch")
            .unwrap()
        {
            let key = index.as_document().unwrap().get_document("key").unwrap();

            let files_id = key.get_i32("files_id");
            let n = key.get_i32("n");
            let files_id_f = key.get_f64("files_id");
            let n_f = key.get_f64("n");

            match (files_id, n, files_id_f, n_f) {
                (Ok(1), Ok(1), _, _) => {
                    have_chunks_index = true;
                }
                (_, _, Ok(x), Ok(y)) if x == 1.0 && y == 1.0 => {
                    have_chunks_index = true;
                }
                (Ok(1), _, _, Ok(x)) if x == 1.0 => {
                    have_chunks_index = true;
                }
                (_, Ok(1), Ok(x), _) if x == 1.0 => {
                    have_chunks_index = true;
                }
                _ => {}
            }
        }
        assert_eq!(have_chunks_index, true, "should found a chunk index");
        db.drop(None).await
        // Ok(())
    }
}
