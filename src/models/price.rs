use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use strum::EnumIter;


#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PricingMode {
    Cheapest,
    MostExpensive,
    Hierarchical,
    Random
}
impl FromStr for PricingMode {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "cheapest" => Ok(Self::Cheapest),
            "mostexpensive" => Ok(Self::MostExpensive),
            "hierarchical" => Ok(Self::Hierarchical),
            "random" => Ok(Self::Random),
            "most" => Ok(Self::MostExpensive),
            "hier" => Ok(Self::Hierarchical),
            "r" => Ok(Self::Random),
            _ => Err( format!("Pricingmode of {} not allowed.", s))
        }
    }
}

//--------------------

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum PricingProvider {
    Csgotrader,
    Csgoskins,
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

#[derive(PartialEq, EnumIter, Serialize, Deserialize, Debug, Clone)]
pub enum Currencies {
    COP, SAR, PLN, ARS, SGD, GBP, USD, 
    PHP, DKK, KRW, INR, ZAR, BRL, BGN, 
    CLP, JPY, PEN, ETH, TRY, RON, NOK, 
    TWD, HUF, MXN, UYU, QAR, AUD, CRC, 
    KZT, RUB, BTC, EUR, AED, CZK, HRK, 
    MYR, CNY, ILS, UAH, HKD, THB, NZD, 
    VND, GEL, SEK, CAD, CHF, ISK, IDR, 
    KWD, FET
}
impl fmt::Display for Currencies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str() )
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
}