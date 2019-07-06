#[macro_use]
extern crate failure;

#[macro_use]
extern crate serde;

use actix_web::{web, App, HttpResponse, HttpServer};
use tera::{Context, Tera};

mod error;
mod highlighter;
mod model;

use error::BlogError;
use model::World;

fn index(tera: web::Data<Tera>, articles: web::Data<World>) -> Result<HttpResponse, BlogError> {
    let mut ctx = Context::new();
    ctx.insert("articles", &articles.articles);

    let body = tera.render("frontpage.tera", ctx)?;

    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

fn single_article(
    tera: web::Data<Tera>,
    world: web::Data<World>,
    name: web::Path<String>,
) -> Result<HttpResponse, BlogError> {
    let article = world.find_by_slug(&name)?;

    let mut ctx = Context::new();
    ctx.insert("article", &article);

    let body = tera.render("single-article.tera", ctx)?;

    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

fn main() -> std::io::Result<()> {
    dotenv::dotenv().expect("dotenv file not found");
    env_logger::init();

    HttpServer::new(move || {
        let mut tera =
            Tera::parse("templates/**/*.tera").expect("failed to initialize tera templates");
        tera.register_filter("highlight", highlighter::highlight);
        tera.register_filter("codeblock", highlighter::codeblock);
        tera.build_inheritance_chains()
            .expect("failed to build tera inheritance chains");
        tera.check_macro_files()
            .expect("failed to check tera macro files");

        let world = World::new(&tera, include_str!("../articles.json"));

        App::new()
            .data(world)
            .data(tera)
            .wrap(actix_web::middleware::Compress::default())
            .wrap(actix_web::middleware::Logger::default())
            .default_service(web::resource("/").to_async(index))
            .service(web::resource("/{name}").to(single_article))
    })
    .bind("0.0.0.0:8080")?
    .run()
}
