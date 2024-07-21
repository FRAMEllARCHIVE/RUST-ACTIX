use actix_files::NamedFile;
use actix_web::{get, post, App, HttpResponse, HttpServer, Responder, Result};
use std::path::PathBuf;
use std::io;

#[get("/")]
async fn index() -> Result<NamedFile> {
    let path: PathBuf = "./static/index.html".parse()?;
    Ok(NamedFile::open(path)?)
}

#[post("/hello")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("hello there")
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(hello)
    })
    .bind("127.0.0.1:8888")?
    .run()
    .await
}