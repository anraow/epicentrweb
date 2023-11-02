#![deny(warnings)]
use mongodb::{Client, options::{ClientOptions}};
use handlebars::Handlebars;
use serde_json::json;
use std::{fs};
use std::str::FromStr;
use futures::StreamExt;
use warp::Filter;
use mongodb::{bson::{Document, doc}};
use mongodb::bson::{Array, DateTime};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

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
    let target_id: ObjectId = ObjectId::from_str("64b6d1b17f20fe2884362ec6").expect("Invalid ObjectId");

    let mut cursor = collection.find(doc! { "_id": target_id }, None).await?;
    while let Some(result) = cursor.next().await {
        match result {
            Ok(document) => {
                // Extract fields as needed
                let caption = document.get_str("caption").unwrap_or("N/A").to_string();
                let date = document.get_datetime("date").map(|d| d.to_owned()).unwrap_or_else(|_| DateTime::now());
                let poster = document.get_str("poster").unwrap_or("N/A").to_string();
                let keyboard = document.get_array("keyboard").map(|a| a.to_owned()).unwrap_or_else(|_| Array::new());

                #[derive(Debug, Serialize, Deserialize)]
                struct Event {
                    caption: String,
                    date: DateTime,
                    poster: String,
                    keyboard: Array,
                }

                let event = Event {
                    caption,
                    date,
                    poster,
                    keyboard
                };

                // Print the event using the Debug trait
                println!("{:?}", event);
            }
            Err(e) => eprintln!("Error while iterating over cursor: {}", e),
        }
    }

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
