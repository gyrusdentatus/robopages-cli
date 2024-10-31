use crate::{book::Book, runtime::CommandLine};

use super::ValidateArgs;

pub(crate) async fn validate(args: ValidateArgs) -> anyhow::Result<()> {
    let book = Book::from_path(args.path.clone(), None)?;

    // we need at least one page
    if book.pages.is_empty() {
        return Err(anyhow::anyhow!("no pages found in {:?}", &args.path));
    }

    for (page_path, page) in book.pages {
        log::info!("validating {:?} ...", page_path);

        // and at least one function per page, at least what's the point of the page?
        if page.functions.is_empty() {
            return Err(anyhow::anyhow!("no functions found in {:?}", page_path));
        } else if page.name.is_empty() {
            // set by Book::from_path if not specified
            return Err(anyhow::anyhow!("page name is empty in {:?}", page_path));
        } else if page.categories.is_empty() {
            // set by Book::from_path if not specified
            return Err(anyhow::anyhow!(
                "page categories are empty in {:?}",
                page_path
            ));
        }

        for (func_name, func) in page.functions {
            // the model needs at least a name and a description
            if func_name.is_empty() {
                return Err(anyhow::anyhow!("function name is empty in {:?}", page_path));
            } else if func.description.is_empty() {
                return Err(anyhow::anyhow!(
                    "function description is empty in {:?}",
                    page_path
                ));
            }

            if func.parameters.is_empty() {
                return Err(anyhow::anyhow!(
                    "function {} parameters are empty in {:?}",
                    func_name,
                    page_path
                ));
            }

            // make sure the function resolves to a valid command line
            let cmdline = func.execution.get_command_line().map_err(|e| {
                anyhow::anyhow!(
                    "error while getting command line for function {}: {}",
                    func_name,
                    e
                )
            })?;

            if cmdline.is_empty() {
                return Err(anyhow::anyhow!(
                    "command line is empty for function {} in {:?}",
                    func_name,
                    page_path
                ));
            }

            let cmdline = CommandLine::from_vec(&cmdline).map_err(|e| {
                anyhow::anyhow!(
                    "error while parsing command line for function {}: {}",
                    func_name,
                    e
                )
            })?;

            // validate container requirements - a container is required if:
            let container = if !cmdline.app_in_path {
                // the binary is not in $PATH
                if let Some(container) = &func.container {
                    Some(container)
                } else {
                    return Err(anyhow::anyhow!(
                        "binary for function {} in {:?} not in $PATH and container not specified",
                        func_name,
                        page_path
                    ));
                }
            } else if func.container.is_some() && func.container.as_ref().unwrap().force {
                // it's set and forced
                Some(func.container.as_ref().unwrap())
            } else {
                None
            };

            // validate the container if any
            if let Some(container) = container {
                if args.skip_docker {
                    // or not :P
                    log::warn!("skipping container resolution for function {}", func_name);
                } else {
                    // this will pull or build the image
                    container.resolve().await.map_err(|e| {
                        anyhow::anyhow!(
                            "error while resolving container for function {} in {}: {}",
                            func_name,
                            page_path,
                            e
                        )
                    })?;

                    // if volumes are defined make sure they exist
                    if let Some(volumes) = &container.volumes {
                        for volume in volumes {
                            let (on_host, on_guest) =
                                volume.split_once(':').unwrap_or((volume, volume));

                            let on_host = shellexpand::full(on_host)
                                .map_err(|e| {
                                    anyhow::anyhow!(
                                        "error while expanding volume path for function {}: {}",
                                        func_name,
                                        e
                                    )
                                })?
                                .to_string();

                            if !std::path::Path::new(&on_host).exists() {
                                return Err(anyhow::anyhow!(
                                    "page {}, function {}, path {} for volume '{}' does not exist",
                                    page_path,
                                    func_name,
                                    on_host,
                                    on_guest
                                ));
                            }
                        }
                    }
                }
            }

            log::info!("  {} - ok", func_name);
            log::debug!("    cmdline = {:?}", cmdline);
            if let Some(container) = container {
                log::debug!("    container = {:?}", container);
            }
        }
    }

    Ok(())
}
