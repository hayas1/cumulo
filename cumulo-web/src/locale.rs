use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::i18n::Locale;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    En,
    Ja,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Ja];

    pub fn as_str(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ja => "ja",
        }
    }
}

impl FromStr for Lang {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "en" => Ok(Lang::En),
            "ja" => Ok(Lang::Ja),
            _ => Err(()),
        }
    }
}

impl From<Locale> for Lang {
    fn from(locale: Locale) -> Self {
        match locale {
            Locale::en => Lang::En,
            Locale::ja => Lang::Ja,
        }
    }
}

impl From<Lang> for Locale {
    fn from(lang: Lang) -> Self {
        match lang {
            Lang::En => Locale::en,
            Lang::Ja => Locale::ja,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_round_trips() {
        for lang in Lang::ALL {
            assert_eq!(lang.as_str().parse(), Ok(lang));
        }
    }

    #[test]
    fn converts_through_i18n_locale() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from(Locale::from(lang)), lang);
        }
    }
}
