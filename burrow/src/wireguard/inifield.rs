use std::str::FromStr;

pub struct IniField(String);

impl FromStr for IniField {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl From<IniField> for Vec<String> {
    fn from(field: IniField) -> Self {
        field.0.split(",").map(|s| s.to_string()).collect()
    }
}

impl From<IniField> for u32 {
    fn from(value: IniField) -> Self {
        value.0.parse().unwrap()
    }
}

impl From<IniField> for Option<u32> {
    fn from(value: IniField) -> Self {
        Some(value.0.parse().unwrap())
    }
}

impl From<IniField> for String {
    fn from(value: IniField) -> Self {
        value.0
    }
}

impl From<IniField> for Option<String> {
    fn from(value: IniField) -> Self {
        Some(value.0)
    }
}

impl<T> From<Option<T>> for IniField
where
    T: ToString,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => Self(v.to_string()),
            None => Self("".to_string()),
        }
    }
}

impl IniField {
    fn new(value: &str) -> Self {
        Self(value.to_string())
    }
}
