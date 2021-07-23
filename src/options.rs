use bson::Document;
use mongodb::options::{ReadConcern, ReadPreference, WriteConcern};
use std::time::Duration;
use typed_builder::TypedBuilder;

// TODO: rethink the name of the trait
// TODO: move the trait in another file
pub trait ProgressUpdate {
    fn update(&self, position: usize) -> ();
}

/// [Spec](https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst#file-upload)
#[derive(Clone, Default, TypedBuilder)]
pub struct GridFSUploadOptions<'a> {
    /**
     * The number of bytes per chunk of this file. Defaults to the
     * chunkSizeBytes in the GridFSBucketOptions.
     */
    #[builder(default = None)]
    pub(crate) chunk_size_bytes: Option<u32>,

    /**
     * User data for the 'metadata' field of the files collection document.
     * If not provided the driver MUST omit the metadata field from the
     * files collection document.
     */
    #[builder(default = None)]
    pub(crate) metadata: Option<Document>,

    /**
     * DEPRECATED: A valid MIME type. If not provided the driver MUST omit the
     * contentType field from the files collection document.
     *
     * Applications wishing to store a contentType should add a contentType field
     * to the metadata document instead.
     */
    #[builder(default = None)]
    content_type: Option<String>,

    /**
     * DEPRECATED: An array of aliases. If not provided the driver MUST omit the
     * aliases field from the files collection document.
     *
     * Applications wishing to store aliases should add an aliases field to the
     * metadata document instead.
     */
    #[builder(default = None)]
    aliases: Option<Vec<String>>,

    /**
     * TODO: Documentation for progress_tick
     */
    // TODO: find a better name.
    #[builder(default = None)]
    pub(crate) progress_tick: Option<&'a dyn ProgressUpdate>, // TODO: test process_tick
}

/// [Spec](https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst#configurable-gridfsbucket-class)
#[derive(Clone, Debug, TypedBuilder)]
pub struct GridFSBucketOptions {
    /**
     * The bucket name. Defaults to 'fs'.
     */
    #[builder(default = "fs".into())]
    pub bucket_name: String,

    /**
     * The chunk size in bytes. Defaults to 255 KiB.
     */
    #[builder(default = 255 * 1024)]
    pub chunk_size_bytes: u32,

    /**
     * The write concern. Defaults to the write concern of the database.
     */
    #[builder(default)]
    pub write_concern: Option<WriteConcern>,

    /**
     * The read concern. Defaults to the read concern of the database.
     */
    #[builder(default)]
    pub read_concern: Option<ReadConcern>,

    /**
     * The read preference. Defaults to the read preference of the database.
     */
    #[builder(default)]
    pub read_preference: Option<ReadPreference>,

    /**
     * TRANSITIONAL: This option is provided for backwards compatibility.
     * It MUST be supported while a driver supports MD5 and MUST be removed
     * (or made into a no-op) when a driver removes MD5 support entirely.
     * When true, the GridFS implementation will not compute MD5 checksums
     * of uploaded files. Defaults to false.
     */
    #[builder(default = false)]
    pub disable_md5: bool,
}

impl Default for GridFSBucketOptions {
    fn default() -> Self {
        GridFSBucketOptions {
            bucket_name: "fs".into(),
            chunk_size_bytes: 255 * 1024,
            write_concern: None,
            read_concern: None,
            read_preference: None,
            disable_md5: false,
        }
    }
}

/// [Spec](https://github.com/mongodb/specifications/blob/master/source/gridfs/gridfs-spec.rst#generic-find-on-files-collection)
#[derive(Clone, Debug, Default, TypedBuilder)]
pub struct GridFSFindOptions {
    /**
     * Enables writing to temporary files on the server. When set to true, the server
     * can write temporary data to disk while executing the find operation on the files collection.
     *
     * This option is sent only if the caller explicitly provides a value. The default
     * is to not send a value. For servers < 3.2, this option is ignored and not sent
     * as allowDiskUse does not exist in the OP_QUERY wire protocol.
     *
     * @see https://docs.mongodb.com/manual/reference/command/find/
     */
    #[builder(default)]
    pub allow_disk_use: Option<bool>,

    /**
     * The number of documents to return per batch.
     */
    #[builder(default)]
    pub batch_size: Option<u32>,

    /**
     * The maximum number of documents to return.
     */
    #[builder(default)]
    pub limit: Option<i64>,

    /**
     * The maximum amount of time to allow the query to run.
     */
    #[builder(default)]
    pub max_time: Option<Duration>,

    /**
     * The server normally times out idle cursors after an inactivity period (10 minutes)
     * to prevent excess memory use. Set this option to prevent that.
     */
    #[builder(default)]
    pub no_cursor_timeout: Option<bool>,

    /**
     * The number of documents to skip before returning.
     */
    #[builder(default)]
    pub skip: i64,

    /**
     * The order by which to sort results. Defaults to not sorting.
     */
    #[builder(default)]
    pub sort: Option<Document>,
}

#[cfg(test)]
mod tests {
    use super::{GridFSBucketOptions, GridFSFindOptions};

    #[test]
    fn grid_fs_bucket_options_default() {
        let options = GridFSBucketOptions::default();
        assert_eq!(options.bucket_name, "fs");
        assert_eq!(options.chunk_size_bytes, 255 * 1024);
        assert_eq!(options.disable_md5, false);
    }
    #[test]
    fn grid_fs_bucket_options_builder_default() {
        let options = GridFSBucketOptions::builder().build();
        assert_eq!(options.bucket_name, "fs");
        assert_eq!(options.chunk_size_bytes, 255 * 1024);
        assert_eq!(options.disable_md5, false);
    }
    #[test]
    fn grid_fs_bucket_options_bucket_name() {
        let options = GridFSBucketOptions::builder()
            .bucket_name("newfs".into())
            .build();
        assert_eq!(options.bucket_name, "newfs");
    }
    #[test]
    fn grid_fs_bucket_options_chunk_size_bytes() {
        let options = GridFSBucketOptions::builder()
            .chunk_size_bytes(1023)
            .build();
        assert_eq!(options.chunk_size_bytes, 1023);
    }
    #[test]
    fn grid_fs_bucket_options_builder_chain() {
        let options = GridFSBucketOptions::builder()
            .bucket_name("newfs".into())
            .chunk_size_bytes(1023)
            .build();
        assert_eq!(options.bucket_name, "newfs");
        assert_eq!(options.chunk_size_bytes, 1023);
    }

    #[test]
    fn grid_fs_find_options_builder_chain() {
        let options = GridFSFindOptions::builder().skip(4).build();
        assert_eq!(options.skip, 4);
    }
    #[test]
    fn grid_fs_find_options_builder_default() {
        let options = GridFSFindOptions::builder().build();
        assert_eq!(options.allow_disk_use, None);
        assert_eq!(options.batch_size, None);
        assert_eq!(options.limit, None);
        assert_eq!(options.max_time, None);
        assert_eq!(options.no_cursor_timeout, None);
        assert_eq!(options.skip, 0);
        assert_eq!(options.sort, None);
    }
    #[test]
    fn grid_fs_find_options_default() {
        let options = GridFSFindOptions::default();
        assert_eq!(options.allow_disk_use, None);
        assert_eq!(options.batch_size, None);
        assert_eq!(options.limit, None);
        assert_eq!(options.max_time, None);
        assert_eq!(options.no_cursor_timeout, None);
        assert_eq!(options.skip, 0);
        assert_eq!(options.sort, None);
    }
}
