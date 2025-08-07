use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use strum::EnumIter;


#[derive(Debug, Deserialize, Serialize, Clone, EnumIter, PartialEq, Copy)]
pub enum PricingMode {
    Cheapest,
    MostExpensive,
    Hierarchical,
    Random
}
impl PricingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PricingMode::Cheapest => "Cheapest",
            PricingMode::MostExpensive => "Most Expensive",
            PricingMode::Hierarchical => "Hierarchical",
            PricingMode::Random => "Random",
        }
    }
}

impl FromStr for PricingMode {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "cheapest" => Ok(Self::Cheapest),
            "mostexpensive" => Ok(Self::MostExpensive),
            "most expensive" => Ok(Self::MostExpensive),
            "hierarchical" => Ok(Self::Hierarchical),
            "random" => Ok(Self::Random),
            "most" => Ok(Self::MostExpensive),
            "hier" => Ok(Self::Hierarchical),
            "r" => Ok(Self::Random),
            _ => Err( format!("Pricingmode of {} not allowed.", s))
        }
    }
}

impl fmt::Display for PricingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

//--------------------

#[derive(Debug, Deserialize, Serialize, Clone, Copy, EnumIter, PartialEq)]
pub enum PricingProvider {
    Csgotrader,
    Csgoskins,
}
impl PricingProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            PricingProvider::Csgoskins => "Csgoskins",
            PricingProvider::Csgotrader => "CsgoTrader"
        }
    } 
}
impl FromStr for PricingProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "CSGOSKINS" => Ok(PricingProvider::Csgoskins),
            "CSGOTRADER" => Ok(PricingProvider::Csgotrader),
            "" => Ok(PricingProvider::Csgotrader),
            e => Err(format!("Invalid PricingProvider: {}", e))
        }
    }
}

impl fmt::Display for PricingProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

//--------------------

#[derive(PartialEq, EnumIter)]
pub enum PriceType {
    StartingAt,
    HightestOrder
}
impl PriceType {
    pub fn as_str(&self) -> &'static str {
        let s = match self {
            PriceType::StartingAt => "starting_at",
            PriceType::HightestOrder => "highest_order"
        };
        s
    }
}

//--------------------

#[derive(PartialEq, EnumIter, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Currencies {
    COP, SAR, PLN, ARS, SGD, GBP, USD, 
    PHP, DKK, KRW, INR, ZAR, BRL, BGN, 
    CLP, JPY, PEN, ETH, TRY, RON, NOK, 
    TWD, HUF, MXN, UYU, QAR, AUD, CRC, 
    KZT, RUB, BTC, EUR, AED, CZK, HRK, 
    MYR, CNY, ILS, UAH, HKD, THB, NZD, 
    VND, GEL, SEK, CAD, CHF, ISK, IDR, 
    KWD, FET, None
}
impl fmt::Display for Currencies {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str() )
    }
}

impl FromStr for Currencies {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "COP" => Ok(Currencies::COP), "SAR" => Ok(Currencies::SAR), "PLN" => Ok(Currencies::PLN),
            "ARS" => Ok(Currencies::ARS), "SGD" => Ok(Currencies::SGD), "GBP" => Ok(Currencies::GBP),
            "USD" => Ok(Currencies::USD), "PHP" => Ok(Currencies::PHP), "DKK" => Ok(Currencies::DKK),
            "KRW" => Ok(Currencies::KRW), "INR" => Ok(Currencies::INR), "ZAR" => Ok(Currencies::ZAR),
            "BRL" => Ok(Currencies::BRL), "BGN" => Ok(Currencies::BGN), "CLP" => Ok(Currencies::CLP),
            "JPY" => Ok(Currencies::JPY), "PEN" => Ok(Currencies::PEN), "ETH" => Ok(Currencies::ETH),
            "TRY" => Ok(Currencies::TRY), "RON" => Ok(Currencies::RON), "NOK" => Ok(Currencies::NOK),
            "TWD" => Ok(Currencies::TWD), "HUF" => Ok(Currencies::HUF), "MXN" => Ok(Currencies::MXN),
            "UYU" => Ok(Currencies::UYU), "QAR" => Ok(Currencies::QAR), "AUD" => Ok(Currencies::AUD),
            "CRC" => Ok(Currencies::CRC), "KZT" => Ok(Currencies::KZT), "RUB" => Ok(Currencies::RUB),
            "BTC" => Ok(Currencies::BTC), "EUR" => Ok(Currencies::EUR), "AED" => Ok(Currencies::AED),
            "CZK" => Ok(Currencies::CZK), "HRK" => Ok(Currencies::HRK), "MYR" => Ok(Currencies::MYR),
            "CNY" => Ok(Currencies::CNY), "ILS" => Ok(Currencies::ILS), "UAH" => Ok(Currencies::UAH),
            "HKD" => Ok(Currencies::HKD), "THB" => Ok(Currencies::THB), "NZD" => Ok(Currencies::NZD),
            "VND" => Ok(Currencies::VND), "GEL" => Ok(Currencies::GEL), "SEK" => Ok(Currencies::SEK),
            "CAD" => Ok(Currencies::CAD), "CHF" => Ok(Currencies::CHF), "ISK" => Ok(Currencies::ISK),
            "IDR" => Ok(Currencies::IDR), "KWD" => Ok(Currencies::KWD), "FET" => Ok(Currencies::FET),
            "NULL" => Ok(Currencies::None), "NONE" => Ok(Currencies::None), "" => Ok(Currencies::None),
            other => Err(format!("Invalid currency code: {}", other)),
        }
    }
}

impl Currencies {
    pub fn as_str(&self) -> &'static str {
        match self {
            Currencies::COP => "COP", Currencies::SAR => "SAR", Currencies::PLN => "PLN",
            Currencies::ARS => "ARS", Currencies::SGD => "SGD", Currencies::GBP => "GBP", 
            Currencies::USD => "USD", Currencies::PHP => "PHP", Currencies::DKK => "DKK", 
            Currencies::KRW => "KRW", Currencies::INR => "INR", Currencies::ZAR => "ZAR", 
            Currencies::BRL => "BRL", Currencies::BGN => "BGN", Currencies::CLP => "CLP", 
            Currencies::JPY => "JPY", Currencies::PEN => "PEN", Currencies::ETH => "ETH", 
            Currencies::TRY => "TRY", Currencies::RON => "RON", Currencies::NOK => "NOK", 
            Currencies::TWD => "TWD", Currencies::HUF => "HUF", Currencies::MXN => "MXN", 
            Currencies::UYU => "UYU", Currencies::QAR => "QAR", Currencies::AUD => "AUD", 
            Currencies::CRC => "CRC", Currencies::KZT => "KZT", Currencies::RUB => "RUB", 
            Currencies::BTC => "BTC", Currencies::EUR => "EUR", Currencies::AED => "AED", 
            Currencies::CZK => "CZK", Currencies::HRK => "HRK", Currencies::MYR => "MYR", 
            Currencies::CNY => "CNY", Currencies::ILS => "ILS", Currencies::UAH => "UAH", 
            Currencies::HKD => "HKD", Currencies::THB => "THB", Currencies::NZD => "NZD", 
            Currencies::VND => "VND", Currencies::GEL => "GEL", Currencies::SEK => "SEK",
            Currencies::CAD => "CAD", Currencies::CHF => "CHF", Currencies::ISK => "ISK", 
            Currencies::IDR => "IDR", Currencies::KWD => "KWD", Currencies::FET => "FET",
            Currencies::None => "None",
        }
    }
}

//--------------------

#[derive(Debug, Serialize, Deserialize, EnumIter, PartialEq, Clone)]
pub enum Doppler {
    Phase1,
    Phase2,
    Phase3,
    Phase4,
    Ruby,
    Sapphire,
    BlackPearl,
    Emerald
}
impl FromStr for Doppler {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().split_whitespace().collect::<String>().as_str() {
            "phase1" => Ok(Self::Phase1),
            "phase2" => Ok(Self::Phase2),
            "phase3" => Ok(Self::Phase3),
            "phase4" => Ok(Self::Phase4),
            "ruby" => Ok(Self::Ruby),
            "sapphire" => Ok(Self::Sapphire),
            "blackpearl" => Ok(Self::BlackPearl),
            "emerald" => Ok(Self::Emerald),
            _ => Err( String::from(format!("{} is not a valid doppler finish.", s)) )
        }
    }
}
impl Doppler {
    pub fn as_str(&self) -> &'static str {
        match self {
            Doppler::Sapphire => "Sapphire",
            Doppler::Ruby => "Ruby",
            Doppler::BlackPearl => "Black Pearl",
            Doppler::Emerald => "Emerald",
            Doppler::Phase1 => "Phase 1",
            Doppler::Phase2 => "Phase 2",
            Doppler::Phase3 => "Phase 3",
            Doppler::Phase4 => "Phase 4"
        }
    }

    pub fn is_doppler(paintindex: u16) -> Option<Doppler> {
        match paintindex {
            415 => Some(Doppler::Ruby),
            416 => Some(Doppler::Sapphire),
            417 => Some(Doppler::BlackPearl),
            418 => Some(Doppler::Phase1),
            419 => Some(Doppler::Phase2),
            420 => Some(Doppler::Phase3),
            421 => Some(Doppler::Phase4),

            // GLOCK GAMMA DOPPLER
            1119 => Some(Doppler::Emerald),
            1120 => Some(Doppler::Phase1),
            1121 => Some(Doppler::Phase2),
            1122 => Some(Doppler::Phase3),
            1123 => Some(Doppler::Phase4),

            // KNIFE GAMMA DOPPLER
            568 => Some(Doppler::Emerald),
            569 => Some(Doppler::Phase1),
            570 => Some(Doppler::Phase2),
            571 => Some(Doppler::Phase3),
            572 => Some(Doppler::Phase4),
            _ => None
        }
    }
}