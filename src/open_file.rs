use http::header::*;
use pingora::http::ResponseHeader;
use pingora::prelude::*;
use std::fs::{File, Metadata};
use std::path::Path;

use super::mime;

pub const ERR_NO_ACCESS: ErrorType = ErrorType::new("ERR_NO_ACCESS");

pub struct Root {
    pub(crate) root: String,
}

impl Root {
    pub fn new(path: &str) -> Self {
        Root {
            root: path.to_owned(),
        }
    }

    // Check if the path is under the root. `/../some_file` is considered invalidate
    fn validate(&self, path: &Path) -> bool {
        // TODO: other possible cases?
        // TODO: Resolve the path without doing IO. Canonicalize() tries to access the file even when the
        // file is not suppose to be accessed. NOTE: std::path::absolute doesn't resolve `..` so don't use it. 
        // FIXME: canonicalize() also follows symlink, which causes false negative.
        let Ok(abs_path) = std::fs::canonicalize(path) else {
            // non exist, no access or anything else
            return false;
        };
        abs_path.starts_with(&self.root)
    }

    /// Return the opened file, the metadata (to generate headers like Content-Length and Last-Modified) and the full path
    ///
    /// `uri_path` is required to start with `/` as it is how HTTP URI passes the path
    pub fn file_path(&self, uri_path: &str) -> Result<Option<(File, Metadata, String)>> {
        // use str concat not PathBuf::push() because push() allows escaping the root
        // TODO: !IMPORTANT! we should avoid input with ".." being used to escape root
        assert!(uri_path.starts_with('/'));
        let path_str = self.root.clone() + uri_path;
        let path = Path::new(&path_str);
        if !self.validate(path) {
            return Ok(None); // or 403??
        }
        dbg!(&uri_path);
        dbg!(&path_str);
        dbg!(&path);
        if !path.exists() {
            return Ok(None);
        }

        // NOTE: path.is_file() will follow symlink
        if !path.is_file() {
            return Error::e_explain(ERR_NO_ACCESS, format!("path {path_str}"));
        }

        let meta = path
            .metadata()
            .or_err_with(ERR_NO_ACCESS, || format!("path {path_str}"))?;

        // Already checked path.exist(), so the error here won't be because of non-existing files
        let file = File::open(path).or_err_with(ERR_NO_ACCESS, || format!("path {path_str}"))?;

        Ok(Some((file, meta, path_str)))
    }
}

// return the response header from the result of accesing the file. And a boolean indicating whether there will be body
pub(crate) fn response_header(
    result: Result<Option<(File, Metadata, String)>>,
) -> (ResponseHeader, Option<(File, Metadata)>) {
    // 4: reserve space for 4 headers: Server, Content-Length, Cache-Control, Content-Type
    const RESERVED_HEADERS_SIZE: Option<usize> = Some(4);

    match result {
        Ok(Some((file, meta, path))) => {
            let mut header = ResponseHeader::build(200, RESERVED_HEADERS_SIZE)
                .expect("200 is a valid status code");

            // Content-Length
            header
                .insert_header(CONTENT_LENGTH, format!("{}", meta.len()))
                .expect("meta.len() u64 to string should contain only valid bytes");

            // Content-Type
            let path = Path::new(&path);
            if let Some(mime) = mime::mime_type_lookup(&path) {
                header
                    .insert_header(CONTENT_TYPE, mime)
                    .expect("mime type should contain only valid bytes");
            }

            // Last-Modified
            if let Ok(modified) = meta.modified() {
                // Datetime in http header should be UTC
                // https://www.rfc-editor.org/rfc/rfc9110#field.last-modified
                let datetime = chrono::DateTime::<chrono::Utc>::from(modified);
                header
                    .insert_header(
                        LAST_MODIFIED,
                        datetime.format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
                    )
                    .expect("time string are valid bytes");
            }

            (header, Some((file, meta)))
        }
        // File not found
        Ok(None) => {
            // TODO: support error page with body
            let mut header = ResponseHeader::build(404, RESERVED_HEADERS_SIZE)
                .expect("404 is a valid status code");
            header
                .insert_header(http::header::CONTENT_LENGTH, &b"0"[..])
                .expect("0 is valid");
            (header, None)
        }
        // Error opening the file
        Err(e) => {
            // TODO: support error page with body
            match e.etype() {
                &ERR_NO_ACCESS => {
                    let mut header = ResponseHeader::build(403, RESERVED_HEADERS_SIZE)
                        .expect("403 is a valid status code");
                    header
                        .insert_header(CONTENT_LENGTH, &b"0"[..])
                        .expect("0 is valid");
                    (header, None)
                }
                // all other errors are 500 for now
                _ => {
                    let mut header = ResponseHeader::build(500, RESERVED_HEADERS_SIZE)
                        .expect("500 is a valid status code");
                    header
                        .insert_header(CONTENT_LENGTH, &b"0"[..])
                        .expect("0 is valid");
                    (header, None)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_file() {
        let root = format!("{}/tests/files", env!("CARGO_MANIFEST_DIR"));
        let root = Root { root };

        let file = root.file_path("/test1").unwrap();

        assert!(file.is_none());
    }

    #[test]
    fn test_no_dir() {
        let root = format!("{}/tests/files", env!("CARGO_MANIFEST_DIR"));
        let root = Root { root };

        let file = root.file_path("/test_dir/test1").unwrap();

        assert!(file.is_none());
    }

    #[test]
    fn test_file() {
        let root = format!("{}/tests/files", env!("CARGO_MANIFEST_DIR"));
        let root = Root { root };

        let (_file, meta, path) = root.file_path("/test.html").unwrap().unwrap();

        assert!(meta.is_file());
        assert_eq!(
            path,
            format!("{}/tests/files/test.html", env!("CARGO_MANIFEST_DIR"))
        );
    }

    #[test]
    fn test_response_headers() {
        let root = format!("{}/tests/files", env!("CARGO_MANIFEST_DIR"));
        let root = Root { root };

        let opened = root.file_path("/test.html");

        let (resp, has_body) = response_header(opened);

        assert_eq!(resp.status, 200);
        assert_eq!(resp.headers.get(CONTENT_LENGTH).unwrap(), "183");
        assert_eq!(resp.headers.get(CONTENT_TYPE).unwrap(), "text/html");
        assert!(resp.headers.get(LAST_MODIFIED).is_some());
        assert!(has_body.is_some());
    }

    #[test]
    fn test_response_no_file() {
        let root = format!("{}/tests/files", env!("CARGO_MANIFEST_DIR"));
        let root = Root { root };

        let opened = root.file_path("/test.no");

        let (resp, has_body) = response_header(opened);

        assert_eq!(resp.status, 404);
        assert_eq!(resp.headers.get(CONTENT_LENGTH).unwrap(), "0");
        assert!(has_body.is_none());
    }

    #[test]
    fn test_response_no_access() {
        let root = format!("{}/tests/files", env!("CARGO_MANIFEST_DIR"));
        let root = Root { root };

        let opened = root.file_path("/test.no");

        let (resp, has_body) = response_header(opened);

        assert_eq!(resp.status, 404);
        assert_eq!(resp.headers.get(CONTENT_LENGTH).unwrap(), "0");
        assert!(has_body.is_none());
    }
}
