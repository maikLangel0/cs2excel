use crate::models::item_metadata::{SPECIAL, WEARS};

pub fn create_csgoskins_urls(s_m_n: &str) -> String {
    let mut name = String::with_capacity(s_m_n.len());

    for c in s_m_n.chars() {
        match c {
            '\'' | '™' | '★' | '.' | '&' | '$' | ':' | '+' => continue,
            _ => name.push(c.to_ascii_lowercase())
        }
    }
    let mut url = String::with_capacity(name.len());

    let pre = ["charm", "patch", "sticker"];
    let suff = ["capsule", "case", "package", "pin"];

    // Checks if the prefixes "pre" are in the name OR suffixes of "suff"
    // Patch, Charm, Sticker, Capsule, Case, Package and Pin implementation
    if pre.iter().any(|&prefix| name.starts_with(prefix)) || suff.iter().any(|&suffix| name.ends_with(suffix)) {
        
        for sub in name.split("|") {
            url.push_str( &sub.replace("(", "").replace(")", "").replace(" ", "-") );
        }
        url = url.replace("--", "-");
    }
    // Has to be a wear value | gun and knife/gloves implementation
    else if WEARS.iter().any( |&w| name.contains(w) ) {
    
        let parts = name.split("(")
            .map(|n| n.to_string())
            .collect::<Vec<_>>();

        let wear = parts[1][0..parts[1].len() - 1]
            .to_string();

        let mut gun = parts[0].split("|")
            .collect::<Vec<&str>>()[0]
            .replace("★ ", "")
            .trim_start()
            .to_string();

        let name = parts[0].split("|")
            .collect::<Vec<&str>>()[1]
            .to_string();

        let mut tag = String::new();

        for spec in SPECIAL.iter() {
            if gun.contains(spec) {
                gun = gun.replace(&format!("{spec}-"), "")
                    .trim_start()
                    .to_string();
                
                tag = format!("{spec}-");
                break;
            }
        }

        url.push_str(&format!("{}-{}/{}{}",
            &gun[0..gun.len()]
                .to_string()
                .replace(" ", "-")
                .replace(&tag, ""),
            &name[1..name.len()]
                .trim()
                .to_string()
                .replace(" ", "-"),
            &tag,
            &wear.replace(" ", "-")
        )
        .replace("--", "-"));
    }
    // annoying edgecase where swap tool and statrak music boxes do not format the same as other stattrak items
    else if name.ends_with("swap tool") || name.ends_with("box") {
        url = name.replace(" ", "-");
    }

    else {
        let mut name_clean = String::with_capacity(name.len());

        for c in name.chars() {
            match c {
                '(' | ')' | ',' => continue,
                _ => name_clean.push(c),
            }
        }

        let mut tag: String = String::new();

        for sub in name_clean.split("|") {
            let mut part = sub.to_string();

            for spec in SPECIAL.iter() {
                
                if sub.contains(spec) {
                    part = sub.replace(spec, "")
                        .trim_start()
                        .to_string();

                    tag = format!("/{spec}");
                }
            }

            url.push_str(&part);
        }

        url = format!( "{}{}", url.replace("  ", " ").replace(" ", "-"), tag );
    }
    url
}