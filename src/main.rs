use ab_glyph::{FontArc, PxScale};
use actix_web::{
    get,
    web::{Bytes, Query, ServiceConfig},
    HttpResponse,
};
use image::{ImageBuffer, Rgb};
use imageproc::drawing::draw_text_mut;
use reqwest::Client;
use serde_json::Value;
use shuttle_actix_web::ShuttleActixWeb;
use std::collections::HashMap;
use textwrap::{wrap, Options, WordSplitter};

#[get("/")]
async fn joke_image(Query(params): Query<HashMap<String, String>>) -> HttpResponse {
    // Fetch joke from API
    let client = Client::new();
    let url = params.iter().fold(
        "https://v2.jokeapi.dev/joke/Any?".to_string(),
        |mut url, (key, value)| {
            if url.chars().last().unwrap() != '?' {
                url.push_str("&");
            }

            url.push_str(&format!("{}={}", key, value));
            url
        },
    );

    let (joke, category) = match client.get(&url).send().await {
        Ok(res) => match res.json::<Value>().await {
            Ok(data) => {
                let joke_text = if data["type"].as_str() == Some("twopart") {
                    format!(
                        "{}\n\n{}",
                        data["setup"].as_str().unwrap_or("No setup found"),
                        data["delivery"].as_str().unwrap_or("No delivery found")
                    )
                } else {
                    data["joke"].as_str().unwrap_or("No joke found").to_string()
                };
                let category = data["category"].as_str().unwrap_or("Unknown").to_string();
                (joke_text, category)
            }
            Err(_) => ("Failed to parse joke".to_string(), "Unknown".to_string()),
        },
        Err(_) => ("Failed to fetch joke".to_string(), "Unknown".to_string()),
    };

    // Load font
    let font_data = include_bytes!("fonts/DejaVuSans.ttf");
    let font = FontArc::try_from_slice(font_data).unwrap();

    // Calculate text dimensions
    let scale = PxScale::from(25.0);
    let max_width = 550.0 - 75.0; // Account for margins
    let char_width = scale.x * 0.6; // Approximate character width
    let max_chars = (max_width / char_width) as usize;

    let wrapped_text = wrap(
        &joke,
        Options::new(max_chars).word_splitter(WordSplitter::NoHyphenation),
    )
    .join("\n");

    // Calculate required image height based on text
    let line_height = (scale.y * 1.2) as i32;
    let text_height = (wrapped_text.lines().count() as i32) * line_height;
    let padding = 100; // Top and bottom padding
    let height = (text_height + padding).max(200); // Minimum height of 200

    // Create image with calculated dimensions
    let width = 550;
    let mut img = ImageBuffer::from_pixel(width, height as u32, Rgb([30u8, 30u8, 30u8]));

    // Draw category tag in upper-right corner
    let tag_scale = PxScale::from(15.0);
    let tag_padding = 5;
    let tag_text = format!(" {} ", category);

    // Calculate tag dimensions
    let tag_width = (tag_text.len() as f32) * (tag_scale.x * 0.6);
    let tag_height = tag_scale.y;

    // Define colors for different categories
    let (bg_color, text_color) = match category.as_str() {
        "Programming" => (Rgb([41, 128, 185]), Rgb([255, 255, 255])),
        "Misc" => (Rgb([46, 204, 113]), Rgb([255, 255, 255])),
        "Dark" => (Rgb([44, 62, 80]), Rgb([255, 255, 255])),
        "Pun" => (Rgb([155, 89, 182]), Rgb([255, 255, 255])),
        "Spooky" => (Rgb([231, 76, 60]), Rgb([255, 255, 255])),
        "Christmas" => (Rgb([192, 57, 43]), Rgb([255, 255, 255])),
        _ => (Rgb([52, 73, 94]), Rgb([255, 255, 255])),
    };

    // Draw tag background
    let tag_x = width as i32 - tag_width as i32 - 50 - tag_padding * 2;
    let tag_y = 30;
    for y in tag_y..(tag_y + tag_height as i32 + tag_padding * 2) {
        for x in tag_x..(tag_x + tag_width as i32 + tag_padding * 2) {
            img.put_pixel(x as u32, y as u32, bg_color);
        }
    }

    // Draw tag text
    draw_text_mut(
        &mut img,
        text_color,
        tag_x + tag_padding,
        tag_y + tag_padding,
        tag_scale,
        &font,
        &tag_text,
    );

    // Draw main text on image with light color
    let color = Rgb([255u8, 255u8, 255u8]); // White text for dark mode
    let mut y = 50;
    for line in wrapped_text.lines() {
        draw_text_mut(&mut img, color, 50, y, scale, &font, line);
        y += line_height;
    }

    // Draw credit text in bottom-left corner
    let credit_scale = PxScale::from(13.0);
    let credit_text = "Thanks, JokeAPI (https://v2.jokeapi.dev)";

    // Calculate credit position
    let credit_x = 50; // Start from left edge with padding
    let credit_y = height as i32 - credit_scale.y as i32 - 15;

    // Draw credit text
    draw_text_mut(
        &mut img,
        Rgb([200u8, 200u8, 200u8]), // Light gray color
        credit_x,
        credit_y,
        credit_scale,
        &font,
        credit_text,
    );

    // Convert image to bytes
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();

    HttpResponse::Ok()
        .content_type("image/png")
        .insert_header(("Cache-Control", "no-store, no-cache, must-revalidate"))
        .insert_header(("Pragma", "no-cache"))
        .insert_header(("Expires", "0"))
        .body(Bytes::from(buf.into_inner()))
}

#[shuttle_runtime::main]
async fn main() -> ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> {
    let config = move |cfg: &mut ServiceConfig| {
        cfg.service(joke_image);
    };

    Ok(config.into())
}
