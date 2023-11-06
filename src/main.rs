#![deny(warnings)]
use mongodb::{Client, options::{ClientOptions}};
use handlebars::Handlebars;
use serde_json::json;
use std::{fs};
use std::str::FromStr;
use futures::StreamExt;
use warp::Filter;
use mongodb::{bson::{Document, doc}};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use std::string::String;
use chrono::{NaiveDateTime, Utc};
use chrono::format::strftime::StrftimeItems;


fn load_template(event: Option<&Event>) -> Result<Handlebars<'static>, handlebars::TemplateError> {
    let template_content = fs::read_to_string("./static/index.html")
        .map_err(|err| handlebars::TemplateError::from((std::io::Error::new(std::io::ErrorKind::Other, err), "Failed to read template".to_string())))?;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("my_template", &template_content)?;

    if let Some(event) = event {
        let data = json!({
            "event_caption": &event.caption,
            "event_date": &event.date,
            "event_poster": &event.poster,
            "event_keyboard": &event.keyboard,
        });

        handlebars.register_template_string("my_template", &template_content)?;
        handlebars.render("my_template", &data).expect("Failed to render HTML");
    }

    Ok(handlebars)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Event {
    caption: String,
    date: String,
    poster: String,
    keyboard: String,
}

async fn fetch_event(collection: &mongodb::Collection<Document>, target_id: ObjectId) -> Option<Event> {
    let mut cursor = collection.find(doc! { "_id": target_id }, None).await.ok()?;

    if let Some(result) = cursor.next().await {
        if let Ok(document) = result {
            println!("BSON Document: {:#?}", document);
            let caption = document.get_str("caption").unwrap_or("N/A").to_string();
            let date_str = document
                .get_str("date")
                .unwrap_or_else(|_| "");
            let date = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S%.3fZ").unwrap_or_else(|err| {
                eprintln!("Error parsing date: {}", err);
                Utc::now().naive_utc()
            });
            let formatted_date: String = date.format_with_items(StrftimeItems::new("%d.%m.%y %H:%M"))
                .to_string();
            let poster = document.get_str("poster").unwrap_or("N/A").to_string();
            // Convert the Array of objects to HTML links representation
            let keyboard_vec: Vec<String> = document.get_array("keyboard")
                .map(|a| {
                    a.iter()
                        .filter_map(|bson| {
                            if let Some(obj) = bson.as_document() {
                                let name = obj.get_str("name").unwrap_or("");
                                let url = obj.get_str("url").unwrap_or("");
                                Some(format!("<a href=\"{}\">{}</a>", url, name))
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_else(|_| Vec::new());

            let keyboard = keyboard_vec.join("");
            let keyboard_script = format!(
                r#"
                <script>
                    var container = document.getElementById('eventLinks');
                    container.innerHTML = '{}';
                </script>
                "#,
                keyboard
            );

            return Some(Event {
                caption,
                date: formatted_date,
                poster,
                keyboard: keyboard_script,
            });
        }
    }

    None
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

    let event = fetch_event(&collection, target_id).await;

    let handlebars = load_template(event.as_ref()).expect("Failed to load template");

    // Data to fill in the template
    let dynamic_route = warp::path("home")
        .and(warp::any().map(move || event.clone()))
        .map(move |event: Option<Event>| {
        let data = json!({
            "event_caption": event.as_ref().map_or("", |e| &e.caption),
            "event_date": event.as_ref().map_or("", |e| &e.date),
            "event_poster": event.as_ref().map_or("", |e| &e.poster),
            "event_keyboard": event.as_ref().map_or("", |e| &e.keyboard),
        });

        let html = handlebars.render("my_template", &data).expect("Failed to render HTML");
        warp::reply::html(html)
    });

    let static_route = warp::path("static").and(warp::fs::dir("./static"));

    let routes = static_route.or(dynamic_route);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 8000))
        .await;

    Ok(())
}
