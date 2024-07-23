use async_trait::async_trait;

use log::error;
use pingora::prelude::*;
use pingora::protocols::http::ServerSession;
use pingora::protocols::Stream;
use pingora::server::ShutdownWatch;
use pingora::services::listening::Service;
use std::sync::Arc;

mod body;
mod mime;
mod open_file;

pub use open_file::Root;

use open_file::*;

pub type WebServer = Service<WebService>;

pub fn new_web_server(root: &str) -> WebServer {
    let service = Arc::new(WebService::new(root));
    Service::new("Pingora Web".to_string(), service)
}

pub struct WebService {
    root: Root,
}

#[async_trait]
impl pingora::apps::HttpServerApp for WebService {
    async fn process_new_http(
        self: &Arc<Self>,
        mut session: ServerSession,
        // TODO: handle shutdown
        _shutdown: &ShutdownWatch,
    ) -> Option<Stream> {
        // just report error and do not reuse connection.
        self.do_process_request(&mut session)
            .await
            .map_err(|e| {
                error!("{e}");
                e
            })
            .ok()?;
        session.finish().await.ok().flatten()
    }
}

impl WebService {
    pub fn new(root: &str) -> Self {
        WebService {
            root: Root::new(root),
        }
    }

    async fn do_process_request(self: &Arc<Self>, session: &mut ServerSession) -> Result<()> {
        let exited = !session.read_request().await?;
        if exited {
            return Ok(());
        }
        let req = session.req_header();
        // TODO: handle req body
        let path = req.uri.path();
        // TODO: the following functions have blocking file read IO operations, move then to threads
        let file_result = self.root.file_path(path);
        let (resp, maybe_body) = response_header(file_result);
        session.write_response_header(Box::new(resp)).await?;

        // return response body
        // TOOD: deal with range here
        if let Some((file, meta)) = maybe_body {
            let mut body_reader = body::BodyReader::new(file, meta.len());
            while let Some(data) = body_reader.read(65536)? {
                session.write_response_body(data).await?;
            }
        }

        Ok(())
    }
}
