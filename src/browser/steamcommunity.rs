
use ahash::{HashMap, HashMapExt};
use reqwest::header::COOKIE;
use serde_json::{from_value, Value};

use crate::models::web::{SteamData, SteamJson, GAMES_TRADE_PROTECTED};

struct Description<'a> {
    inspect: Option<&'a str>,
    name_on_market: &'a str,
    is_tradable: bool,
    has_owner_descriptions: bool,
}

struct Properties {
    float: Option<f64>,
    pattern: Option<u32>,
}

struct IntermediateSteamData<'a> {
    inspect_link: Option<String>,
    name_on_market: &'a str,
    asset_id: u64,
    float: Option<f64>,
    pattern: Option<u32>
}

#[derive(Debug)]
pub struct SteamInventory { 
    data: SteamJson,
    steamid: u64,
}
impl SteamInventory {
    ///Initializes the connection to the steam inventory and stores the inventory JSON in self
    pub async fn init(steamid: u64, gameid: u32, cookie: Option<String>) -> Result<Self, String> {
        let client = reqwest::Client::new();
        let cockie = cookie.unwrap_or("".to_string());
        
        //                                              https://steamcommunity.com/inventory/76561198389123475/730/2?l=english&count=2000
        let steam_response: Value = client.get(format!("https://steamcommunity.com/inventory/{}/{}/2?l=english&count=2000", steamid, gameid))
            .header(COOKIE, &cockie )
            .send()
            .await.map_err( |e| format!("Failed sending the HTTP request to steam! \n{}", e) )?
            .json()
            .await.map_err( |e| format!("Failed to parse steam inventory to a JSON. \n{}", e) )?;

        if steam_response.is_null() {
            return Err( "Oopsie JSON data is null! steamID and/or gameID might be wrong double check pls thank you!".into() );
        }
        
        let mut data = from_value::<SteamJson>(steam_response).map_err( |e| 
            format!("Parsing the json data from steam into the SteamJson struct did not work! Usual cause is failure to get proper inventory data.\n{}.", e) 
        )?;

        let trade_protected: Option<Value> = if !cockie.is_empty() && GAMES_TRADE_PROTECTED.contains(&gameid) {
            match client.get(format!("https://steamcommunity.com/inventory/{}/{}/16?l?=english&count=2000", steamid, gameid))
                .header(COOKIE, &cockie)
                .send()
                .await.map_err( |e| format!("Failed sending the trade protect check HTTP request to steam! \n{}", e) ) 
                {
                    Ok(res) => {
                        // Fails silently and just returns None since user might not have any trade protected items in inv OR its not their inv
                        res.json::<Value>().await.ok()
                    },
                    Err(e) => { return Err(e) }
                }
        } else { None };

        if let Some(tp) = trade_protected  
        && let Some(assets) = tp.get("assets").and_then(|v| v.as_array())
        && let Some(descriptions) = tp.get("descriptions").and_then(|v| v.as_array() )
        && let Some(asset_properties) = tp.get("asset_properties").and_then(|v| v.as_array() )
        && let Some(tic) = tp.get("total_inventory_count").and_then(|v| v.as_u64()) 
        {
            if let Some(ref mut ass) = data.asset_properties {
                ass.append(&mut asset_properties.clone());
            } else {
                data.asset_properties = Some( asset_properties.clone() );
            }

            data.assets.append(&mut assets.clone());
            data.descriptions.append(&mut descriptions.clone());
            data.total_inventory_count += tic as u16;
        }

        Ok( SteamInventory { data, steamid } )
    }

    ///Gets the names of the items in the inventory aswell as the quantity. 
    /// 
    ///`marketable` is true if you only want items from inventory that can be traded and/or listed to the community market.
    /// 
    /// The assets serde_json::Value the de-facto iterator, while descriptions and asset_properties are turned into hashmaps.
    pub fn get_steam_items(self: &SteamInventory, group_simular_items: bool, marketable: bool) -> Result<Vec<SteamData>, String> { 
        
        let mut desc_map: HashMap<u64, Description> = HashMap::new(); // classid key
        let mut asset_prop_map: HashMap<u64, Properties> = HashMap::new(); // assetid key
        
        // construct hashmap for Descriptions
        for desc in &self.data.descriptions {
            let classid = desc.get("classid")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .ok_or_else(|| String::from("Classid fetch failed desc wat."))?;

            let name_on_market: &str = desc.get("market_name").and_then( |v| v.as_str() )
                .ok_or_else(|| String::from("Market name from desc failed wat."))?; 

            let is_tradable = desc.get("tradable").and_then(|v| v.as_i64()).unwrap_or(0) != 0;

            let has_owner_descriptions = desc.get("owner_descriptions").is_some();

            let inspect: Option<&str> = desc.get("actions")
                .and_then( |v| v.as_array() )
                .and_then( |arr| arr.first() )
                .and_then( |obj| obj.get("link") )
                .and_then( |v| v.as_str() );

            desc_map.insert(classid, Description { inspect, name_on_market, is_tradable, has_owner_descriptions });
        }
        
        // Construct hashmap for Properties
        if let Some(ass_prop) = &self.data.asset_properties {    
            for prop in ass_prop {
               let asset_id = prop.get("assetid")
                   .and_then(|v| v.as_str())
                   .and_then(|s| s.parse::<u64>().ok())
                   .ok_or_else(|| String::from("Assetid fetch failed asset_properties wat."))?;

               let asset_properties_iter = prop.get("asset_properties")
                   .ok_or_else(|| String::from("Failed to fetch asset_properties from asset_properties wat."))?
                   .as_array()
                   .ok_or_else(|| String::from("Failed to turn into array wat."))?
                   .iter();

               let mut float: Option<f64> = None;
               let mut pattern: Option<u32> = None;

               for property in asset_properties_iter {
                   if let Some(flt) = property.get("float_value")
                       .and_then(|v| v.as_str())
                       .and_then(|s| s.parse::<f64>().ok()) 
                   {
                           float = Some(flt);
                   }

                   if let Some(ptrn) = property.get("int_value")
                       .and_then(|v| v.as_str())
                       .and_then(|s| s.parse::<u32>().ok()) 
                   {
                           pattern = Some(ptrn);
                   }
               }

               asset_prop_map.insert(asset_id, Properties { float, pattern });
            }
        }

        let mut intermediate: Vec<IntermediateSteamData> = Vec::with_capacity(self.data.assets.len());

        for asset in &self.data.assets {
            let class_id = asset.get("classid")
                .and_then(|v| v.as_str())
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or_else(|| String::from("No classid in assets WHAT."))?;

            let description = desc_map.get(&class_id).ok_or_else(|| String::from("Description not found from hashmap WHAT."))?;
            
            if marketable && !description.is_tradable && !description.has_owner_descriptions { continue }

            let asset_id: u64 = asset.get("assetid")
                .and_then(|v| v.as_str())
                .and_then(|v| v.parse::<u64>().ok())
                .ok_or_else(|| String::from("No assetid in assets WHAT."))?;
            
            let (float, pattern): (Option<f64>, Option<u32>) = 
                if asset_prop_map.is_empty() { (None, None) } else {
                    if let Some(property) = asset_prop_map.get(&asset_id) { 
                        (property.float, property.pattern)
                    } else { (None, None) }
                };
            
            let inspect_link: Option<String> = description.inspect.map(|s| s
                .replace( "%owner_steamid%", &self.steamid.to_string() )
                .replace( "%assetid%", &asset_id.to_string() ) 
            );
                
            intermediate.push( 
                IntermediateSteamData { 
                    inspect_link, 
                    name_on_market: description.name_on_market, 
                    asset_id, 
                    float, 
                    pattern
                }
            );
        }

        let mut inventory: Vec<SteamData> = Vec::new();

        if group_simular_items {
            struct NamedValues { 
                inspect_link: Option<String>,
                float: Option<f64>,
                asset_id: u64,
                pattern: Option<u32>,
                quantity: u16, 
            }

            let mut data_mapped_with_quantity: HashMap<&str, NamedValues> = HashMap::new();

            for data in intermediate {
                let entry = data_mapped_with_quantity.entry(data.name_on_market)
                    .or_insert( 
                        NamedValues { 
                            inspect_link: data.inspect_link, 
                            float: data.float, 
                            asset_id: data.asset_id, 
                            pattern: data.pattern, 
                            quantity: 0
                        }
                );
                entry.quantity += 1;
            }

            for (name, data) in data_mapped_with_quantity {
                inventory.push(
                    SteamData { 
                        name: name.to_string(), 
                        quantity: Some(data.quantity), 
                        inspect_link: data.inspect_link, 
                        float: data.float, 
                        pattern: data.pattern, 
                        asset_id: data.asset_id 
                    }
                );
            }
        }
        else {
            for data in intermediate {
                inventory.push(
                    SteamData { 
                        name: data.name_on_market.to_string(), 
                        quantity: None, 
                        inspect_link: data.inspect_link, 
                        float: data.float, 
                        pattern: data.pattern, 
                        asset_id: data.asset_id 
                    }
                );
            }
        }
        
        Ok(inventory)
    }

    pub fn get_assets_length(self: &SteamInventory) -> usize {
        self.data.assets.len()
    }

    pub fn get_total_inventory_length(self: &SteamInventory) -> usize { 
        self.data.total_inventory_count as usize
    }
}

