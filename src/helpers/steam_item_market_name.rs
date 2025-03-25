use crate::{
    helpers::helper_consts::{
        FINISHES, SPECIAL, WEARS, 
        WEAPON_ABBRIV, WEAR_ABBRIV
    },
    Error, Regex
};

//s_m_n = steam_market_name
pub fn cs_get_metadata_from_market_name(s_m_n: &str) -> Result<[String; 3], Box<dyn Error>> {
    // For dividing up info about the item so its optimal to use in URL and spreadsheet
    let mut gun_sticker_case;
    let mut skin_name;
    let mut wear: String = String::new();

    let mut name = String::with_capacity( s_m_n.len() );

    for c in s_m_n.chars() {
        match c {
            '\'' | '™' | '★' | '(' | ')' => continue,
            _ => name.push(c.to_ascii_lowercase())
        }
    }
    name = name.trim_start().to_string();

    // Regex to find if String has a year in it or not (4 decimals)
    let year = Regex::new(r"\b\d{4}\b").unwrap();

    let pre = ["charm", "patch", "sticker"];
    let suff = ["capsule", "case", "package", "pin"];

    // Checks if the prefixes "pre" are in the name OR suffixes of "suff"
    if pre.iter().any(|&prefix| name.starts_with(prefix)) || suff.iter().any(|&suffix| name.ends_with(suffix)) {
        
        // how did I make this work lol lamao rust silly moments (made Vec<String> so no worry bout borrow)
        let parts = name.split(" | ")
            .collect::<Vec<&str>>().join(" ").split(" ")
            .collect::<Vec<&str>>().iter().map(|s| s.to_string())
            .collect::<Vec<String>>();

        let p_len = parts.len();

        // Charms and patches
        if name.starts_with("charm") || name.starts_with("patch") {
            gun_sticker_case = parts[0].clone();
            skin_name = parts[1..].join(" ");

            for finish in FINISHES.iter() {
                if parts.contains(&finish.to_string()) {
                    let finito = format!("{} ", finish);
                    skin_name = skin_name.replace(&finito, "");
                    wear = finish.to_string();
                    break;
                }
            }
        }

        // Capsules
        else if name.contains("capsule") {

            // If capsule does not contain a year (enfu sticker capsule)
            if year.find(&name).is_none() {
                gun_sticker_case = "capsule".to_string(); //capsule 
                skin_name = name; //enfu sticker capsule
            }
            // Paris 2023 contenders autograph capsule
            else { 
                gun_sticker_case = format!("{} {}", parts[0], parts[1]); //Paris 2023
                skin_name = parts[2..p_len - 2].join(" "); //contenders
                if parts.contains( &"autograph".to_string() ) { 
                    skin_name.push_str(" auto") //auto (?)
                } 
            }
        }

        // Case and pin (Howl pin)
        else if name.ends_with("case") || name.ends_with("pin") {
            gun_sticker_case = parts[p_len - 1].clone(); //pin
            skin_name = parts[0..p_len - 1].join(" "); //Howl
        }

        // Sticker
        // sticker pain gaming gold paris 2023 !!OR!! sticker lefty ct
        else if name.starts_with("sticker") {
            
            if parts[p_len - 1].parse::<u16>().is_ok() {
                gun_sticker_case = parts[p_len - 2..p_len].join(" "); //paris 2023
            }
            else {
                gun_sticker_case = "sticker".to_string(); //sticker
            }

            wear = FINISHES.iter() 
                .find( |&&s| parts.contains( &s.to_string() ) )
                .unwrap_or(&"paper")
                .to_string(); // gold

            skin_name = name.replace(&wear, "")
                .replace(&gun_sticker_case, "")
                .replace(" | ", "")
                .replace("sticker", "")
                .trim()
                .to_string(); //Pain gaming
        }
        
        // Package (shanghai 2024 dust ii package)
        else {
            gun_sticker_case = parts[0..2]
                .join(" ")
                .to_string(); //shanghai 2024

            skin_name = parts[2..]
                .join(" ")
                .replace(" souvenir", "")
                .replace("ii", "2"); //dust 2 package
        }
    }
    
    // If name contains a wear, has to be a gun at this point
    else if WEARS.iter().any(|w| name.contains(w)) {
        let parts = name.split(" | ")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        let wear_temp = WEARS.iter()
            .find( |&&w| name.contains( w ) )
            .unwrap_or(&"n/a")
            .to_string();

        let tag_temp = SPECIAL.iter()
            .find( |&&t| name.contains( t ) )
            .unwrap_or(&"")
            .to_string();

        let weapon_name = parts[0].replace(&tag_temp, "")
            .trim()
            .to_string(); 

        wear = format!("{} {}", 
            WEAR_ABBRIV.get(&tag_temp.as_str())
                .unwrap_or(&""), 
            WEAR_ABBRIV.get(&wear_temp.as_str())
                .unwrap_or(&""))
            .trim()
            .to_string();

        skin_name = parts[1].replace(&wear_temp, "")
            .trim()
            .to_string();

        gun_sticker_case = WEAPON_ABBRIV.get( &weapon_name.as_str() )
            .unwrap_or( &weapon_name.as_str() )
            .to_string();

        let g_s_c_temp: Vec<&str> = gun_sticker_case.split_whitespace().collect();

        gun_sticker_case = match g_s_c_temp[g_s_c_temp.len() - 1] {
            "knife" => g_s_c_temp[0].to_string(),
            "gloves" => "gloves".to_string(),
            _ => gun_sticker_case,
        }        
    }

    // Extreme edgecase | Is a capsule but of the old style so doesnt get cought by above parameters
    // Examples: esl one cologne 2015 legends (foil) | esl one cologne 2014 challengers
    // Returns 4 digit number in key, None if number not found.
    // LOL THIS WORKS FOR PATCH PACKS
    else if year.find(&name).is_some() {
        let advanced = Regex::new(r"([a-z\-]+\s+\d{4})").unwrap();
        
        if let Some(found) = advanced.find(&name) {
            gun_sticker_case = found.as_str().to_string();
        }
        else {
            gun_sticker_case = String::from("This shouldn't happen lol lamo");
        }
        
        wear = FINISHES.iter()
            .filter(|&&f| name.contains(f))
            .max_by_key(|&&f| f.len() )
            .copied()
            .unwrap_or(&"")
            .to_string();
        
        let skin_name_temp = name.replace(&gun_sticker_case, "")
            .replace(&wear, "")
            .replace("()", "")
            .replace("|", "")
            .replace("capsule", "")
            .replace("sticker", "")
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");

        if skin_name_temp.contains("auto") {
            skin_name = format!("{} {}", skin_name_temp.replace("autograph", ""), "auto")
                .trim()
                .to_string();
        }
        else {
            skin_name = skin_name_temp;
        }
    }

    // Misc implementation (name tag, music kits, etc...)
    else {
        gun_sticker_case = if name.contains("music kit") {"music kit".to_string()} else {"misc".to_string()};
        
        let wear_temp = SPECIAL.iter()
            .find(|&&s| name.contains(s))
            .unwrap_or(&"")
            .to_string();

        skin_name = name.replace(&gun_sticker_case, "")
            .replace(&wear_temp, "")
            .replace("|", "")
            .trim()
            .to_string();

        wear = WEAR_ABBRIV.get(&wear_temp.as_str())
            .unwrap_or(&"")
            .to_string();
    }   
    Ok([gun_sticker_case, skin_name, wear])
}