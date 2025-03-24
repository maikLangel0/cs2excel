// used for constants that multiple helper files access
use crate::{ LazyLock, HashSet, HashMap };

pub static SPECIAL: LazyLock<HashSet<&'static str>> = LazyLock::new(|| HashSet::from([
    "souvenir", "stattrak"
]));

pub static WEARS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| HashSet::from([
    "factory new", "minimal wear", "field-tested", "well-worn", "battle-scarred"
]));

pub static FINISHES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| HashSet::from([
    "glitter", "holo-foil", "foil", "holo", "gold", "lenticular"
]));

pub static WEAR_ABBRIV: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| HashMap::from([
    ("stattrak", "st"), ("souvenir", "sv"), ("minimal wear", "mw"), ("factory new", "fn"),
    ("field-tested", "ft"), ("well-worn", "ww"), ("battle-scarred", "bs")
]));

pub static WEAPON_ABBRIV: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| HashMap::from([
    ("desert eagle", "deagle"), ("dual berettas", "dualies"), ("galil ar", "galil"),
    ("mp5-sd", "mp5"), ("r8 revolver", "r8"), ("pp-bizon", "bizon"), ("scar-20", "scar"),
    ("sg 553", "sg"), ("ssg 08", "ssg"), ("usp-s", "usp"), ("m4a1-s", "m4a1"),
    ("glock-18", "glock"), ("xm1014", "xm"), ("ump-45", "ump"), ("zeus x27", "zeus"),
    ("ak-47", "ak"), ("tec-9", "tec9"), ("m9 bayonet", "m9"), ("cz-75 auto", "cz"),
    ("g3sg1", "g3")
]));