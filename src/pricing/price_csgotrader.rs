use serde_json::Value;

use crate::models::price::{PriceType, Doppler};

pub fn get_price(item_name: &str, prices: &Value, want: &PriceType, phase: &Option<Doppler>) -> Option<f64> {
    if let Some(p_one) = prices.get(item_name) { 
        // If the json is key value pair (youpin)
        if p_one.is_f64() { return p_one.as_f64() }  
        
        // One layer deep
        else if p_one.is_object() { 
            
            // If doppler phase is provided, try and find the price of that phase
            if let Some(doppler_phase) = phase {
                if let Some(p_two) = p_one.get("doppler") { 
                    if let Some(phase_price) = p_two.get( doppler_phase.as_str() ) {
                        return phase_price.as_f64();
                    }   
                }
            }

            if let Some(p_two) = p_one.get("price") { return p_two.as_f64() }
            if let Some(p_two) = p_one.get( want.as_str() ) { 
                if let Some(p_three) = p_two.get("price") { return p_three.as_f64() } 
            }
            
            if let Some(p_two) = p_one.get("starting_at") { return p_two.as_f64() } // skinport

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