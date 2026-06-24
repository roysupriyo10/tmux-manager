use std::fs;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas/config.schema.json");
    let json = tmux_manager::schema_gen::store_schema_json()?;

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, json)?;
    eprintln!("wrote {}", out.display());
    Ok(())
}
