use std::collections::HashMap;
use serde_json::Value;

use reqwest::Client;
use umya_spreadsheet::Worksheet;

use crate::{
    browser::{cookies::FirefoxDb, csfloat, csgotrader},
    models::{excel::ExcelData, price::{Currencies, Doppler, PriceType, PricingMode}, user_sheet::{SheetInfo, UserInfo}, web::{ExtraItemData, ItemInfoProvider, Sites, SteamData, SITE_HAS_DOPPLER}}, 
    parsing::{self, csgoskins_url, item_csgotrader, market_name_parse}
};

pub fn get_steamloginsecure(sls: &Option<String>) -> Option<String> {
    if let Some(sls) = sls { 
        Some(sls.to_string())
    } else { 
        if let Ok(db) = FirefoxDb::init() {
            match db.get_cookies(vec!["name", "value"], "steamcommunity.com", vec!["steamLoginSecure"]) {
                Ok(cookie) => Some(cookie),
                Err(e) => { println!("FRICK.\n{}", e); None }
            }      
        } else { println!("WARNING: Failed to connect to firefox."); None }
    }
}

pub async fn get_exchange_rate(
    usd_to_x: &Currencies, 
    rowcol_usd_to_x: &Option<String>, 
    sheet: &mut Worksheet
) -> Result<f64, String> {
    
    debug_assert!( !( usd_to_x != &Currencies::None && rowcol_usd_to_x.is_some() ) );

    if usd_to_x != &Currencies::None {
        if usd_to_x == &Currencies::USD { return Ok(1.0); }
        let rates: HashMap<String, f64> = csgotrader::get_exchange_rates().await?;
        Ok( *rates.get( usd_to_x.as_str() ).unwrap_or( &1.0 ) )
    } else { 
        
        if let Some(cell) = rowcol_usd_to_x {
            let res = sheet.get_cell_value( cell.as_ref() )
                .get_raw_value()
                .to_string()
                .trim()
                .to_string();
            
            if res.is_empty() { Err( String::from("usd_to_x cell is empty!") ) }
            else {
                Ok(
                    res.parse::<f64>()
                        .map_err(|_| String::from(
                            "usd_to_x cell was not able to be converted to a number!"
                        ) 
                    )?
                )
            }
        } else { Ok(1.0) }
    }
}

pub fn get_market_price(
    user: &UserInfo,
    markets_to_check: &Vec<Sites>,
    all_market_prices: &HashMap<String, Value>,
    rate: f64,
    item_name: &String,
    phase: &Option<String>,
    doppler: &Option<Doppler>
) -> Result<(Option<String>, Option<f64>), String> {
    if !user.fetch_prices { Ok((None, None)) } 
    else {
        #[derive(Clone, Copy)]
        struct MarketPrice { market: &'static str, price: f64 }
        
        let mut prices: Vec<MarketPrice> = Vec::new();
        
        // Finds the prices for each market
        for market in markets_to_check {
            // If site does not have doppler pricings AND doppler is something, SKIP
            if phase.is_some() && !*SITE_HAS_DOPPLER.get(market)
                .ok_or_else(|| "Didnt find market in SITE_HAS_DOPPLER what?")? { continue; }
            
            if let Some(market_prices) = all_market_prices.get( market.as_str() ) {
                if let Some(price) = item_csgotrader::get_price(
                    item_name, 
                    market_prices, 
                    market, 
                    &PriceType::StartingAt, 
                    doppler
                ) { prices.push( MarketPrice { market: market.as_str(), price: price * rate } ) }    
            }
        }
        if prices.is_empty() { Ok((Some("No Market(s) Found".to_string()), None)) } 
        else {
            match user.pricing_mode {
                PricingMode::Cheapest => { 
                    prices.sort_by(|a,b| a.price.partial_cmp(&b.price).unwrap());
                    Ok((Some(prices[0].market.to_string()), Some(prices[0].price)))
                },
                PricingMode::MostExpensive => { 
                    prices.sort_by(|a,b| b.price.partial_cmp(&a.price).unwrap());
                    Ok((Some(prices[0].market.to_string()), Some(prices[0].price)))
                },                          
                PricingMode::Random => {
                    let wiener = prices.get( rand::random_range(0..prices.len()) )
                        .ok_or_else(|| "PricingMode::Random failed what.")
                        .map(|mp| *mp)?;
                    Ok((Some(wiener.market.to_string()), Some(wiener.price)))
                },
                PricingMode::Hierarchical => { 
                    prices.sort_by(|a,b| a.price.partial_cmp(&b.price).unwrap());
                    let mut curr = MarketPrice { market: prices[0].market, price: prices[0].price };
                    for mp in prices.iter().skip(1) {
                        if curr.price > mp.price * user.percent_threshold as f64 
                        { curr = *mp } else { break }
                    }
                    Ok((Some(curr.market.to_string()), Some(curr.price)) )
                }
            }
        }
    }
}

pub async fn fetch_iteminfo_via_itemprovider_persistent(
    client: &mut Client,
    col_inspect_link: &Option<String>,
    iteminfo_provider: &ItemInfoProvider,
    inspect_link: &Option<String>,
    pause_time_ms: u16
) -> Result<Option<Value>, String> {
    
    if col_inspect_link.is_some() {
        if let Some(inspect) = inspect_link {
            match iteminfo_provider {
                ItemInfoProvider::Csfloat => {
                    let tmp = csfloat::fetch_iteminfo_persistent(client, inspect, 10, pause_time_ms as u64).await?;
                    Ok(tmp)
                }
                ItemInfoProvider::Csgotrader => {
                    let tmp = csgotrader::fetch_iteminfo_persistent(client, inspect, 10, pause_time_ms as u64).await?;
                    Ok(tmp)
                }
                ItemInfoProvider::None => { Ok(None) }
            }
            
        } else { Ok(None) }
    } else { Ok(None) }
}

pub async fn wrapper_fetch_iteminfo_via_itemprovider_persistent(
    client: &mut Client,
    iteminfo_provider: &ItemInfoProvider,
    col_inspect_link: &Option<String>,
    pause_time_ms: u16,
    steamdata: &SteamData,
) -> Result<Option<ExtraItemData>, String> {
    
    let json_response = fetch_iteminfo_via_itemprovider_persistent(
        client, 
        col_inspect_link, 
        iteminfo_provider,
        &steamdata.inspect_link, 
        pause_time_ms
    ).await?;
        
    if let Some(json_body) = json_response {
        match iteminfo_provider {
            ItemInfoProvider::Csfloat => { 
                let res = parsing::item_csfloat::parse_iteminfo_min(&json_body, Some(&steamdata.name) )?;
                Ok(Some(res)) 
            },
            ItemInfoProvider::Csgotrader => { 
                let res = parsing::item_csgotrader::parse_iteminfo_min(&json_body, Some(&steamdata.name) )?;
                Ok(Some(res))
            }
            ItemInfoProvider::None => Ok(None)
        }
    } else { Ok(None) }
}

pub async fn insert_new_exceldata(
    user: &UserInfo, 
    excel: &SheetInfo, 
    steamdata: &SteamData, 
    extra_itemdata: &Option<ExtraItemData>,
    markets_to_check: &Vec<Sites>, 
    all_market_prices: &HashMap<String, Value>, 
    rate: f64, 
    row_in_excel: usize,
    sheet: &mut Worksheet
) -> Result<ExcelData, String> {

    let (doppler, phase): (Option<Doppler>, Option<String>) = {

        if let Some(itemdata) = extra_itemdata {
            let doppler = itemdata.phase.clone();
            let phase: Option<String> = doppler.as_ref()
                .and_then( |p| Some( p.as_str() ) )
                .map( str::to_owned );

            (doppler, phase)
        } else { (None, None) }
    };

    let (market, price): (Option<String>, Option<f64>) = get_market_price(
        user, 
        markets_to_check, 
        all_market_prices, 
        rate, 
        &steamdata.name, 
        &phase, 
        &doppler
    )?;

    // Inserting into the spreadsheet
    let cell_steam_name = format!("{}{}", excel.col_steam_name, row_in_excel);
    sheet.get_cell_value_mut(cell_steam_name).set_value_string(&steamdata.name);

    if excel.col_gun_sticker_case.is_some() || excel.col_skin_name.is_some() || excel.col_wear.is_some() {
        let [gun_sticker_case, skin_name, wear] = market_name_parse::metadata_from_market_name(&steamdata.name);

        if let Some(col_gun_sticker_case) = &excel.col_gun_sticker_case {
            if !gun_sticker_case.is_empty() {
                let cell_gsc = format!("{}{}", col_gun_sticker_case, row_in_excel);
                sheet.get_cell_value_mut(cell_gsc).set_value_string(gun_sticker_case);
            }
        }

        if let Some(col_skin_name) = &excel.col_skin_name {
            if !skin_name.is_empty() {
                let cell_sn = format!("{}{}", col_skin_name, row_in_excel);
                sheet.get_cell_value_mut(cell_sn).set_value_string(skin_name);
            }
        }

        if let Some(col_wear) = &excel.col_wear {
            if !wear.is_empty() {
                let cell_wear = format!("{}{}", col_wear, row_in_excel);
                sheet.get_cell_value_mut(cell_wear).set_value_string(wear);
            }
        }
    }

    if let Some(itemdata) = extra_itemdata {
        
        if let Some(col_float) = &excel.col_float {
            if let Some(float) = itemdata.float {
                let cell_float = format!("{}{}", col_float, row_in_excel);
                sheet.get_cell_value_mut(cell_float).set_value_number(float);
            }
        }

        if let Some(col_pattern) = &excel.col_pattern {
            if let Some(pattern) = itemdata.paintseed {
                let cell_pattern = format!("{}{}", col_pattern, row_in_excel);
                sheet.get_cell_value_mut(cell_pattern).set_value_number(pattern);
            }
        }

        if let Some(col_phase) = &excel.col_phase {
            if let Some(faze) = &itemdata.phase {
                let cell_phase = format!("{}{}", col_phase, row_in_excel);
                sheet.get_cell_value_mut(cell_phase).set_value_string(faze.as_str());
            }
        }
    }

    if let Some(col_quantity) = &excel.col_quantity {
        if let Some(quantity) = steamdata.quantity {
            let cell_quantity = format!("{}{}", col_quantity, row_in_excel);
            sheet.get_cell_value_mut(cell_quantity).set_value_number(quantity);
        }
    }

    if let Some(monetary) = price {
        let cell_price = format!("{}{}", &excel.col_price, row_in_excel);
        sheet.get_cell_value_mut(cell_price).set_value_number(monetary);
    }

    if let Some(col_market) = &excel.col_market {
        if let Some(marquet) = market {
            let cell_market = format!("{}{}", col_market, row_in_excel);
            sheet.get_cell_value_mut(cell_market).set_value_string(marquet);
        }
    }

    if let Some(col_inspect_link) = &excel.col_inspect_link {
        if let Some(inspect_link) = &steamdata.inspect_link {
            let cell = format!("{}{}", col_inspect_link, row_in_excel);
            sheet.get_cell_value_mut(cell).set_value_string(inspect_link);
        }
    }

    if let Some(col_asset_id) = &excel.col_asset_id {
        if !user.group_simular_items {
            let cell = format!("{}{}", col_asset_id, row_in_excel);
            sheet.get_cell_value_mut(cell).set_value_number(steamdata.asset_id as f64);
        }
    }

    if let Some(col_csgoskins_link) = &excel.col_csgoskins_link {
        let csgoskins_url = csgoskins_url::create_csgoskins_urls(&steamdata.name);

        let cell = format!("{}{}", col_csgoskins_link, row_in_excel);
        let link = format!("https://csgoskins.gg/items/{}", csgoskins_url);

        sheet.get_cell_value_mut(cell).set_value_string(link);
    }

    Ok(ExcelData { 
        name: steamdata.name.clone(), 
        quantity: steamdata.quantity, 
        phase, 
        // price: price.map_or_else(|| 0.0, |p| p), 
        // inspect_link: steamdata.inspect_link.clone(),
        asset_id: if !user.group_simular_items { Some(steamdata.asset_id) } else { None },
        sold: None
    })
}

pub fn update_quantity_exceldata(
    steamdata: &SteamData, 
    col_quantity: &Option<String>,
    data: &mut ExcelData, 
    row_in_excel: usize, 
    sheet: &mut Worksheet, 
) {
    if let Some(col_quantity) = col_quantity {
        if let Some(steam_quantity) = steamdata.quantity {
            if let Some(data_quantity) = data.quantity { 
                if data_quantity < steam_quantity {
                    let cell_quantity = format!("{}{}", col_quantity, row_in_excel);

                    data.quantity = Some(steam_quantity);
                    sheet.get_cell_value_mut( cell_quantity.as_ref() ).set_value_number(steam_quantity);
                    println!("UPDATED {} QUANTITY TO {:?} | ROW {}\n", &steamdata.name, &data.quantity, &row_in_excel);
                }
            }
        }
    }
}