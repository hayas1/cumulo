use std::path::PathBuf;

use axum::Router;
use tokio::task::JoinHandle;
use tower_http::services::{ServeDir, ServeFile};

pub(crate) struct Site {
    base_url: String,
    _server: JoinHandle<()>,
}

impl Site {
    pub(crate) async fn serve() -> Site {
        let dist = Self::dist();
        let index = dist.join("index.html");
        let service = ServeDir::new(dist).not_found_service(ServeFile::new(index));
        let app = Router::new().fallback_service(service);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Site {
            base_url: format!("http://{addr}"),
            _server: server,
        }
    }

    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    fn dist() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../cumulo-web/dist"))
    }
}
