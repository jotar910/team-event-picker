use std::fmt::Display;

use chrono_tz::{Tz, Asia, Europe, Australia, Pacific, America, Africa};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Timezone {
    UTC,
    GMT,
    ECT,
    EET,
    ART,
    EAT,
    MET,
    NET,
    PLT,
    IST,
    BST,
    VST,
    CTT,
    JST,
    ACT,
    AET,
    SST,
    NST,
    MIT,
    HST,
    AST,
    PST,
    PNT,
    MST,
    CST,
    EST,
    IET,
    PRT,
    CNT,
    AGT,
    BET,
    CAT,
}

#[derive(Serialize)]
pub struct TimezoneOption {
    label: String,
    value: String,
}

impl Timezone {
    pub fn all() -> [Timezone; 32] {
        [
            Timezone::UTC,
            Timezone::GMT,
            Timezone::ECT,
            Timezone::EET,
            Timezone::ART,
            Timezone::EAT,
            Timezone::MET,
            Timezone::NET,
            Timezone::PLT,
            Timezone::IST,
            Timezone::BST,
            Timezone::VST,
            Timezone::CTT,
            Timezone::JST,
            Timezone::ACT,
            Timezone::AET,
            Timezone::SST,
            Timezone::NST,
            Timezone::MIT,
            Timezone::HST,
            Timezone::AST,
            Timezone::PST,
            Timezone::PNT,
            Timezone::MST,
            Timezone::CST,
            Timezone::EST,
            Timezone::IET,
            Timezone::PRT,
            Timezone::CNT,
            Timezone::AGT,
            Timezone::BET,
            Timezone::CAT,
        ]
    }

    pub fn option(self) -> TimezoneOption {
        TimezoneOption {
            label: self.to_string(),
            value: self.into(),
        }
    }

    pub fn tz(&self) -> Tz {
        match self {
            Timezone::GMT => Europe::Lisbon,
            Timezone::ECT => Europe::Paris,
            Timezone::EET => Europe::Athens,
            Timezone::ART => Africa::Cairo,
            Timezone::EAT => Africa::Nairobi,
            Timezone::MET => Europe::Madrid,
            Timezone::NET => Asia::Tashkent,
            Timezone::PLT => Asia::Karachi,
            Timezone::IST => Asia::Kolkata,
            Timezone::BST => Asia::Dhaka,
            Timezone::VST => Asia::Ho_Chi_Minh,
            Timezone::CTT => Asia::Shanghai,
            Timezone::JST => Asia::Tokyo,
            Timezone::ACT => Australia::Darwin,
            Timezone::AET => Australia::Sydney,
            Timezone::SST => Pacific::Guadalcanal,
            Timezone::NST => Pacific::Auckland,
            Timezone::MIT => Pacific::Midway,
            Timezone::HST => Pacific::Honolulu,
            Timezone::AST => America::Halifax,
            Timezone::PST => America::Los_Angeles,
            Timezone::PNT => America::Phoenix,
            Timezone::MST => America::Denver,
            Timezone::CST => America::Chicago,
            Timezone::EST => America::New_York,
            Timezone::IET => America::Indiana::Indianapolis,
            Timezone::PRT => America::Puerto_Rico,
            Timezone::CNT => America::St_Johns,
            Timezone::AGT => America::Argentina::Buenos_Aires,
            Timezone::BET => America::Sao_Paulo,
            Timezone::CAT => Africa::Harare,
            Timezone::UTC => Europe::Lisbon,
        }
    }

    pub fn options() -> [TimezoneOption; 32] {
        Timezone::all().map(|t| TimezoneOption {
            label: t.to_string(),
            value: t.into(),
        })
    }
}

impl Display for Timezone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let description = match self {
            Timezone::GMT => "Greenwich Mean Time (GMT)",
            Timezone::UTC => "Universal Coordinated Time (UTC)",
            Timezone::ECT => "European Central Time (GMT+1:00)",
            Timezone::EET => "Eastern European Time (GMT+2:00)",
            Timezone::ART => "(Arabic) Egypt Standard Time (GMT+2:00)",
            Timezone::EAT => "Eastern African Time (GMT+3:00)",
            Timezone::MET => "Middle East Time (GMT+3:30)",
            Timezone::NET => "Near East Time (GMT+4:00)",
            Timezone::PLT => "Pakistan Lahore Time (GMT+5:00)",
            Timezone::IST => "India Standard Time (GMT+5:30)",
            Timezone::BST => "Bangladesh Standard Time (GMT+6:00)",
            Timezone::VST => "Vietnam Standard Time (GMT+7:00)",
            Timezone::CTT => "China Taiwan Time (GMT+8:00)",
            Timezone::JST => "Japan Standard Time (GMT+9:00)",
            Timezone::ACT => "Australia Central Time (GMT+9:30)",
            Timezone::AET => "Australia Eastern Time (GMT+10:00)",
            Timezone::SST => "Solomon Standard Time (GMT+11:00)",
            Timezone::NST => "New Zealand Standard Time (GMT+12:00)",
            Timezone::MIT => "Midway Islands Time (GMT-11:00)",
            Timezone::HST => "Hawaii Standard Time (GMT-10:00)",
            Timezone::AST => "Alaska Standard Time (GMT-9:00)",
            Timezone::PST => "Pacific Standard Time (GMT-8:00)",
            Timezone::PNT => "Phoenix Standard Time (GMT-7:00)",
            Timezone::MST => "Mountain Standard Time (GMT-7:00)",
            Timezone::CST => "Central Standard Time (GMT-6:00)",
            Timezone::EST => "Eastern Standard Time (GMT-5:00)",
            Timezone::IET => "Indiana Eastern Standard Time (GMT-5:00)",
            Timezone::PRT => "Puerto Rico and US Virgin Islands Time (GMT-4:00)",
            Timezone::CNT => "Canada Newfoundland Time (GMT-3:30)",
            Timezone::AGT => "Argentina Standard Time (GMT-3:00)",
            Timezone::BET => "Brazil Eastern Time (GMT-3:00)",
            Timezone::CAT => "Central African Time (GMT-1:00)",
        };
        write!(f, "{}", description)
    }
}

impl From<String> for Timezone {
    fn from(value: String) -> Self {
        match value.as_str() {
            "GMT" => Timezone::GMT,
            "ECT" => Timezone::ECT,
            "EET" => Timezone::EET,
            "ART" => Timezone::ART,
            "EAT" => Timezone::EAT,
            "MET" => Timezone::MET,
            "NET" => Timezone::NET,
            "PLT" => Timezone::PLT,
            "IST" => Timezone::IST,
            "BST" => Timezone::BST,
            "VST" => Timezone::VST,
            "CTT" => Timezone::CTT,
            "JST" => Timezone::JST,
            "ACT" => Timezone::ACT,
            "AET" => Timezone::AET,
            "SST" => Timezone::SST,
            "NST" => Timezone::NST,
            "MIT" => Timezone::MIT,
            "HST" => Timezone::HST,
            "AST" => Timezone::AST,
            "PST" => Timezone::PST,
            "PNT" => Timezone::PNT,
            "MST" => Timezone::MST,
            "CST" => Timezone::CST,
            "EST" => Timezone::EST,
            "IET" => Timezone::IET,
            "PRT" => Timezone::PRT,
            "CNT" => Timezone::CNT,
            "AGT" => Timezone::AGT,
            "BET" => Timezone::BET,
            "CAT" => Timezone::CAT,
            _ => Timezone::UTC,
        }
    }
}

impl From<Timezone> for String {
    fn from(value: Timezone) -> Self {
        match value {
            Timezone::UTC => "UTC",
            Timezone::GMT => "GMT",
            Timezone::ECT => "ECT",
            Timezone::EET => "EET",
            Timezone::ART => "ART",
            Timezone::EAT => "EAT",
            Timezone::MET => "MET",
            Timezone::NET => "NET",
            Timezone::PLT => "PLT",
            Timezone::IST => "IST",
            Timezone::BST => "BST",
            Timezone::VST => "VST",
            Timezone::CTT => "CTT",
            Timezone::JST => "JST",
            Timezone::ACT => "ACT",
            Timezone::AET => "AET",
            Timezone::SST => "SST",
            Timezone::NST => "NST",
            Timezone::MIT => "MIT",
            Timezone::HST => "HST",
            Timezone::AST => "AST",
            Timezone::PST => "PST",
            Timezone::PNT => "PNT",
            Timezone::MST => "MST",
            Timezone::CST => "CST",
            Timezone::EST => "EST",
            Timezone::IET => "IET",
            Timezone::PRT => "PRT",
            Timezone::CNT => "CNT",
            Timezone::AGT => "AGT",
            Timezone::BET => "BET",
            Timezone::CAT => "CAT",
        }
        .to_string()
    }
}

impl From<Timezone> for i32 {
    fn from(value: Timezone) -> Self {
        let hours: f64 = match value {
            Timezone::UTC => 0.0,
            Timezone::GMT => 0.0,
            Timezone::ECT => 1.0,
            Timezone::EET => 2.0,
            Timezone::ART => 2.0,
            Timezone::EAT => 3.0,
            Timezone::MET => 3.5,
            Timezone::NET => 4.0,
            Timezone::PLT => 5.0,
            Timezone::IST => 5.5,
            Timezone::BST => 6.0,
            Timezone::VST => 7.0,
            Timezone::CTT => 8.0,
            Timezone::JST => 9.0,
            Timezone::ACT => 9.5,
            Timezone::AET => 10.0,
            Timezone::SST => 11.0,
            Timezone::NST => 12.0,
            Timezone::MIT => -11.0,
            Timezone::HST => -10.0,
            Timezone::AST => -9.0,
            Timezone::PST => -8.0,
            Timezone::PNT => -7.0,
            Timezone::MST => -7.0,
            Timezone::CST => -6.0,
            Timezone::EST => -5.0,
            Timezone::IET => -5.0,
            Timezone::PRT => -4.0,
            Timezone::CNT => -3.5,
            Timezone::AGT => -3.0,
            Timezone::BET => -3.0,
            Timezone::CAT => -1.0,
        };
        (hours * 60.0 * 60.0) as i32
    }
}
