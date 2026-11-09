use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let addr = ("127.0.0.1", 8080);
    println!("GarryNet backend listening on http://{}:{}", addr.0, addr.1);

    HttpServer::new(|| App::new()).bind(addr)?.run().await
}
