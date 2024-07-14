use std::str::FromStr;

use anyhow::{Error, Result};

pub struct IniField(String);

impl FromStr for IniField {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl TryFrom<IniField> for Vec<String> {
    type Error = Error;

    fn try_from(field: IniField) -> Result<Self, Self::Error> {
        Ok(field.0.split(',').map(|s| s.trim().to_string()).collect())
    }
}

impl TryFrom<IniField> for u32 {
    type Error = Error;

    fn try_from(value: IniField) -> Result<Self, Self::Error> {
        value.0.parse().map_err(Error::from)
    }
}

impl TryFrom<IniField> for Option<u32> {
    type Error = Error;

    fn try_from(value: IniField) -> Result<Self, Self::Error> {
        if value.0.is_empty() {
            Ok(None)
        } else {
            value.0.parse().map(Some).map_err(Error::from)
        }
    }
}

impl TryFrom<IniField> for String {
    type Error = Error;

    fn try_from(value: IniField) -> Result<Self, Self::Error> {
        Ok(value.0)
    }
}

impl TryFrom<IniField> for Option<String> {
    type Error = Error;

    fn try_from(value: IniField) -> Result<Self, Self::Error> {
        if value.0.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value.0))
        }
    }
}

impl<T> TryFrom<Option<T>> for IniField
where
    T: ToString,
{
    type Error = Error;

    fn try_from(value: Option<T>) -> Result<Self, Self::Error> {
        Ok(match value {
            Some(v) => Self(v.to_string()),
            None => Self(String::new()),
        })
    }
}

impl IniField {
    fn new(value: &str) -> Self {
        Self(value.to_string())
    }
}
