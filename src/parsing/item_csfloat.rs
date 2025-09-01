use serde_json::Value;

use crate::models::{price::Doppler, web::ExtraItemData};

// Må få tak i doppler/phase, market, price
pub fn parse_iteminfo_min(data: &Value, item_name: Option<&str>) -> Result<ExtraItemData, String> {
    let name  = match item_name {
        Some(name) => { name.to_string() },
        None => {
            data.get("full_item_name")
                .and_then(|n| n.as_str() )
                .map( str::to_owned )
                .ok_or_else(|| "full_item_name NOT FOUND")?
        }
    };

    let float = {
        let tmp = data.get("floatvalue")
            .and_then(|f| f.as_f64() )
            .ok_or_else(|| "floatvalue NOT FOUND")?;

        if tmp == 0.0 { None } else { Some(tmp) }
    };
    
    // let max_float = {
        // let tmp = data.get("max")
            // .and_then(|m| m.as_f64() )
            // .unwrap_or( 1.0 );

        // if tmp == 0.0 { None } else { Some(tmp) }
    // };
 
    // let min_float = {
        // let tmp = data.get("min")
            // .and_then(|m| m.as_f64() )
            // .unwrap_or( 0.0 );

        // if tmp == 0.0 && float.is_none() { None } else { Some(tmp) }
    // };

    let phase = {
        let tmp = data.get("paintindex")
            .and_then(|p| p.as_f64() )
            .map(|p| p as u16)
            .ok_or_else(|| "paintindex NOT FOUND")?;

        if let Some(dplr) = Doppler::is_doppler(tmp) && name.to_lowercase().contains("doppler") { Some(dplr) } else { None }
    };

    let paintseed = {
        let tmp = data.get("paintseed")
            .and_then(|p| p.as_f64() )
            .map(|p| p as u16)
        .ok_or_else(|| "paintseed NOT FOUND")?;
    
        if tmp == 0 { None } else { Some(tmp) }
    };

    Ok( ExtraItemData { /*name,*/ float, /*max_float, min_float,*/ phase, paintseed } )
}