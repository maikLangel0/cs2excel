use crate::{
    Error, HashMap, COOKIE, Deserialize, 
    Serialize, from_value, Value, dprintln
};

#[derive(Deserialize, Serialize, Debug)]
struct SteamJson {
    assets: Vec<Value>,
    descriptions: Vec<Value>,
    total_inventory_count: u16,
    success: i8,
    rwgrsn: i8,
}

pub struct SteamInventory { data: SteamJson }
impl SteamInventory {
    ///Initializes the connection to the steam inventory and stores the inventory JSON in self
    pub async fn init(steamid: u64, gameid: u32, cookie: &str) -> Result<Self, Box<dyn Error>> {
        let client = reqwest::Client::new();
        //                                              https://steamcommunity.com/inventory/76561198389123475/730/2?l=english&count=2000
        let steam_response: Value = client.get(format!("https://steamcommunity.com/inventory/{}/{}/2?l=english&count=2000", steamid, gameid))
            .header(COOKIE, cookie)
            .send()
            .await?
            .json()
            .await?;
        
        if steam_response.is_null() {
            return Err( "Oopsie JSON data is null! steamID and/or gameID might be wrong double check pls thank you!".into() );
        }

        let data = from_value::<SteamJson>(steam_response)?;
        Ok(SteamInventory { data })
    }

    ///Gets the names of the items in the inventory aswell as the quantity. 
    /// 
    ///`marketable` is true if you only want items from inventory that can be traded and/or listed to the community market.
    pub fn get_item_names(self: &SteamInventory, marketable: bool) -> Result<HashMap<String, u16>, Box<dyn Error>> { 
        dprintln!("Total assets: {}", self.data.assets.len());
        dprintln!("Inventory count: {}", self.data.total_inventory_count);

        let mut market_names: Vec<&str> = Vec::new();
        // Should be safe to unwrap here as steam would need to change the JSON for this to panic
        for asset in &self.data.assets {
            for desc in &self.data.descriptions {
                if asset.get("classid").unwrap() == desc.get("classid").unwrap() { //if "classid"s correlate, can fetch metadata for the skin/item from item_descriptions
                    let empty : Vec<Value> = Vec::new();
                    let tradable: i64 = desc.get("tradable").and_then( |v| v.as_i64() ).unwrap_or( 0 ); 
                    let owner: &Vec<Value> = desc.get("owner_descriptions").and_then( |v| v.as_array() ).unwrap_or( &empty ); 

                    if marketable && ( tradable == 0 && owner.is_empty() ) { continue } // if only marketable allowed and (not tradable and no owner_descriptions), SKIP
                    
                    let market_name: &str = desc.get("market_name").and_then( |v| v.as_str() ).unwrap(); 
                    market_names.push(market_name);
                    break;
                }
            }
        }

        let mut inventory: HashMap<String, u16> = HashMap::new();
        for name in market_names {
            *inventory.entry(name.to_string()).or_insert(0) += 1;
        }
        return Ok(inventory);
    }

    pub fn get_assets_length(self: &SteamInventory) -> usize {
        self.data.assets.len()
    }

    pub fn get_total_inventory_length(self: &SteamInventory) -> usize { 
        self.data.total_inventory_count as usize
    }
}

