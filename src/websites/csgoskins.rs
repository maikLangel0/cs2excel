use crate::{
    Error, Arc, COOKIE, Html, Selector,
    IndexMap, LazyLock, CookieStoreMutex,
    dprintln, USER_AGENT
};

// This is so that I dont have to make a new client each time a new request is made
// AND it also stores the cookies so they dont have to be instanciated again and again
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| { 
    let store_method = Arc::new( CookieStoreMutex::default() );
    
    reqwest::Client::builder()
        .cookie_provider(store_method)
        .build()
        .unwrap()
});

struct MarketData {
    price: f64,
    _stars: f32,
    _reviews: u32,
    _active_offers: u32,
    name: String,
}
pub struct Csgoskins { markets_data: Vec<MarketData> }
impl Csgoskins {
    pub async fn init(url: &str, cookie: &str, user_agent: &str) -> Result<Self, Box<dyn Error>> {

        let response = CLIENT.get(url)
            .header(COOKIE, cookie)
            .header(USER_AGENT, user_agent)
            .send()
            .await?;

        // Response needs to be gucci
        if !response.status().is_success() {
            return Err( 
                format!("Response was not successfull!\nURL: {}\nStatus code: {}", url, response.status()).into() 
            )
        }
        
        let body = response.text().await?;
        let document = Html::parse_document(&body);

        let market_selector = Selector::parse(
            "div.active-offer.bg-gray-800.rounded-sm.shadow-md.relative.flex.items-center.flex-wrap.my-4")
            .unwrap();

        let market_div = document.select(&market_selector);
        let mut all_markets: Vec<MarketData> = Vec::new();

        for info in market_div {
            let market_info_raw = info.text().collect::<Vec<&str>>();
            
            //Removes all whitespace, lowercases, and only uses the last 8 elements of the Vec because 
            //the Vec<String> is frontloaded with junk if the market_info_raw[iter] is a sponsored site
            let market_info: Vec<String> = market_info_raw
                .iter()
                .filter(|s| !s.trim().is_empty() )
                .map(|s| s.trim().to_lowercase() )
                .collect::<Vec<String>>()
                .into_iter().rev().take(8)
                .rev().collect();

            // Checking that market_info is of expected length
            if market_info.len() != 8 {
                return Err( 
                    format!(
                        "Market_info is not the expected size!\nLength: {}\nmarket_info: {:?}", 
                        market_info.len(), 
                        market_info
                    ).into() 
                )
            }

            let star_and_review: Vec<&str> = market_info[1].split("•").collect();

            dprintln!("{:?}", market_info);

            let review_str = star_and_review[1]
                .replace("reviews", "")
                .replace("k", if star_and_review[1].contains(".") {"00"} else {"000"})
                .replace(".", "")
                .trim()
                .to_string();
            
            let reviews: u32 = review_str
                .parse::<u32>()
                .map_err(|_| format!("Failed to parse review from csgoskins given the value {}\nURL: {}", &review_str, url) )?;

            let stars: f32 = star_and_review[0]
                .trim_end()
                .parse::<f32>()
                .map_err(|_| format!("Failed to parse stars from csgoskins given the value {}\nURL: {}", &star_and_review[0], url) )?;

            let active_offers: u32 = market_info[3]
                .replace("k", if market_info[3].contains(".") {"00"} else {"000"})
                .replace(".", "")
                .parse::<u32>()
                .map_err(|_| format!("Failed to parse active offers from csgoskins offers given the value {}\nURL: {}", &market_info[3], url) )?;
            
            let price: f64 = market_info[5]
                .trim_start_matches("$")
                .replace(",", "")
                .parse::<f64>()
                .map_err(|_| format!("Failed to parse price from csgoskins given the value {}\nURL: {}", &market_info[5], url) )?;
                
            let name = market_info[0].to_owned();

            let market = MarketData {
                name,
                _stars: stars,
                _active_offers: active_offers,
                _reviews: reviews,
                price,
            };
            
            all_markets.push(market);
        }

        Ok( Csgoskins { markets_data: all_markets } )
    }

    pub async fn get_name_price(self: &Csgoskins) -> Result<IndexMap<String, f64>, Box<dyn Error>> {
        let mut name_price: IndexMap<String, f64> = IndexMap::new();

        for market in self.markets_data.iter() {
            name_price.insert(market.name.to_string(), market.price);
        }
        name_price.sort_by( |_, v1, _, v2| v1.partial_cmp(v2).unwrap() );
        Ok(name_price)
    }
}