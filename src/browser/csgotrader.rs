use std::{collections::HashMap, io::Read};
use reqwest::{header::{self, HeaderMap, HeaderValue}, Client};
use flate2::read::GzDecoder;
use serde_json::{self, Value};
use crate::models::web::{Sites, FIREFOX_CSGOTRADERAPP_HEADERS_BASE, FIREFOX_CSGOTRADERAPP_HEADERS_DEFAULT, FIREFOX_USER_AGENTS};

// USD is 1.0
pub async fn get_exchange_rates() -> Result<HashMap<String, f64>, String> {
    let client = Client::new();

    // Sending the GET request trying to mimic the one used by the csgotrader.app extension
    let response = client.get( format!("https://prices.csgotrader.app/latest/exchange_rates.json"))
        .headers( FIREFOX_CSGOTRADERAPP_HEADERS_BASE.to_owned() )
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

//https://github.com/gergelyszabo94/csgo-trader-extension/blob/master/extension/src/utils/pricing.js#L393
pub async fn get_market_data(market: &Sites) -> Result<Value, String> {
    let client = Client::new();

    // Sending the GET request trying to mimic the one used by the csgotrader.app extension
    let response = client.get( format!("https://prices.csgotrader.app/latest/{}.json", market.as_str() ))
        .headers( FIREFOX_CSGOTRADERAPP_HEADERS_BASE.to_owned() )
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
pub async fn get_iteminfo(client: &Client, inspect_link: &String) -> Result<Value, String> {
    //println!("https://api.csgotrader.app/float?url={}", urlencoding::encode(inspect_link));

    // Sending the GET request trying to mimic the one used by the csgotrader.app extension
    let response = client.get( format!("https://api.csgotrader.app/float?url={}", urlencoding::encode(inspect_link) ))
        .send()
        .await.map_err(|e| format!("Error sending GET request to the csgotraderapp price API. {}", e))?;

    if !response.status().is_success() { 
        //println!("{:?}", response.headers()); 
        return Err( format!("GET Request failed! {}", response.status()) ) 
    }

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

pub fn new_extra_iteminfo_client() -> reqwest::Client {
    let mut headers: HeaderMap = HeaderMap::new();
    let user_agent = FIREFOX_USER_AGENTS[ rand::random_range( 0..FIREFOX_USER_AGENTS.len() )];
    headers.insert( header::USER_AGENT, HeaderValue::from_static( user_agent ) );
    headers.insert(header::HOST, HeaderValue::from_static( "api.csgotrader.app" ) );

    println!("curr user_agent: {}", user_agent);

    reqwest::Client::builder()
        .default_headers( FIREFOX_CSGOTRADERAPP_HEADERS_DEFAULT.clone() )
        .default_headers( headers )
        .build()
        .expect("DEFAULT CLIENT BUILDER FAILED")
}

// --floatapi headers --

// https://api.csgotrader.app/float?url=steam%3A%2F%2Frungame%2F730%2F76561202255233023%2F%2Bcsgo_econ_action_preview%2520S76561198841632579A42955488866D2759832313987649292 <- CONTAINS PHASE ASWELL FAK

// example : https://api.csgotrader.app/float?url=steam%3A%2F%2Frungame%2F730%2F76561202255233023%2F%2Bcsgo_econ_action_preview%2520S76561198389123475A34543022281D9279926981479153949

/*
GOATA CONVO MED MR GIPITI OM KORDAN DET KAN HENDE "items" SER UT "yippi"! 
Hold et øye i tilfelle Gery migrate float fetchinga fra å være singular instance til en batch med float request
https://chatgpt.com/c/68073977-5bd8-800c-8635-781f63c90db4

https://github.com/gergelyszabo94/csgo-trader-extension/blob/3537e40de84be8f0b836ccd2c3e1005f9245b7a6/extension/src/utils/utilsModular.js#L1155

const loadFloatData = (items, ownerID, isOwn, type) => new Promise((resolve, reject) => {
  const trimmedItemProperties = [];

  items.forEach((item) => {
    if (item.inspectLink !== undefined
      && item.inspectLink !== null) {
      trimmedItemProperties.push({
        assetid: item.assetid,
        classid: item.classid,
        instanceid: item.instanceid,
        inspectLink: item.inspectLink,
        name: item.name,
        market_name: item.market_hash_name,
      });
    }
  });

  const getFloatsRequest = new Request('https://api.csgotrader.app/getFloats', {
    method: 'POST',
    body: JSON.stringify({
      items: trimmedItemProperties,
      isOwn,
      ownerID,
      type,
    }),
  });

  fetch(getFloatsRequest)
    .then((response) => {
      if (!response.ok) {
        const errorMessage = `Error code: ${response.status} Status: ${response.statusText}`;
        console.log(errorMessage);
        reject(errorMessage);
      } else return response.json();
    }).then((body) => {
      if (body.status) {
        resolve(body.floatData);
      } else {
        console.log(body);
        reject(body);
      }
    }).catch((err) => {
      console.log(err);
      reject(err);
    });
});

EXAMPLE OF CALL:
const loadFloatData = (items, ownerID,           isOwn,                               type       )
      loadFloatData(   items, request.inventory, steamIDOfUser === request.inventory, 'inventory'  ).then((itemsWithFloats)
*/