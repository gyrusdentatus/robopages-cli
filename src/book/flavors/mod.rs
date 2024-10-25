use std::collections::HashMap;

pub(crate) mod nerve;
pub(crate) mod openai;

#[derive(Default, Debug)]
pub(crate) enum Flavor {
    #[default]
    OpenAI,
    Nerve,
}

impl Flavor {
    pub fn from_string(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Flavor::OpenAI),
            "nerve" => Ok(Flavor::Nerve),
            _ => Err(anyhow!("unknown flavor: {}", s)),
        }
    }

    pub fn from_map_or_default(query: &HashMap<String, String>) -> anyhow::Result<Self> {
        query
            .get("flavor")
            .map_or(Ok(Flavor::default()), |s| Self::from_string(s))
    }

    pub fn is_openai(&self) -> bool {
        matches!(self, Flavor::OpenAI)
    }

    pub fn is_nerve(&self) -> bool {
        matches!(self, Flavor::Nerve)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flavor_from_string() {
        assert!(matches!(Flavor::from_string("openai"), Ok(Flavor::OpenAI)));
        assert!(matches!(Flavor::from_string("OpenAI"), Ok(Flavor::OpenAI)));
        assert!(matches!(Flavor::from_string("OPENAI"), Ok(Flavor::OpenAI)));

        assert!(matches!(Flavor::from_string("nerve"), Ok(Flavor::Nerve)));
        assert!(matches!(Flavor::from_string("Nerve"), Ok(Flavor::Nerve)));
        assert!(matches!(Flavor::from_string("NERVE"), Ok(Flavor::Nerve)));

        assert!(Flavor::from_string("unknown").is_err());
        assert!(Flavor::from_string("").is_err());
    }

    #[test]
    fn test_flavor_from_map_or_default() {
        let mut map = HashMap::new();

        // Test default case
        assert!(matches!(
            Flavor::from_map_or_default(&map),
            Ok(Flavor::OpenAI)
        ));

        // Test valid flavor
        map.insert("flavor".to_string(), "openai".to_string());
        assert!(matches!(
            Flavor::from_map_or_default(&map),
            Ok(Flavor::OpenAI)
        ));

        // Test invalid flavor
        map.insert("flavor".to_string(), "unknown".to_string());
        assert!(Flavor::from_map_or_default(&map).is_err());

        // Test empty string
        map.insert("flavor".to_string(), "".to_string());
        assert!(Flavor::from_map_or_default(&map).is_err());
    }
}
