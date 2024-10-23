use std::collections::{BTreeMap, HashMap};

use camino::Utf8PathBuf;
use glob::glob;
use serde::{Deserialize, Serialize};

use crate::runtime::{CommandLine, ContainerSource};

pub(crate) mod openai;
pub(crate) mod runtime;

macro_rules! eval_if_in_filter {
    ($path:expr, $filter:expr, $action:expr) => {
        // include by default
        let mut include = true;
        // if filter is set
        if let Some(filter) = &$filter {
            // if it does not match, do not include
            if !$path.as_str().contains(filter) {
                include = false;
            }
        }
        if include {
            $action
        }
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Parameter {
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: String,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<String>>,
}

fn default_required() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Container {
    #[serde(flatten)]
    pub source: ContainerSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<String>>,
    #[serde(default = "default_force")]
    #[serde(skip_serializing_if = "is_false")]
    pub force: bool,
}

fn is_false(b: &bool) -> bool {
    *b == false
}

fn default_force() -> bool {
    false
}

impl Container {
    pub fn wrap(&self, cmdline: CommandLine) -> anyhow::Result<CommandLine> {
        let mut dockerized = CommandLine {
            sudo: false,
            app: which::which("docker")
                .map_err(|e| anyhow::anyhow!("docker executable not found: {}", e))?
                .to_string_lossy()
                .to_string(),
            app_in_path: true,
            args: vec!["run".to_string(), "--rm".to_string()],
        };

        // add volumes if any
        if let Some(volumes) = &self.volumes {
            for volume in volumes {
                dockerized.args.push(format!("-v{}", volume));
            }
        }

        // add any additional args
        if let Some(args) = &self.args {
            dockerized.args.extend(args.clone());
        }

        // add image
        dockerized.args.push(self.source.image().to_string());

        // add the original arguments
        dockerized.args.extend(cmdline.args);

        Ok(dockerized)
    }
}

// TODO: add optional parsers to reduce output tokens
// TODO: add python functions support

#[derive(Debug, Serialize, Deserialize)]
pub struct Function {
    pub description: String,
    pub parameters: BTreeMap<String, Parameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<Container>,
    #[serde(flatten)]
    pub execution: runtime::ExecutionContext,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Page {
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default = "String::new")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub functions: BTreeMap<String, Function>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default = "Vec::new")]
    pub categories: Vec<String>,
}

impl Page {
    fn preprocess(path: &Utf8PathBuf, text: String) -> anyhow::Result<String> {
        let base_path = path.parent().unwrap().to_string();

        Ok(text.replace("${cwd}", &base_path))
    }

    pub fn from_path(path: &Utf8PathBuf) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let text = Self::preprocess(&path, text)?;
        let page = serde_yaml::from_str(&text)?;
        Ok(page)
    }
}

#[derive(Debug)]
pub struct Book {
    pub pages: BTreeMap<Utf8PathBuf, Page>,
}

impl Book {
    pub fn from_path(path: Utf8PathBuf, filter: Option<String>) -> anyhow::Result<Self> {
        let mut page_paths = Vec::new();

        let path = Utf8PathBuf::from(
            shellexpand::full(path.as_str())
                .map_err(|e| anyhow::anyhow!("failed to expand path: {}", e))?
                .into_owned(),
        );

        if path.is_file() {
            eval_if_in_filter!(path, filter, page_paths.push(path.to_path_buf()));
        } else if path.is_dir() {
            for entry in glob(path.join("**/*.yml").as_str())? {
                match entry {
                    Ok(entry_path) => {
                        if let Ok(utf8_path) = Utf8PathBuf::from_path_buf(entry_path) {
                            eval_if_in_filter!(utf8_path, filter, page_paths.push(utf8_path));
                        } else {
                            log::error!("failed to convert path to Utf8PathBuf");
                        }
                    }
                    Err(e) => {
                        log::error!("error in glob pattern: {:?}", e);
                    }
                }
            }
        }

        if page_paths.is_empty() {
            return Err(anyhow::anyhow!("no pages found in {:?}", path));
        }

        log::debug!("loading {} pages from {:?}", page_paths.len(), path);

        let mut pages = BTreeMap::new();
        let mut function_names = HashMap::new();

        for page_path in page_paths {
            let mut page = Page::from_path(&page_path)?;

            // if name is not set, use the file name
            if page.name.is_empty() {
                page.name = page_path.file_stem().unwrap().to_string();
            }

            // if categories are not set, use the path components
            if page.categories.is_empty() {
                page.categories = page_path
                    .strip_prefix(&path)
                    .unwrap()
                    .parent()
                    .map(|p| {
                        p.components()
                            .map(|c| c.as_os_str().to_string_lossy().into_owned())
                            .collect()
                    })
                    .unwrap_or_default();
            }

            // make sure function names are unique
            let mut renames = HashMap::new();
            for func_name in page.functions.keys() {
                if function_names.contains_key(func_name) {
                    let new_func_name = format!("{}_{}", &page.name, func_name);
                    if !function_names.contains_key(&new_func_name) {
                        log::warn!(
                            "function name {} in {:?} is not unique, renaming to {}",
                            func_name,
                            page_path,
                            new_func_name
                        );
                        renames.insert(func_name.clone(), new_func_name.clone());
                    } else {
                        return Err(anyhow::anyhow!(
                            "function name {} in {:?} is not unique",
                            func_name,
                            page_path
                        ));
                    }
                }
                function_names.insert(func_name.clone(), 1);
            }

            for (old_name, new_name) in renames {
                let function = page.functions.remove(&old_name).unwrap();
                page.functions.insert(new_name, function);
            }

            pages.insert(page_path, page);
        }

        Ok(Self { pages })
    }

    pub fn size(&self) -> usize {
        self.pages.len()
    }

    pub fn get_function<'a>(&'a self, name: &str) -> anyhow::Result<runtime::FunctionRef<'a>> {
        for (page_path, page) in &self.pages {
            if let Some(function) = page.functions.get(name) {
                return Ok(runtime::FunctionRef {
                    name: name.to_owned(),
                    path: page_path,
                    page,
                    function,
                });
            }
        }

        Err(anyhow::anyhow!("function {} not found", name))
    }

    // TODO: add support for different flavors? https://github.com/groq/groq-api-cookbook/blob/main/tutorials/function-calling-101-ecommerce/Function-Calling-101-Ecommerce.ipynb
    pub fn as_tools(&self, filter: Option<String>) -> Vec<openai::Tool> {
        let mut tools = Vec::new();

        for (page_path, page) in &self.pages {
            eval_if_in_filter!(
                page_path,
                filter,
                tools.extend(<&Page as Into<Vec<openai::Tool>>>::into(page))
            );
        }

        tools
    }
}
