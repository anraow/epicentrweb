#![deny(warnings)]
use mongodb::{Client, options::{ClientOptions}};
use handlebars::Handlebars;
use serde_json::json;
use std::{fs};
use warp::Filter;
use mongodb::{bson::{Document, doc}};

fn load_template() -> Result<Handlebars<'static>, handlebars::TemplateError> {
    let template_content = fs::read_to_string("./static/index.html")
        .map_err(|err| handlebars::TemplateError::from((std::io::Error::new(std::io::ErrorKind::Other, err), "Failed to read template".to_string())))?;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("my_template", &template_content)?;

    Ok(handlebars)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    // MONGODB CONNECTION IS HERE
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017").await?;

    // Manually set an option.
    client_options.app_name = Some("epicentrweb".to_string());

    // Get a handle to the deployment.
    let client = Client::with_options(client_options)?;

    // Get a handle to a database.
    let db = client.database("bot");

    // Get a handle to the "users" collection.
    let collection = db.collection::<Document>("users");

    let user = collection.find_one(
        doc! {
            "name": "anraow",
        },
        None
    ).await;



    let handlebars = load_template().expect("Failed to load template");

    let static_route = warp::path("static").and(warp::fs::dir("./static"));
    // Data to fill in the template
    let dynamic_route = warp::path("home").map(move || {
        let data = json!({
            "link_name": "Tg"
        });

        let html = handlebars.render("my_template", &data).expect("Failed to render HTML");
        warp::reply::html(html)
    });

    let routes = static_route.or(dynamic_route);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 8000))
        .await;

    Ok(())
}
