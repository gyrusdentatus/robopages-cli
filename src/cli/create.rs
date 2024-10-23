use camino::Utf8PathBuf;

use crate::book::templates::Template;

pub(crate) async fn create(template: Template, name: Utf8PathBuf) -> anyhow::Result<()> {
    if name.exists() {
        return Err(anyhow::anyhow!("{:?} already exists", name));
    }

    for parts in template.get_data()? {
        if let Some(part_name) = parts.name {
            let asset = name.parent().unwrap().join(part_name);
            log::info!("creating asset {:?}", asset.to_string());

            std::fs::write(asset, parts.data)?;
        } else {
            log::info!("creating {:?} from template {}", name, template.to_string());

            std::fs::write(&name, parts.data)?;
        }
    }

    Ok(())
}
