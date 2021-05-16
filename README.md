This crate provides an implementation of Mongo GridFS on the top of mongodb's crate.
This implementation only use the _async/await_ version of mongodb. 

From https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst
> GridFS is a convention drivers use to store and retrieve BSON binary data (type “\x05”) that exceeds MongoDB’s BSON-document size limit of 16 MiB. When this data, called a user file, is written to the system, GridFS divides the file into chunks that are stored as distinct documents in a chunks collection. To retrieve a stored file, GridFS locates and returns all of its component chunks. Internally, GridFS creates a files collection document for each stored file. Files collection documents hold information about stored files, and they are stored in a files collection.
# Examples
Uploading a document:
 ```rust
 use mongodb_gridfs::{options::GridFSBucketOptions, GridFSBucket};
 let bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
 let id = bucket
     .upload_from_stream("test.txt", "stream your data here".as_bytes(), None)
     .await?;
 ```
 Downloading a document:
 ```rust
use futures::stream::StreamExt;
use mongodb_gridfs::{options::GridFSBucketOptions, GridFSBucket, GridFSError};

let bucket = GridFSBucket::new(db.clone(), Some(GridFSBucketOptions::default()));
let mut cursor = bucket.open_download_stream(id).await?;
let buffer = cursor.next().await.unwrap();
 ```

# Features
| Feature                                     | Status | Notes                                           |
| ------------------------------------------- | ------ | ----------------------------------------------- |
| GridFSUploadOptions                         | DONE   | `contentType` and `aliases` are not implemented |
| GridFSBucketOption                          | DONE   | concerns not used when ensuring indexes         |
| GridFSFindOptions                           | DONE   |                                                 |
| GridFSDownloadByNameOptions                 | TODO   |                                                 |
| GridFSBucket                                | DONE   |                                                 |
| GridFSBucket . open_upload_stream           | DONE   |                                                 |
| GridFSBucket . open_upload_stream_with_id   |        |                                                 |
| GridFSBucket . upload_from_stream           | NO     | No Implementation planned                       |
| GridFSBucket . upload_from_stream_with_id   | NO     | No Implementation planned                       |
| GridFSBucket . open_download_stream         | DONE   |                                                 |
| GridFSBucket . download_to_stream           | NO     | No Implementation planned                       |
| GridFSBucket . delete                       | DONE   |                                                 |
| GridFSBucket . find                         | DONE   |                                                 |
| GridFSBucket . rename                       | DONE   |                                                 |
| GridFSBucket . drop                         | DONE   | no `DropCollectionOptions` used during the drop |
| GridFSBucket . open_download_stream_by_name |        |                                                 |
| GridFSBucket . download_to_stream_by_name   |        |                                                 |
| indexes                                     | DONE   |                                                 |

