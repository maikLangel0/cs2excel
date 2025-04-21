use std::{collections::HashMap, io::Read};
use reqwest::{header::{self}, Client};
use flate2::read::GzDecoder;
use serde_json::{self, Value};
use crate::models::web::{FIREFOX_CSGOTRADERAPP_HEADERS_DEFAULT, Sites};

//https://github.com/gergelyszabo94/csgo-trader-extension/blob/master/extension/src/utils/pricing.js#L393
pub async fn get_exchange_rates() -> Result<HashMap<String, f64>, String> {
    let client = Client::new();

    // Sending the GET request trying to mimic the one used by the csgotrader.app extension
    let response = client.get( format!("https://prices.csgotrader.app/latest/exchange_rates.json"))
        .headers( FIREFOX_CSGOTRADERAPP_HEADERS_DEFAULT.to_owned() )
        .header( header::HOST, "prices.csgotrader.app" )
        .send()
        .await.map_err(|e| format!("Error sending GET request to the csgotraderapp exchange API. \n{}", e))?;

    if !response.status().is_success() { return Err( format!("GET Request failed! \n{}", response.status()) ) }

    let bytes = response.bytes()
        .await.map_err( |e| format!("Unable to turn http response into bytes. \n{}", e) )?;

    let mut raw_data = String::new();
    GzDecoder::new(&bytes[..])
        .read_to_string(&mut raw_data)
        .map_err(|e| format!("Error decoding the gzipped bytes from the csgotraderapp exchange API. \n{}", e))?;

    let exchange_rates: HashMap<String, f64> = serde_json::from_str(&raw_data)
        .map_err(|e| format!("Parsing the decoded gzip response to hashmap failed. \n{}", e))?;
    
    Ok(exchange_rates)
}

// USD is 1.0
//https://github.com/gergelyszabo94/csgo-trader-extension/blob/master/extension/src/utils/pricing.js#L393
pub async fn get_market_data(market: &Sites) -> Result<Value, String> {
    let client = Client::new();

    // Sending the GET request trying to mimic the one used by the csgotrader.app extension
    let response = client.get( format!("https://prices.csgotrader.app/latest/{}.json", market.as_str() ))
        .headers( FIREFOX_CSGOTRADERAPP_HEADERS_DEFAULT.to_owned() )
        .header( header::ACCEPT_ENCODING, "gzip" )
        .header( header::HOST, "prices.csgotrader.app" )
        .send()
        .await.map_err(|e| format!("Error sending GET request to the csgotraderapp price API. {}", e))?;

    if !response.status().is_success() { return Err( format!("GET Request failed! {}", response.status()) ) }

    let bytes = response.bytes()
        .await.map_err( |e| format!("Unable to turn http response into bytes. {}", e) )?;

    let mut raw_data = String::new();
    GzDecoder::new(&bytes[..])
        .read_to_string(&mut raw_data)
        .map_err(|e| format!("Error decoding the gzipped bytes from the csgotraderapp price API. {}", e))?;

    let prices: Value = serde_json::from_str(&raw_data)         
        .map_err(|e| format!("Parsing the decoded gzip given the market {:?} response to hashmap failed. {}", market, e))?;

    Ok(prices)
}

//
pub async fn get_iteminfo(inspect_link: &String) -> Result<Value, String> {
    let client = Client::new();

    //println!("https://api.csgotrader.app/float?url={}", urlencoding::encode(inspect_link));

    // Sending the GET request trying to mimic the one used by the csgotrader.app extension
    let response = client.get( format!("https://api.csgotrader.app/float?url={}", urlencoding::encode(inspect_link) ))
        .headers( FIREFOX_CSGOTRADERAPP_HEADERS_DEFAULT.to_owned() )
        .header( header::HOST, "api.csgotrader.app" )
        .header( header::ACCEPT_ENCODING, "gzip")
        .send()
        .await.map_err(|e| format!("Error sending GET request to the csgotraderapp price API. {}", e))?;

    if !response.status().is_success() { return Err( format!("GET Request failed! {}", response.status()) ) }

    let bytes = response.bytes()
        .await.map_err( |e| format!("Unable to turn http response into bytes. {}", e) )?;

    let mut raw_data = String::new();
    GzDecoder::new(&bytes[..])
        .read_to_string(&mut raw_data)
        .map_err(|e| format!("Error decoding the gzipped bytes from the csgotraderapp float API. {}", e))?;

    let value: Value = serde_json::from_str(&raw_data)         
        .map_err(|e| format!("Parsing the decoded gzip given the inspect link {:?} response to hashmap failed. {}", inspect_link, e))?;

    let iteminfo = value.get("iteminfo")
        .ok_or( String::from("Couldn't get iteminfo from csgotraderapp float API"))?
        .clone();

    Ok(iteminfo)
}

// --floatapi headers --

// https://api.csgotrader.app/float?url=steam%3A%2F%2Frungame%2F730%2F76561202255233023%2F%2Bcsgo_econ_action_preview%2520S76561198841632579A42955488866D2759832313987649292 <- CONTAINS PHASE ASWELL FAK

// example : https://api.csgotrader.app/float?url=steam%3A%2F%2Frungame%2F730%2F76561202255233023%2F%2Bcsgo_econ_action_preview%2520S76561198389123475A34543022281D9279926981479153949

// GET /float?url=steam%3A%2F%2Frungame%2F730%2F76561202255233023%2F%2Bcsgo_econ_action_preview%2520S76561198389123475A34543022281D9279926981479153949 HTTP/2
// Host: api.csgotrader.app
// User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:137.0) Gecko/20100101 Firefox/137.0
// Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8
// Accept-Language: en-GB,en;q=0.5
// Accept-Encoding: gzip, deflate, br, zstd
// Connection: keep-alive
// Upgrade-Insecure-Requests: 1
// Sec-Fetch-Dest: document
// Sec-Fetch-Mode: navigate
// Sec-Fetch-Site: none
// Sec-Fetch-User: ?1
// Priority: u=0, i
// Pragma: no-cache
// Cache-Control: no-cache