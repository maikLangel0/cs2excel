use std::collections::HashMap;
use reqwest::header::COOKIE;
use serde_json::{from_value, Value};

use crate::models::web::{SteamJson, SteamData};

pub struct SteamInventory { 
    data: SteamJson,
    steamid: u64,
}
impl SteamInventory {
    ///Initializes the connection to the steam inventory and stores the inventory JSON in self
    pub async fn init(steamid: u64, gameid: u32, cookie: Option<String>) -> Result<Self, String> {
        let client = reqwest::Client::new();
        //                                              https://steamcommunity.com/inventory/76561198389123475/730/2?l=english&count=2000
        let steam_response: Value = client.get(format!("https://steamcommunity.com/inventory/{}/{}/2?l=english&count=2000", steamid, gameid))
            .header(COOKIE, if let Some(c) = cookie { c } else { String::new() } )
            .send()
            .await.map_err( |e| format!("Failed sending the HTTP request to steam! \n{}", e) )?
            .json()
            .await.map_err( |e| format!("Failed to parse steam inventory to a JSON. \n{}", e) )?;
        
        if steam_response.is_null() {
            return Err( "Oopsie JSON data is null! steamID and/or gameID might be wrong double check pls thank you!".into() );
        }

        let data = from_value::<SteamJson>(steam_response).map_err( |e| 
            format!("Parsing the json data from steam into the SteamJson struct did not work!\n{}", e) 
        )?;

        Ok( SteamInventory { data, steamid } )
    }

    ///Gets the names of the items in the inventory aswell as the quantity. 
    /// 
    ///`marketable` is true if you only want items from inventory that can be traded and/or listed to the community market.
    pub fn get_steam_items(self: &SteamInventory, group_simular_items: bool, marketable: bool) -> Result<Vec<SteamData>, String> { 
        let mut market_names: Vec<( &str, Option<String>, u64, u64 )> = Vec::new(); // name & inspect link & asset id
        
        for asset in &self.data.assets {
            for desc in &self.data.descriptions {
                
                //if "classid"s correlate, can fetch metadata for the skin/item from item_descriptions
                if asset.get("classid")
                    .ok_or_else(|| format!("'classid' not found in the asset: \n{:#?}", asset) )? 
                ==  desc.get("classid")
                    .ok_or_else(|| format!("'classid' not found in the description: \n{:#?}", desc) )? 
                { 
                    let empty: Vec<Value> = Vec::new();
                    let tradable: i64 = desc.get("tradable").and_then( |v| v.as_i64() ).unwrap_or( 0 ); 
                    let owner: &Vec<Value> = desc.get("owner_descriptions").and_then( |v| v.as_array() ).unwrap_or( &empty ); 
                    
                    // if only marketable allowed and (not tradable and no owner_descriptions), SKIP
                    if marketable && tradable == 0 && owner.is_empty() { continue }
                    
                    let market_name: &str = desc.get("market_name").and_then( |v| v.as_str() )
                        .ok_or_else(|| format!("'market_name' not found in the description! \nDescription: \n{:#?}", desc) )?; 
                    
                    let asset_id: u64 = asset.get("assetid")
                        .unwrap_or_else(|| &Value::Null )
                        .as_str()
                        .unwrap_or_else(|| &"0" )
                        .parse::<u64>()
                        .unwrap();

                        let instance_id: u64 = asset.get("instanceid")
                        .unwrap_or_else(|| &Value::Null )
                        .as_str()
                        .unwrap_or_else(|| &"0" )
                        .parse::<u64>()
                        .unwrap();

                    let inspect: Option<String> = desc.get("actions")
                        .and_then( |v| v.as_array() )
                        .and_then( |arr| arr.first() )
                        .and_then( |obj| obj.get("link") )
                        .and_then( |v| v.as_str() )
                        .map( |s| 
                            s.replace( "%owner_steamid%", &self.steamid.to_string() )
                            .replace( "%assetid%", &asset_id.to_string() )
                        );

                    market_names.push( (market_name, inspect, asset_id, instance_id) );
                    break;
                }
            }
        }

        let mut inventory: Vec<SteamData> = Vec::new();
        
        if group_simular_items {
            let mut name_quantity: HashMap<&str, (u16, Option<String>, u64, u64)> = HashMap::new();

            for (name, inspect_link, asset_id, instance_id) in market_names {
                let entry = name_quantity.entry(name).or_insert( (0, inspect_link, asset_id, instance_id) );
                entry.0 += 1;
            }

            for (name, (quantity, inspect_link, asset_id, instance_id)) in name_quantity {
                inventory.push( 
                    SteamData { 
                        name: name.to_string(), 
                        quantity: Some(quantity), 
                        inspect_link: { if quantity == 1 { inspect_link } else { None } }, 
                        asset_id,
                        instance_id
                    } 
                );
            }
        } 
        else {    
            for (name, inspect_link, asset_id, instance_id) in market_names {
                inventory.push(
                    SteamData { 
                        name: name.to_string(), 
                        quantity: None, 
                        inspect_link, 
                        asset_id,
                        instance_id
                    } 
                );
            }
        }
        //dprintln!("ALL STEAM ITEM DATA: {inventory:#?}");
        return Ok(inventory);
    }

    pub fn get_assets_length(self: &SteamInventory) -> usize {
        self.data.assets.len()
    }

    pub fn get_total_inventory_length(self: &SteamInventory) -> usize { 
        self.data.total_inventory_count as usize
    }
}

