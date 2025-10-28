use std::{env, path::{Path, PathBuf}};
use ahash::{HashMap, HashMapExt};
use chrono::Utc;
use rand::{rng, seq::IndexedRandom};
use serde_json::Value;
use tokio::{fs, io::AsyncWriteExt};

use reqwest::Client;
use umya_spreadsheet::Worksheet;

use crate::{
    browser::{cookies::FirefoxDb, csfloat, csgotrader}, dprintln, gui::ice::Progress, models::{excel::ExcelData, price::{Currencies, Doppler, PriceType, PricingMode, PricingProvider}, user_sheet::{SheetInfo, UserInfo}, web::{CachedMarket, ExtraItemData, ItemInfoProvider, Sites, SteamData}}, parsing::{self, csgoskins_url, item_csgotrader, market_name_parse}, CACHE_TIME
};

pub fn get_steamloginsecure(sls: &Option<String>) -> Option<Vec<String>> {
    if let Some(sls) = sls { Some( Vec::from([sls.to_string()]) ) }
    else if let Ok(db) = FirefoxDb::init() {
        match db.get_cookies(vec!["name", "value"], "steamcommunity.com", vec!["steamLoginSecure"]) {
            Ok(cookie) => Some(cookie),
            Err(_e) => { dprintln!("FRICK.\n{}", _e); None }
        }
    } else { dprintln!("WARNING: Failed to connect to firefox."); None }
}

pub async fn get_exchange_rate(
    usd_to_x: &Currencies,
    rowcol_usd_to_x: &Option<String>,
    sheet: &mut Worksheet
) -> Result<f64, String> {

    if usd_to_x != &Currencies::None {
        if usd_to_x == &Currencies::USD { return Ok(1.0); }

        let rates: HashMap<String, f64> = csgotrader::get_exchange_rates().await?;
        Ok( *rates.get( usd_to_x.as_str() ).unwrap_or( &1.0 ) )

    } else if let Some(cell) = rowcol_usd_to_x {

        let res = sheet.get_cell_value( cell.as_ref() )
            .get_raw_value()
            .to_string()
            .trim()
            .to_string();

        if res.is_empty() { Err( String::from("usd_to_x cell is empty!") ) }
        else {
            Ok(
                res.parse::<f64>()
                    .map_err(|_| String::from("usd_to_x cell was not able to be converted to a number!")
                )?
            )
        }
    } else { Ok(1.0) }
}

pub async fn get_market_price(
    user: &UserInfo,
    markets_to_check: &Vec<Sites>,
    all_market_prices: &HashMap<Sites, Value>,
    rate: f64,
    item_name: &str,
    doppler: &Option<Doppler>,
    progress: &mut sipper::Sender<Progress>
) -> Result<(Option<String>, Option<f64>), String> {
    if !user.fetch_prices { Ok((None, None)) }
    else {
        #[derive(Clone, Copy)]
        struct MarketPrice { market: &'static str, price: f64 }

        let mut prices: Vec<MarketPrice> = Vec::new();

        // Finds the prices for each market
        for market in markets_to_check {
            // If site does not have doppler pricings AND doppler is something, SKIP
            if doppler.is_some() && !market.has_doppler() { continue; }

            if let Some(market_prices) = all_market_prices.get(market)
            && let Some(price) = item_csgotrader::get_price(
                item_name,
                market_prices,
                market,
                &PriceType::StartingAt,
                doppler,
                progress
            ).await? { prices.push( MarketPrice { market: market.as_str(), price: price * rate } ) }

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
                        .ok_or("PricingMode::Random failed what.")
                        .copied()?;
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
    pause_time_ms: u16,
    progress: &mut sipper::Sender<Progress>
) -> Result<Option<Value>, String> {

    if col_inspect_link.is_some() {
        if let Some(inspect) = inspect_link {
            match iteminfo_provider {
                ItemInfoProvider::Csfloat => {
                    let tmp = csfloat::fetch_iteminfo_persistent(client, progress, inspect, 10, pause_time_ms as u64).await?;
                    Ok(tmp)
                }
                ItemInfoProvider::Csgotrader => {
                    // TODO: MB IMPLEMENT FETCH_ITEMINFO_PERSISTENT for CSGOTRADER
                    let tmp = csfloat::fetch_iteminfo_persistent(client, progress, inspect, 10, pause_time_ms as u64).await?;
                    Ok(tmp)
                }
                ItemInfoProvider::Steam => { Ok(None) }
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
    progress: &mut sipper::Sender<Progress>
) -> Result<Option<ExtraItemData>, String> {

    let json_response = fetch_iteminfo_via_itemprovider_persistent(
        client,
        col_inspect_link,
        iteminfo_provider,
        &steamdata.inspect_link,
        pause_time_ms,
        progress
    ).await?;

    if let Some(json_body) = json_response {
        match iteminfo_provider {
            ItemInfoProvider::Csfloat => {
                let res = parsing::item_csfloat::parse_iteminfo_min(&json_body, Some(&steamdata.name) )?;
                Ok(Some(res))
            },
            ItemInfoProvider::Csgotrader => {
                let res = parsing::item_csfloat::parse_iteminfo_min(&json_body, Some(&steamdata.name) )?;
                Ok(Some(res))
            }
            ItemInfoProvider::Steam => Ok(None)
        }
    } else { Ok(None) }
}

pub async fn insert_new_exceldata(
    user: &UserInfo,
    excel: &SheetInfo,
    steamdata: &SteamData,
    extra_itemdata: &Option<ExtraItemData>,
    markets_to_check: &Option<Vec<Sites>>,
    all_market_prices: &Option<HashMap<Sites, Value>>,
    rate: f64,
    row_in_excel: usize,
    sheet: &mut Worksheet,
    progress: &mut sipper::Sender<Progress>
) -> Result<ExcelData, String> {

    let doppler: Option<Doppler> = extra_itemdata.as_ref()
        .and_then(|ei| ei.phase.clone());

    let (market, price): (Option<String>, Option<f64>) = if let Some(m_t_c) = markets_to_check && let Some(a_m_p) = all_market_prices {
        get_market_price(
            user,
            m_t_c,
            a_m_p,
            rate,
            &steamdata.name,
            &doppler,
            progress
        ).await?
    } else { (None, None) };

    // Inserting into the spreadsheet
    insert_string_in_sheet(sheet, &excel.col_steam_name, row_in_excel, &steamdata.name);

    if excel.col_gun_sticker_case.is_some() || excel.col_skin_name.is_some() || excel.col_wear.is_some() {
        let [gun_sticker_case, skin_name, wear] = market_name_parse::metadata_from_market_name(&steamdata.name);

        if let Some(col_gun_sticker_case) = &excel.col_gun_sticker_case && !gun_sticker_case.is_empty() { insert_string_in_sheet(sheet, &col_gun_sticker_case, row_in_excel, &gun_sticker_case); }
        if let Some(col_skin_name)        = &excel.col_skin_name && !skin_name.is_empty()               { insert_string_in_sheet(sheet, &col_skin_name, row_in_excel, &skin_name); }
        if let Some(col_wear)             = &excel.col_wear && !wear.is_empty()                         { insert_string_in_sheet(sheet, &col_wear, row_in_excel, &wear); }
    }

    if let Some(col_quantity)     = &excel.col_quantity && let Some(quantity) = steamdata.quantity              { insert_number_in_sheet(sheet, &col_quantity, row_in_excel, quantity); }
    if let Some(monetary)         = price                                                                       { insert_number_in_sheet(sheet, &excel.col_price, row_in_excel, monetary); }
    if let Some(col_market)       = &excel.col_market && let Some(marquet) = market                             { insert_string_in_sheet(sheet, col_market, row_in_excel, &marquet); }
    if let Some(col_inspect_link) = &excel.col_inspect_link && let Some(inspect_link) = &steamdata.inspect_link { insert_string_in_sheet(sheet, &col_inspect_link, row_in_excel, inspect_link); }
    if let Some(col_asset_id)     = &excel.col_asset_id && !user.group_simular_items                            { insert_number_in_sheet(sheet, &col_asset_id, row_in_excel, steamdata.asset_id as f64); }

    if steamdata.quantity == Some(1) || steamdata.quantity == None {
        if let Some(itemdata) = extra_itemdata {
            if let Some(col_float)   = &excel.col_float && let Some(float) = itemdata.float         { insert_number_in_sheet(sheet, &col_float, row_in_excel, float); }
            if let Some(col_pattern) = &excel.col_pattern && let Some(pattern) = itemdata.paintseed { insert_number_in_sheet(sheet, &col_pattern, row_in_excel, pattern); }
            if let Some(col_phase)   = &excel.col_phase && let Some(faze) = &itemdata.phase         { insert_string_in_sheet(sheet, &col_phase, row_in_excel, faze.as_str()); }
        } // Use data from steam if extra_itemdata is None
        else {
            if let Some(col_float)   = &excel.col_float && let Some(float) = steamdata.float       { insert_number_in_sheet(sheet, &col_float, row_in_excel, float); }
            if let Some(col_pattern) = &excel.col_pattern && let Some(pattern) = steamdata.pattern { insert_number_in_sheet(sheet, &col_pattern, row_in_excel, pattern); }
        }
    }

    if let Some(col_csgoskins_link) = &excel.col_csgoskins_link {
        let csgoskins_url = csgoskins_url::create_csgoskins_urls(steamdata.name.as_str());
        let link = format!("https://csgoskins.gg/items/{}", csgoskins_url);
        insert_string_in_sheet(sheet, &col_csgoskins_link, row_in_excel, &link);
    }

    spot(progress, format!("\t* INSERTING: {:-<75} | ROW: {}\n", &steamdata.name, row_in_excel)).await;

    Ok(ExcelData {
        name: steamdata.name.clone(),
        quantity: steamdata.quantity,
        phase: doppler.as_ref().map(|d| d.as_str().to_string()),
        // price: price.map_or_else(|| 0.0, |p| p),
        // inspect_link: steamdata.inspect_link.clone(),
        asset_id: if !user.group_simular_items { Some(steamdata.asset_id) } else { None },
        sold: None
    })
}

pub async fn update_quantity_exceldata(
    steamdata: &SteamData,
    col_quantity: &Option<String>,
    data: &mut ExcelData,
    row_in_excel: usize,
    sheet: &mut Worksheet,
    progress: &mut sipper::Sender<Progress>
) {
    if data.sold.is_none()
    && let Some(col_quantity) = col_quantity
    && let Some(steam_quantity) = steamdata.quantity
    && let Some(data_quantity) = data.quantity
    && data_quantity < steam_quantity
    {
        spot(progress, format!("\t* UPDATING QUANTITY OF {:-<75} FROM => {} TO => {} ROW: {}\n", &steamdata.name, &data.quantity.unwrap_or(0), steam_quantity, &row_in_excel)).await;
        data.quantity = Some(steam_quantity);
        insert_number_in_sheet(sheet, &col_quantity, row_in_excel, steam_quantity);
    }
}

#[inline]
pub fn insert_number_in_sheet(sheet: &mut Worksheet, col: &str, row_in_excel: usize, value: impl Into<f64>) {
    let cell = (col.to_column().unwrap(), row_in_excel as u32);
    sheet.get_cell_value_mut(cell).set_value_number(value);
}
#[inline]
pub fn insert_string_in_sheet(sheet: &mut Worksheet, col: &str, row_in_excel: usize, value: impl Into<String>) {
    let cell = (col.to_column().unwrap(), row_in_excel as u32);
    sheet.get_cell_value_mut(cell).set_value_string(value);
}

pub async fn get_cached_markets_data(markets_to_check: &Vec<Sites>, pricing_provider: &PricingProvider) -> Result<HashMap<Sites, serde_json::Value>, String> {
    let mut amp: HashMap<Sites, Value> = HashMap::new();

    let cache_dir = dirs::cache_dir()
        .unwrap_or(std::env::temp_dir())
        .join("cs2excel\\cache");

    for market in markets_to_check {
        let market_prices = match pricing_provider {
            PricingProvider::Csgoskins => { // IF I IMPLEMENT CSGOSKINS IN THE FUTURE
                get_cached_market_data(cache_dir.as_path(), &PricingProvider::Csgotrader, market, csgotrader::get_market_data).await?
            },
            PricingProvider::Csgotrader => {
                get_cached_market_data(cache_dir.as_path(), pricing_provider, market, csgotrader::get_market_data).await?
            },
        };

        amp.insert(market.to_owned(), market_prices);
    }
    Ok(amp)
}

async fn load_cache(cache_path: &Path) -> Result<CachedMarket, String> {
    let file = fs::read(cache_path).await.map_err(|e| format!("Read sink failed! | {}", e))?;
    let read = serde_json::from_slice::<CachedMarket>(&file).map_err(|e| format!("Failed to deserialize! | {}", e))?;
    Ok(read)
}

async fn save_cache(cache_path: &Path, marketjson: &Value) -> Result<(), String> {
    let cached = CachedMarket {
        prices: marketjson.clone(),
        timestamp: Utc::now()
    };

    dprintln!("{}", cache_path.display());

    let bytes = match serde_json::to_vec(&cached) {
        Ok(b) => b,
        Err(e) => {
            dprintln!("Error serializing cache | {}", e);
            return Err( format!("Error serializing cache | {}", e) );
        }
    };

    if let Some(parent_dir) = cache_path.parent() && let Err(e) = fs::create_dir_all(parent_dir).await {
        dprintln!("Failed to create cache directories: {}", e);
        return Err( format!("Failed to create cache directories: {}", e) );
    }

    let mut file = match fs::OpenOptions::new()
        .write(true)
        .read(true)
        .truncate(true)
        .create(true)
        .open(cache_path).await {
            Ok(f) => {f},
            Err(e) =>  {
                dprintln!("Error building OpenOptions | {}", e);
                return Err( format!("Error building OpenOptions | {}", e) );
            },
        };

    match file.write_all(&bytes).await {
        Ok(_) => {dprintln!("Cache saved successfully!")},
        Err(e) => {
            dprintln!("Error saving cache | {}", e);
            return Err( format!("Error saving cache | {}", e))
        },
    }

    file.flush().await.map_err(|e| format!("Error flushing file | {}", e))?;
    Ok(())

}

async fn get_cached_market_data<'a, F, Fut>(cache_dir: &Path, iteminfo_provider: &PricingProvider, market: &'a Sites, fetch: F) -> Result<serde_json::Value, String>
where
    F: Fn(&'a Sites) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, String>>
{
    let cache_path = cache_dir.join( format!("{}_cache_{}.json", market.as_str(), iteminfo_provider.as_str().to_lowercase()) );

    if cache_path.exists() {
        match load_cache(&cache_path).await {
            Ok(cm) => {
                let elapsed = Utc::now().signed_duration_since(cm.timestamp);
                if elapsed.num_seconds() < CACHE_TIME.as_secs() as i64 {
                    Ok(cm.prices)
                } else {
                    let market_data = fetch(market).await?;
                    save_cache(&cache_path, &market_data).await?;
                    Ok(market_data)
                }
            },
            Err(e) => {
                return Err( format!("Couldn't load cached market from {} \n{}", cache_path.to_string_lossy(), e) )
            },
        }
    } else {
        let market_data = fetch(market).await?;
        save_cache(&cache_path, &market_data).await?;
        Ok(market_data)
    }
}

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

pub fn rand_ascii_string(len: usize) -> String {
    let mut rng = rng();
    let fallback: u8 = b'e';
    (0..len).map(|_| *ALPHABET.choose(&mut rng).unwrap_or(&fallback) as char).collect()
}

pub fn generate_fallback_path(path: &mut Option<PathBuf>) {
    let mut p = dirs::desktop_dir()
        .or(dirs::home_dir())
        .or(env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from( format!("C\\Users\\{}", whoami::username()) ));

    p.push(format!("cs2_invest_sheet_{}.xlsx", rand_ascii_string(16)));

    *path = Some(p);
}

#[inline]
pub fn clear_extra_iteminfo_given_quantity(sheet: &mut Worksheet, quantity: Option<u16>, row_in_excel: usize, cols: [Option<&str>; 3] ) {
    if quantity != Some(1) && quantity.is_some() {
        for col in cols.iter().flatten() {
            insert_string_in_sheet(sheet, *col, row_in_excel, "");
        }
    }
}

/// Send progress only text
pub async fn spot<T>(sender: &mut sipper::Sender<Progress>, msg: T)
where T: Into<String>
{
    sender.send( Progress { message: msg.into(), percent: 0.0 }).await;
}

pub trait ToColumn {
    fn to_column(self) -> Option<u32>;
}

impl ToColumn for &str {
    fn to_column(self) -> Option<u32> {
        if self.chars().any(|c| !c.is_ascii()) { return None };

        let mut res: u32 = 0;
        let mut base: u8;

        for letter in self.bytes() {
            if letter >= b'a' && letter <= b'z' { base = b'a' }
            else if letter >= b'A' && letter <= b'Z' { base = b'A' }
            else { return None }

            res = res * 26 + (letter - base + 1) as u32;
        }

        Some(res)
    }
}

impl ToColumn for String {
    fn to_column(self) -> Option<u32> {
        self.as_str().to_column()
    }
}

pub trait LastInX {
    fn take_last_x(self, x: usize) -> String;
}

impl LastInX for &str {
    fn take_last_x(self, x: usize) -> String {
        self.chars()
            .rev()
            .take(x)
            .collect::<String>()
            .chars()
            .rev()
            .collect()
    }
}

impl LastInX for String {
    fn take_last_x(self, x: usize) -> String {
        self.as_str().take_last_x(x)
    }
}
