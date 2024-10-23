use clap::ValueEnum;
use include_dir::{include_dir, Dir};
use lazy_regex::{lazy_regex, Lazy};
use regex::Regex;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/book/templates");

static ASSETS_REF_PARSER: Lazy<Regex> = lazy_regex!(r"(?m)\$\{cwd\}/(.+)");

pub(crate) struct TemplateData {
    pub(crate) name: Option<String>,
    pub(crate) data: &'static str,
}

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum Template {
    Basic,
    DockerImage,
    DockerBuild,
}

impl Template {
    pub fn get_data(&self) -> anyhow::Result<Vec<TemplateData>> {
        let base_name = self.to_string().to_lowercase();
        let template_name = format!("{}.yml", &base_name);
        let template_data = TEMPLATES
            .get_file(&template_name)
            .ok_or_else(|| anyhow::anyhow!("template not found: {}", template_name))?
            .contents_utf8()
            .ok_or_else(|| {
                anyhow::anyhow!("failed to read template file as utf8: {}", template_name)
            })?;

        let mut parts = vec![TemplateData {
            name: None,
            data: template_data,
        }];

        // check if the template references any assets in ${cwd}
        let caps = ASSETS_REF_PARSER.captures(template_data);
        if let Some(caps) = caps {
            let asset_name = caps.get(1).unwrap().as_str();
            let asset = TEMPLATES.get_file(asset_name).unwrap();
            parts.push(TemplateData {
                name: Some(asset_name.to_string()),
                data: asset.contents_utf8().unwrap(),
            });
        }

        Ok(parts)
    }
}

impl ToString for Template {
    fn to_string(&self) -> String {
        match self {
            Template::Basic => "basic".to_string(),
            Template::DockerImage => "docker_image".to_string(),
            Template::DockerBuild => "docker_build".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::Page;

    #[test]
    fn test_templates_deserialize() {
        for entry in TEMPLATES.files() {
            let template_name = entry.path().file_stem().unwrap().to_str().unwrap();
            if template_name.ends_with(".yml") {
                let yaml_content = entry.contents_utf8().unwrap();
                let page: Page = serde_yaml::from_str(yaml_content).unwrap_or_else(|e| {
                    panic!(
                        "failed to deserialize template '{}': {:?}",
                        template_name, e
                    )
                });

                assert!(
                    !page.functions.is_empty(),
                    "template '{}' has no functions",
                    template_name
                );
            }
        }
    }
}
