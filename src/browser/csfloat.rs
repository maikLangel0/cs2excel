use std::{error::Error, time::Duration};

use reqwest::{header::{self, HeaderValue}, Client};
use serde_json::Value;

use crate::models::web::{InspectLinks, ExtraItemData, CSFLOAT_HEADERS_DEFAULT, PHASES};


pub async fn fetch_iteminfo(
    client: &Client, 
    inspect_link: &str
) -> Result<Option<Value>, String> {
    let url_base = "https://api.csfloat.com/?url=";
    let url = format!("{}{}", url_base, inspect_link);

    println!("Curr url: {}", url);

    let response = client.get(url)
        // .headers( CSFLOAT_HEADERS_DEFAULT.clone() )
        .headers( CSFLOAT_HEADERS_DEFAULT.clone() )
        .send()
        .await.map_err(|_| "woopsie")?;

    if !response.status().is_success() { 
        println!("\n\nFAILED RESPONSE HEADER: {:?}\n", response.headers()); 
        
        return Err( 
            format!("GET Request failed! {} Response text: {:#?}", 
                &response.status(), 
                &response.text()
                    .await
                    .map_err(|_| String::from("Should never happen"))? 
            ).into() 
        ) 
    }
    
    let json_obj: Value = serde_json::from_str( 
        &response.text()
            .await.map_err(|e| format!("Could not turn json into text wat {}.",e))? 
    ).map_err(|_| format!("Could not turn text into serde json value what."))?;
    
    let iteminfo: Value = json_obj.get("iteminfo")
        .unwrap_or( &Value::Null )
        .to_owned();

    Ok( if iteminfo.is_null() {None} else {Some(iteminfo)} )
}

pub async fn fetch_iteminfo_persistent(
    client: &mut Client, 
    inspect_link: &str, 
    max_retries: u8, 
    pause_time_millis: u64
) -> Result<Option<Value>, String> {
    
    tokio::time::sleep( Duration::from_millis(pause_time_millis) ).await;
    let mut attempt = 1;

    loop {
        match fetch_iteminfo(client, inspect_link).await {
            Ok(json) => { break Ok(json) }
            Err(e) => {
                if attempt >= max_retries { break Err( "Exhausted all retries".into() )}
                println!("Error in single_fetch_request_persistent: {:?}",e);
                
                tokio::time::sleep( 
                    Duration::from_millis( 
                        pause_time_millis * (attempt * 2 - attempt) as u64 
                    ) 
                ).await;
                
                attempt += 1;
            }
        }
    }
}

pub fn new_extra_iteminfo_client() -> reqwest::Client {
    Client::builder()
        .default_headers( CSFLOAT_HEADERS_DEFAULT.clone() )
        .brotli(true)
        .build()
        .expect("Build of csfloat client failed")
}

// DOENST WORK WITHOUT BOT SET UP (saj)
pub async fn batched_float_request(
    client: &Client, 
    inspect_links: &InspectLinks, 
    key: &'static str
) -> Result<Value, Box<dyn Error>> {
    let url_base = "https://api.csfloat.com/bulk";
    
    let response = client.post(url_base)
        .headers( CSFLOAT_HEADERS_DEFAULT.clone() )
        .header ( header::CONTENT_TYPE, HeaderValue::from_static("application/json, text/plain, */*") )
        .header ( header::AUTHORIZATION, HeaderValue::from_static( key ))
        .json   ( inspect_links )
        .send()
        .await?;

    if !response.status().is_success() { 
        println!("\n\nFAILED RESPONSE HEADER: {:?}\n", response.headers()); 
        
        return Err( 
            format!("GET Request failed! {} Response text: {:#?}", 
                &response.status(),
                &response.text()
                    .await
                    .map_err(|_| String::from("Should never happen"))? 
            ).into() 
        ) 
    }

    let json_obj: Value = serde_json::from_str( &response.text().await? )?;
    Ok( json_obj )
}