#[macro_use]
extern crate failure;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate lazy_static;

mod error;
mod model;
mod highlighter;
mod stats;

use actix::{prelude::*, Actor};
use actix_web::{web::{self, Data, Path}, App, HttpServer, HttpResponse};
use tera::{Tera, Context};
use model::World;
use error::BlogError;
use stats::{StatisticsServer, system_stats, GetInitialValues};

fn create_context(stats: Data<Addr<StatisticsServer>>) -> Context {
    let mut ctx = tera::Context::new();
    ctx.insert("websocket", &std::env::var("BLOG_WEBSOCKET").unwrap_or("ws://localhost:8080".into()));
    if let Ok(stats) = stats.send(GetInitialValues {}).wait() {
        if let Ok(values) = stats {
            ctx.insert("stats", &values);
        }
    }

    ctx
}

fn index(world: Data<World>, tera: Data<Tera>, stats: Data<Addr<StatisticsServer>>) -> Result<HttpResponse, BlogError> {
    let mut ctx = create_context(stats);
    ctx.insert("articles", &world.articles);
    let body = tera.render("frontpage.tera", &ctx)?;

    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

fn single_article(world: Data<World>, tera: Data<Tera>, stats: Data<Addr<StatisticsServer>>, slug: Path<String>) 
    -> Result<HttpResponse, BlogError> 
{
    let article = world.find_by_slug(&slug)?;
    let mut ctx = create_context(stats);
    ctx.insert("article", &article);

    let body = tera.render("single-article.tera", &ctx)?;

    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

fn main() -> std::io::Result<()> {
    let _sys = actix::System::new("system");

    let stats_server = StatisticsServer::default().start();

    HttpServer::new(move || {
        let mut tera = Tera::new("resources/templates/**/*")
            .expect("failed to initialize templates");
        
        tera.register_function("highlight", Box::new(highlighter::highlight));
        tera.register_filter("codeblock", highlighter::codeblock);

        let world = World::new(&tera, include_str!("../resources/articles.json"));

        App::new()
            .data(stats_server.clone())
            .data(world)
            .data(tera)
            .wrap(actix_web::middleware::Logger::default())
            .default_service(
                web::resource("/").to(index)
            )
            .service(
                actix_files::Files::new("/static", "resources/static")
            )
            .service(
                web::resource("/statistics").to(system_stats)
            )
            .service(
                web::resource("/articles/{slug}").to(single_article)
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
}
