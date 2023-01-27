use bson::doc;
#[cfg(feature = "async-std-runtime")]
use futures::StreamExt;
use mongodb::{error::Result, Client, Database};
use mongodb_gridfs::{
    bucket::GridFSBucket,
    options::{GridFSBucketOptions, GridFSFindOptions},
};
#[cfg(any(feature = "default", feature = "tokio-runtime"))]
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::with_uri_str(
        &std::env::var("MONGO_URI").unwrap_or_else(|_| "mongodb://localhost:27017/".to_string()),
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
