use std::str::FromStr;

use serde_json::Value;

use crate::models::{price::{Doppler, PriceType}, web::{ExtraItemData, Sites, SteamData}};

pub fn get_price(item_name: &str, prices: &Value, market: &Sites, want: &PriceType, phase: &Option<Doppler>) -> Option<f64> {
    if let Some(p_one) = prices.get(item_name) { 
        // If the json is key value pair (youpin) | doesnt have doppler prices
        if p_one.is_f64() { return p_one.as_f64() }  
        
        // One layer deep
        else if p_one.is_object() { 
            
            // If doppler phase is provided, try and find the price of that phase
            if let Some(dp_price) = doppler_price(p_one, phase, item_name, market) { return Some(dp_price) }
            
            if let Some(p_two) = p_one.get("price") { return p_two.as_f64() }
            if let Some(p_two) = p_one.get( want.as_str() ) { 
                if p_two.is_f64() { return p_two.as_f64() } // skinport

                if let Some(dp_price) = doppler_price(
                    p_two, phase, item_name, market
                ) { return Some(dp_price) }   // buff163 doppler price

                if let Some(p_three) = p_two.get("price") { return p_three.as_f64() } 
            }

            let steam_prices: Vec<f64> = Vec::from( [p_one.get("last_24h"),  p_one.get("last_7d"), p_one.get("last_30d"), p_one.get("last_90d")] )
                .iter()
                .filter_map(|p| *p)
                .filter_map( |p| p.as_f64() )
                .collect::<Vec<f64>>();

            if !steam_prices.is_empty() { return Some( steam_prices[0] ) } // For steam, always default to most recent price available
            
        }
    } 
    None
}

fn doppler_price(p: &Value, phase: &Option<Doppler>, item_name: &str, market: &Sites) -> Option<f64> {
    if let Some(doppler_phase) = phase {
        if let Some(p_two) = p.get("doppler") { 
            if let Some(phase_price) = p_two.get( doppler_phase.as_str() ) {
                return phase_price.as_f64();
            }
            else { 
                println!("NOTE: Doppler of type {} found but did not have active price for item {} on the site {}", 
                    doppler_phase.as_str(), 
                    item_name, market.as_str() 
                ); 
            } 
        }
    }
    None
}

pub fn parse_iteminfo_min(data: &Value, steamdata: &SteamData) -> Result<ExtraItemData, String> {
    let name = steamdata.name.clone();

    let float = {
        let tmp = data.get("floatvalue")
            .and_then(|f| f.as_f64() )
            .ok_or_else(|| "floatvalue NOT FOUND")?;

        if tmp == 0.0 { None } else { Some(tmp) }
    };

    let max_float = {
        let tmp = data.get("max")
            .and_then(|m| m.as_f64() )
            .unwrap_or( 1.0 );

        if tmp == 0.0 { None } else { Some(tmp) }
    };

    let min_float = {
        let tmp = data.get("min")
            .and_then(|m| m.as_f64() )
            .unwrap_or( 0.0 );

        if tmp == 0.0 && float.is_none() { None } else { Some(tmp) }
    };

    let phase = if let Some(dplr) = data.get("phase") { 
        Some( Doppler::from_str( 
            dplr.as_str().ok_or_else(|| "Dplr to string didnt work what.".to_string())? 
        )? ) 
    } else { None };

    let paintseed = {
        let tmp = data.get("paintseed")
            .and_then(|p| p.as_f64() )
            .map(|p| p as u16)
        .ok_or_else(|| "paintseed NOT FOUND")?;
    
        if tmp == 0 { None } else { Some(tmp) }
    };

    Ok( ExtraItemData { name, float, max_float, min_float, phase, paintseed } )
}