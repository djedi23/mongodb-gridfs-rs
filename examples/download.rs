use futures::stream::StreamExt;
use mongodb::Client;
use mongodb::Database;
use mongodb_gridfs::{options::GridFSBucketOptions, GridFSBucket, GridFSError};
use uuid::Uuid;

fn db_name_new() -> String {
    "test_".to_owned()
        + Uuid::new_v4()
            .to_hyphenated()
            .encode_lower(&mut Uuid::encode_buffer())
}

#[tokio::main]
async fn main() -> Result<(), GridFSError> {
    let client = Client::with_uri_str(
        &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
    )
    .await?;
    let dbname = db_name_new();
    let db: Database = client.database(&dbname);
    let bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
    let id = bucket
        .clone()
        .upload_from_stream("test.txt", "test data".as_bytes(), None)
        .await?;
    println!("{}", id);

    let mut cursor = bucket.open_download_stream(id).await?;
    let buffer = cursor.next().await.unwrap();
    println!("{:?}", buffer);

    db.drop(None).await?;
    Ok(())
}
