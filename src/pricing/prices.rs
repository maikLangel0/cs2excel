use std::collections::HashMap;
use serde_json::Value;
use strum::IntoEnumIterator;

use crate::{
    browser::csgotrader, 
    models::{
        web::{Sites, SteamData}, 
        price::{PricingProvider, PriceType}, 
        user_sheet::UserInfo,
        item::ItemData
    }
};

use super::price_csgotrader;

//pub async fn get_prices(user: &UserInfo, cs_inventory: &Vec<SteamData>, rate: f64) -> Result<Vec<ItemData>, String> {
pub async fn get_prices(user: &UserInfo, cs_inventory: &Vec<SteamData>, rate: f64) -> Result<HashMap<String, ItemData>, String> {
    // Initialization woohoo
    let mut market_prices: Value;
    let mut itemdata: HashMap<String, ItemData> = HashMap::new();
    //let mut itemdata: Vec<ItemData> = Vec::new();
    let mut all_market_prices: HashMap<String, Value> = HashMap::new();
    
    // Formaterer alle markeder til å være Vec<Sites> i stedet for å måtte bruke Sites::iter() 1/2 av gangene
    let markets_to_check: Vec<Sites> = if let Some(markets) = &user.prefer_markets { markets.clone() } 
    else { Sites::iter().collect::<Vec<Sites>>() };

    // Henter all informasjonen om prisene for markedene og legger det i et HashMap<markedsnavn, alle priser>
    for market in &markets_to_check {
        
        match user.pricing_provider {
            PricingProvider::Csgoskins => { market_prices = csgotrader::get_market_data( market ).await? }, // IF I IMPLEMENT CSGOSKINS IN THE FUTURE
            PricingProvider::Csgotrader => { market_prices = csgotrader::get_market_data( market ).await? }
        }

        all_market_prices.insert(market.to_string(), market_prices);
    }
    
    // For vært item i inventory så lages det en ny hashmap egnet for <markedsnavn, pris>
    for item in cs_inventory {
        let mut prices: HashMap<String, f64> = HashMap::new();

        // Hver iterasjon hentes prisen til item'et gitt navnet, dataen om selve markedet sine priser, og hvilken pristype man velger
        for market in &markets_to_check {
            if let Some(market_prices) = all_market_prices.get( market.as_str() ) {
                if let Some(price) = price_csgotrader::get_price(&item.name, &market_prices, market, &PriceType::StartingAt, &None) {
                    prices.insert(market.to_string(), price * rate);
                }
            }
        }

        // itemdata.push( 
            // ItemData {
                // name: item.name.clone(),
                // asset_id: item.asset_id,
                // price: prices,
                // inspect_link: item.inspect_link.clone(),
                // quantity: item.quantity,
            // } 
        // );

        itemdata.insert(
            item.name.clone(), 
            ItemData { 
                asset_id: item.asset_id, 
                price: prices, 
                inspect_link: item.inspect_link.clone(), 
                quantity: item.quantity 
            }
        );
    };

    Ok(itemdata)
}

pub async fn get_itemdata(user: &UserInfo, steamdata: &SteamData, rate: f64, /* IF FIRST TIME FETCHING, GET FLOAT AND MAYBE ADDITIONAL PRICE */) -> Result<ItemData, String> {
    let mut market_prices: Value;
    let mut all_market_prices: HashMap<String, Value> = HashMap::new();
    
    let markets_to_check: Vec<Sites> = if let Some(markets) = &user.prefer_markets { markets.clone() } 
    else { Sites::iter().collect::<Vec<Sites>>() };

    // Henter all informasjonen om prisene for markedene og legger det i et HashMap<markedsnavn, alle priser>
    for market in &markets_to_check {
        
        match user.pricing_provider {
            PricingProvider::Csgoskins => { market_prices = csgotrader::get_market_data( market ).await? }, // IF I IMPLEMENT CSGOSKINS IN THE FUTURE
            PricingProvider::Csgotrader => { market_prices = csgotrader::get_market_data( market ).await? }
        }

        all_market_prices.insert(market.to_string(), market_prices);
    }

    Ok(ItemData { asset_id: todo!(), price: todo!(), inspect_link: todo!(), quantity: todo!()  })
}