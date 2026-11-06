use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum ContentKind {
    Text = 0,
    Code = 1,
    Url = 2,
    Image = 3,
    Color = 4,
    Files = 5,
    Html = 6,
    Binary = 7,
    Audio = 8,
    Video = 9,
}

impl ContentKind {
    pub fn as_i32(self) -> i32 {
        self as i32
    }

    pub fn from_i32(val: i32) -> Result<Self, CoreError> {
        match val {
            0 => Ok(Self::Text),
            1 => Ok(Self::Code),
            2 => Ok(Self::Url),
            3 => Ok(Self::Image),
            4 => Ok(Self::Color),
            5 => Ok(Self::Files),
            6 => Ok(Self::Html),
            7 => Ok(Self::Binary),
            8 => Ok(Self::Audio),
            9 => Ok(Self::Video),
            other => Err(CoreError::InvalidContentKind(other)),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Code => "code",
            Self::Url => "url",
            Self::Image => "image",
            Self::Color => "color",
            Self::Files => "files",
            Self::Html => "html",
            Self::Binary => "binary",
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }
}

impl fmt::Display for ContentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl FromStr for ContentKind {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "code" => Ok(Self::Code),
            "url" | "link" => Ok(Self::Url),
            "image" => Ok(Self::Image),
            "color" => Ok(Self::Color),
            "files" | "file" => Ok(Self::Files),
            "html" => Ok(Self::Html),
            "binary" => Ok(Self::Binary),
            "audio" => Ok(Self::Audio),
            "video" => Ok(Self::Video),
            _ => Err(CoreError::UnknownKindLabel(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_i32() {
        for i in 0..=9 {
            let kind = ContentKind::from_i32(i).unwrap();
            assert_eq!(kind.as_i32(), i);
        }
        assert!(ContentKind::from_i32(10).is_err());
    }

    #[test]
    fn parse_label() {
        assert_eq!("code".parse::<ContentKind>().unwrap(), ContentKind::Code);
        assert_eq!("link".parse::<ContentKind>().unwrap(), ContentKind::Url);
        assert!("invalid".parse::<ContentKind>().is_err());
    }
}
