use bson::doc;
use futures::stream::StreamExt;
use mongodb::error::Result;
use mongodb::Client;
use mongodb::Database;
use mongodb_gridfs::options::GridFSBucketOptions;
use mongodb_gridfs::{bucket::GridFSBucket, options::GridFSFindOptions};
#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::with_uri_str(
        &std::env::var("MONGO_URI").unwrap_or("mongodb://localhost:27017/".to_string()),
    )
    .await?;
    let db: Database = client.database("test");
    let bucket = &GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
    let mut cursor = bucket
        .find(doc! {"filename":"test.txt"}, GridFSFindOptions::default())
        .await?;

    while let Some(_doc) = cursor.next().await {
        // ...
    }
    Ok(())
}
