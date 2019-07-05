#[macro_use]
extern crate failure;

#[macro_use]
extern crate serde;

use tera::{Tera, Context};
use actix_web::{App, HttpServer, HttpResponse, web};

mod model;
mod error;
mod highlighter;

use model::{World};
use error::BlogError;

fn index(tera: web::Data<Tera>, articles: web::Data<World>) -> Result<HttpResponse, BlogError> {
    let mut ctx = Context::new();
    ctx.insert("articles", &articles.articles);

    let body = tera.render("frontpage.tera", &ctx)?;

    Ok(HttpResponse::Ok().body(body))
}

fn single_article(tera: web::Data<Tera>, world: web::Data<World>, name: web::Path<String>) -> Result<HttpResponse, BlogError> {
    let article = world.find_by_slug(&name)?;

    let mut ctx = Context::new();
    ctx.insert("article", &article);

    let body = tera.render("single-article.tera", &ctx)?;
    
    Ok(HttpResponse::Ok().body(body))
}

fn main() -> std::io::Result<()> {
    dotenv::dotenv().expect("dotenv file not found");
    env_logger::init();

    HttpServer::new(move || {
        let mut tera = Tera::parse("templates/**/*.tera")
            .expect("failed to initialize tera templates");
        tera.register_filter("highlight", highlighter::highlight);
        tera.register_filter("codeblock", highlighter::codeblock);
        tera.build_inheritance_chains()
            .expect("failed to initialize tera templates");

        let world = World::new(&tera, include_str!("../articles.json"));

        App::new()
            .data(world)
            .data(tera)
            .wrap(actix_web::middleware::Compress::default())
            .wrap(actix_web::middleware::Logger::default())
            .default_service(
                web::resource("/").to_async(index)
            )
            .service(
                web::resource("/{name}").to(single_article)
            )
    })
    .bind("0.0.0.0:8080")?
    .run()
}