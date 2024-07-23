use pingora::server::Server;

use pingora_web::new_web_server;

pub fn main() {
    env_logger::init();

    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();

    let mut web_server = new_web_server(&format!("{}/tests/files/", env!("CARGO_MANIFEST_DIR")));

    web_server.add_tcp("0.0.0.0:8000");

    my_server.add_service(web_server);
    my_server.run_forever();
}
