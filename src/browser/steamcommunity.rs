
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
        let cookie = cookie.unwrap_or("".to_string());
        
        //                                              https://steamcommunity.com/inventory/76561198389123475/730/2?l=english&count=2000
        let steam_response: Value = client.get(format!("https://steamcommunity.com/inventory/{}/{}/2?l=english&count=2000", steamid, gameid))
            .header(COOKIE, &cookie )
            .send()
            .await.map_err( |e| format!("Failed sending main HTTPS request to steam. Check internet connection or steam availability. \n{}", e) )?
            .json()
            .await.map_err( |e| format!("Failed to parse steam inventory to a JSON. \n{}", e) )?;

        if steam_response.is_null() {
            return Err( "Oopsie JSON data is null! steamID and/or gameID might be wrong double check pls thank you!".into() );
        }
        
        let mut data = from_value::<SteamJson>(steam_response).map_err( |e| 
            format!("Parsing the json data from steam into the SteamJson struct did not work! Usual cause is failure to get proper inventory data.\n{}.", e) 
        )?;

        let trade_protected: Option<SteamJson> = if !cookie.is_empty() && GAMES_TRADE_PROTECTED.contains(&gameid) {
            match client.get(format!("https://steamcommunity.com/inventory/{}/{}/16?l=english&count=2000", steamid, gameid))
                .header(COOKIE, &cookie)
                .send()
                .await.map_err( |e| format!("Failed sending trade protect HTTPS request to steam. \n{}", e) ) 
                {
                    Ok(res) => {
                        // Fails silently and just returns None since user might not have any trade protected items in inv OR its not their inv
                        res.json::<SteamJson>().await.ok()
                    },
                    Err(e) => { return Err(e) }
                }
        } else { None };

        if let Some(mut tp) = trade_protected {
            if let Some(ref mut ass) = data.asset_properties && let Some(ref mut more_ass) = tp.asset_properties {
                ass.append(more_ass);
            } else if let Some(more_ass) = tp.asset_properties.take() {
                data.asset_properties = Some(more_ass);
            }

            data.assets.append(&mut tp.assets);
            data.descriptions.append(&mut tp.descriptions);
            data.total_inventory_count += tp.total_inventory_count;
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

               let asset_properties = prop.get("asset_properties")
                   .and_then(|a| a.as_array())
                   .ok_or_else(|| String::from("Failed to get/use inner asset_properties."))?;

               let mut float: Option<f64> = None;
               let mut pattern: Option<u32> = None;

               // Loop here to future-proof the implementation 
               for property in asset_properties {
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
                if !asset_prop_map.is_empty() && let Some(property) = asset_prop_map.get(&asset_id) {
                    (property.float, property.pattern)
                } else { (None, None) };
            
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

    pub fn assets_len(self: &SteamInventory) -> usize {
        self.data.assets.len()
    }

    pub fn inventory_len(self: &SteamInventory) -> usize { 
        self.data.total_inventory_count as usize
    }
}

