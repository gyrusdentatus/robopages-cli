use std::{
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

use camino::Utf8PathBuf;

use super::InstallArgs;

fn extract_archive_without_intermediate_folder(
    mut archive: zip::ZipArchive<File>,
    target_path: &Path,
) -> io::Result<()> {
    // Iterate through each entry in the ZIP archive
    for i in 0..archive.len() {
        let mut file_in_zip = archive.by_index(i)?;
        let file_path = file_in_zip.mangled_name();

        // Skip directories by default
        if file_in_zip.is_dir() {
            continue;
        }

        // Strip the first component of the file path (e.g., intermediate-folder-name)
        let stripped_path = file_path.iter().skip(1).collect::<PathBuf>();
        let target_file_path = target_path.join(stripped_path);

        // Create parent directories as needed
        if let Some(parent) = target_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write the file to the target path
        let mut outfile = File::create(&target_file_path)?;
        io::copy(&mut file_in_zip, &mut outfile)?;
    }

    Ok(())
}

fn extract_archive(archive_path: &Path, target_path: &Path) -> io::Result<()> {
    log::info!("extracting to {:?}", target_path);

    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // check if all files share the same prefix
    let file_names: Vec<_> = archive.file_names().collect();
    let mut single_root_folder = false;

    if !file_names.is_empty() {
        if let Some(first_name) = file_names.first() {
            if let Some(first_prefix) = first_name.split('/').next() {
                single_root_folder = file_names
                    .iter()
                    .all(|name| name.split('/').next() == Some(first_prefix))
            }
        }
    }

    if single_root_folder {
        // if the archive comes from a github repository, it will have a single root folder
        // so we can extract it without the intermediate folder
        extract_archive_without_intermediate_folder(archive, target_path)?;
    } else {
        // otherwise, we extract the archive as it is
        archive.extract(target_path)?;
    }

    Ok(())
}

pub(crate) async fn install(args: InstallArgs) -> anyhow::Result<()> {
    let path = Utf8PathBuf::from(
        shellexpand::full(args.path.as_str())
            .map_err(|e| anyhow::anyhow!("failed to expand path: {}", e))?
            .into_owned(),
    );
    if path.exists() {
        return Err(anyhow::anyhow!("{:?} already exists", path));
    }

    if args.source.ends_with(".zip") {
        // install from zip archive
        log::info!("extracting archive {} to {:?}", &args.source, &path);
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&args.source)?)?;
        zip.extract(path)?;
    } else {
        // install from github repository
        let source = if !args.source.contains("://") {
            format!(
                "https://github.com/{}/archive/refs/heads/main.zip",
                &args.source
            )
        } else {
            format!("{}/archive/refs/heads/main.zip", &args.source)
        };

        log::info!("downloading robopages from {} ...", source);

        let temp_file = tempfile::NamedTempFile::new()?;
        let mut response = reqwest::get(&source).await?;
        let mut file = std::fs::File::create(temp_file.path())?;

        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk)?;
        }

        extract_archive(temp_file.path(), path.as_std_path())?;
    }

    Ok(())
}
