use mongodb::{error::Error, Client, Database};
use mongodb_gridfs::{options::GridFSBucketOptions, GridFSBucket};
use uuid::Uuid;

fn db_name_new() -> String {
    "test_".to_owned()
        + Uuid::new_v4()
            .to_hyphenated()
            .encode_lower(&mut Uuid::encode_buffer())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client = Client::with_uri_str(
        &std::env::var("MONGO_URI").unwrap_or_else(|_| "mongodb://localhost:27017/".to_string()),
    )
    .await?;
    let dbname = db_name_new();
    let db: Database = client.database(&dbname);
    let mut bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
    let id = bucket
        .upload_from_stream("test.txt", "test data".as_bytes(), None)
        .await?;
    println!("{}", id);

    db.drop(None).await
}
