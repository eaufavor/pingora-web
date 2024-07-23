use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;

/// Lookup file MIME type based on file extension
/// source https://svn.apache.org/repos/asf/httpd/httpd/trunk/docs/conf/mime.types
/// or https://github.com/nginx/nginx/blob/master/conf/mime.types
// TODO: Make this confiurable
// TODO: use phf or something to make it more efficient
static MIME_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    // TODO: finish this list
    [
        // text
        ("html", "text/html"),
        ("htm", "text/html"),
        ("shtml", "text/html"),
        ("css", "text/css"),
        ("xml", "text/xml"),
        ("txt", "text/plain"),
        // media
        ("gif", "image/gif"),
        ("jpeg", "image/jpeg"),
        ("jpg", "image/jpeg"),
        ("png", "image/png"),
        ("webp", "image/webp"),
        // js
        ("js", "application/javascript"),
        // font
        ("woff", "font/woff"),
        ("woff2", "font/woff2"),
    ]
    .iter()
    .copied()
    .collect()
});

pub(crate) fn mime_type_lookup(path: &Path) -> Option<&'static str> {
    path.extension().and_then(|ext| {
        let ext = ext.to_ascii_lowercase();
        MIME_MAP.get(ext.to_str()?).copied()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_positive() {
        let path = Path::new("/tmp/1.html");
        assert_eq!(mime_type_lookup(&path), Some("text/html"));
    }

    #[test]
    fn lookup_negative() {
        let path = Path::new("/tmp/1.lol");
        assert_eq!(mime_type_lookup(&path), None);
    }

    #[test]
    fn lookup_case() {
        let path = Path::new("/tmp/1.HTmL");
        assert_eq!(mime_type_lookup(&path), Some("text/html"));
    }

    #[test]
    fn lookup_no_ext() {
        let path = Path::new("/tmp/just_file");
        assert_eq!(mime_type_lookup(&path), None);
    }
}
