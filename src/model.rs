use tera::{Tera, Context};
use crate::error::BlogError;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Author<'a> {
	pub name: &'a str,
	pub email: Option<&'a str>
}

#[derive(Serialize, Deserialize)]
pub struct Article<'a> {
	pub title: &'a str,
	pub author: Author<'a>,
	pub published: u64,
	pub body: String,
	pub slug: String,
}

#[derive(Serialize, Deserialize)]
pub struct ArticleMetaData<'a> {
	pub title: &'a str,
	pub author: Author<'a>,
	pub published: u64,
	pub template: &'a str,
	pub slug: Option<&'a str>,
}

pub struct World<'a> {
	pub articles: Vec<Article<'a>>,
}

impl<'a> World<'a> {
	pub fn new(tera: &tera::Tera, articles_json: &'a str) -> World<'a> {
		let metadata: Vec<ArticleMetaData<'a>> = serde_json::from_str(articles_json)
			.expect("failed to load article metadata");

		let articles = metadata.iter()
			.map(|article| {
				println!("{:?}", tera);

				let body = tera.render(&format!("articles/{}", &article.template), Context::new())
					.expect(&format!("unable to load article template {}", &article.template));


				// Unless a slug is explicitly specified, generate one using tera's slugify functionality
				// based on the article's title
				let slug = match article.slug {
					Some(slug) => slug.into(),
					None => {
						let mut ctx = Context::new();
						ctx.insert(&"title", &article.title);
						Tera::one_off("{{ title | slugify }}", ctx, false)
							.expect(&format!("unable to generate slug for article {}", &article.title))
					}
				};

				Article {
					title: article.title,
					author: article.author,
					published: article.published,
					body,
					slug
				}
			})
			.collect();

		World {
			articles
		}
	}

	pub fn find_by_slug(&'a self, slug: &str) -> Result<&'a Article<'a>, BlogError> {
		let article = self.articles.iter().find(|&article| {
			article.slug == slug
		});

		match article {
			Some(article) => Ok(article),
			None => Err(BlogError::MissingContent(format!("no article with that name found")))
		}
	}

}