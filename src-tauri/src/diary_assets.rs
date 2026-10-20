use crate::db::{DiaryAsset, TodoDb};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

fn extension_for_mime(mime: &str) -> Result<&'static str, String> {
    match mime {
        "image/png" => Ok("png"),
        "image/jpeg" => Ok("jpg"),
        "image/gif" => Ok("gif"),
        "image/webp" => Ok("webp"),
        _ => Err("只支持 PNG、JPEG、GIF 和 WebP 图片".to_string()),
    }
}

fn mime_for_extension(extension: &str) -> Result<&'static str, String> {
    match extension.to_ascii_lowercase().as_str() {
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "gif" => Ok("image/gif"),
        "webp" => Ok("image/webp"),
        _ => Err("只支持 PNG、JPEG、GIF 和 WebP 图片".to_string()),
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    let bytes = id.as_bytes();
    if bytes.len() != 36
        || [8, 13, 18, 23].iter().any(|index| bytes[*index] != b'-')
        || bytes
            .iter()
            .enumerate()
            .any(|(index, value)| ![8, 13, 18, 23].contains(&index) && !value.is_ascii_hexdigit())
    {
        return Err("日记图片标识无效".to_string());
    }
    Ok(())
}

fn validate_image(mime: &str, bytes: &[u8]) -> Result<&'static str, String> {
    let extension = extension_for_mime(mime)?;
    let valid = match mime {
        "image/png" => bytes.starts_with(PNG),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "image/webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
        _ => false,
    };
    if !valid {
        return Err("图片内容与文件类型不匹配".to_string());
    }
    Ok(extension)
}

fn assets_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("diary-assets")
}

fn asset_path(app_data_dir: &Path, asset: &DiaryAsset) -> Result<PathBuf, String> {
    validate_id(&asset.id)?;
    let extension = asset
        .relative_path
        .strip_prefix(&format!("{}.", asset.id))
        .ok_or_else(|| "日记图片路径无效".to_string())?;
    mime_for_extension(extension)?;
    Ok(assets_dir(app_data_dir).join(&asset.relative_path))
}

pub fn save_bytes(
    db: &TodoDb,
    app_data_dir: &Path,
    id: String,
    entry_id: i64,
    mime_type: String,
    bytes: &[u8],
) -> Result<DiaryAsset, String> {
    validate_id(&id)?;
    let extension = validate_image(&mime_type, bytes)?;
    let directory = assets_dir(app_data_dir);
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let relative_path = format!("{id}.{extension}");
    let path = directory.join(&relative_path);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| error.to_string())?;
    if let Err(error) = file.write_all(bytes) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error.to_string());
    }
    drop(file);
    match db.register_diary_asset(id, entry_id, relative_path, mime_type) {
        Ok(asset) => Ok(asset),
        Err(error) => {
            let _ = fs::remove_file(path);
            Err(error)
        }
    }
}

pub fn import_file(
    db: &TodoDb,
    app_data_dir: &Path,
    id: String,
    entry_id: i64,
    source_path: &Path,
) -> Result<DiaryAsset, String> {
    let extension = source_path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "无法识别图片文件类型".to_string())?;
    let mime_type = mime_for_extension(extension)?;
    let bytes = fs::read(source_path).map_err(|error| error.to_string())?;
    save_bytes(
        db,
        app_data_dir,
        id,
        entry_id,
        mime_type.to_string(),
        &bytes,
    )
}

pub fn resolve(db: &TodoDb, app_data_dir: &Path, id: &str) -> Result<String, String> {
    validate_id(id)?;
    let asset = db
        .diary_asset(id)?
        .ok_or_else(|| "找不到这张日记图片".to_string())?;
    let path = asset_path(app_data_dir, &asset)?;
    Ok(path.to_string_lossy().into_owned())
}

pub fn delete(db: &TodoDb, app_data_dir: &Path, id: &str) -> Result<(), String> {
    validate_id(id)?;
    let Some(asset) = db.diary_asset(id)? else {
        return Ok(());
    };
    let path = asset_path(app_data_dir, &asset)?;
    match fs::remove_file(path) {
        Ok(()) => db.remove_diary_asset_record(id),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            db.remove_diary_asset_record(id)
        }
        Err(error) => Err(error.to_string()),
    }
}

pub fn delete_many(
    db: &TodoDb,
    app_data_dir: &Path,
    assets: Vec<DiaryAsset>,
) -> Result<(), String> {
    let mut first_error = None;
    for asset in assets {
        if let Err(error) = delete(db, app_data_dir, &asset.id) {
            first_error.get_or_insert(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

pub fn prune(
    db: &TodoDb,
    app_data_dir: &Path,
    entry_id: i64,
    keep_ids: Vec<String>,
) -> Result<(), String> {
    let assets = db.prune_diary_assets(entry_id, keep_ids)?;
    delete_many(db, app_data_dir, assets)
}

pub fn purge_pending(db: &TodoDb, app_data_dir: &Path) -> Result<(), String> {
    delete_many(db, app_data_dir, db.pending_diary_assets()?)
}

#[cfg(test)]
mod tests {
    use super::{asset_path, delete, import_file, prune, save_bytes, validate_id};
    use crate::db::{DiaryAsset, TodoDb};
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn db() -> TodoDb {
        TodoDb::open(Path::new(":memory:")).unwrap()
    }

    const ID: &str = "01234567-89ab-cdef-0123-456789abcdef";

    #[test]
    fn rejects_path_traversal_and_malformed_ids() {
        assert!(validate_id("../image.png").is_err());
        assert!(validate_id("0123456789abcdef0123456789abcdef").is_err());
        let unsafe_asset = DiaryAsset {
            id: "01234567-89ab-cdef-0123-456789abcdef".into(),
            entry_id: Some(1),
            relative_path: "../../outside.png".into(),
            mime_type: "image/png".into(),
            pending_delete: false,
        };
        assert!(asset_path(Path::new("C:/app-data"), &unsafe_asset).is_err());
    }

    #[test]
    fn writes_supported_image_bytes_and_rejects_mime_mismatch() {
        let folder = tempdir().unwrap();
        let db = db();
        let entry = db.create_diary_entry("2026-10-20".into()).unwrap();
        let image = save_bytes(
            &db,
            folder.path(),
            ID.into(),
            entry.id,
            "image/png".into(),
            b"\x89PNG\r\n\x1a\nrest",
        )
        .unwrap();
        assert!(folder
            .path()
            .join("diary-assets")
            .join(&image.relative_path)
            .is_file());
        assert!(save_bytes(
            &db,
            folder.path(),
            ID.into(),
            entry.id,
            "image/png".into(),
            b"\x89PNG\r\n\x1a\nreplacement"
        )
        .is_err());
        assert!(save_bytes(
            &db,
            folder.path(),
            "11234567-89ab-cdef-0123-456789abcdef".into(),
            entry.id,
            "image/png".into(),
            b"not png"
        )
        .is_err());
        assert!(save_bytes(
            &db,
            folder.path(),
            "21234567-89ab-cdef-0123-456789abcdef".into(),
            entry.id,
            "image/svg+xml".into(),
            b"<svg/>"
        )
        .is_err());
    }

    #[test]
    fn imports_only_a_valid_image_file() {
        let folder = tempdir().unwrap();
        let db = db();
        let entry = db.create_diary_entry("2026-10-20".into()).unwrap();
        let source = folder.path().join("photo.png");
        fs::write(&source, b"\x89PNG\r\n\x1a\nrest").unwrap();
        assert!(import_file(&db, folder.path(), ID.into(), entry.id, &source).is_ok());
    }

    #[test]
    fn pruning_marks_and_removes_unreferenced_assets() {
        let folder = tempdir().unwrap();
        let db = db();
        let entry = db.create_diary_entry("2026-10-20".into()).unwrap();
        let asset = save_bytes(
            &db,
            folder.path(),
            ID.into(),
            entry.id,
            "image/png".into(),
            b"\x89PNG\r\n\x1a\nrest",
        )
        .unwrap();
        let path = folder
            .path()
            .join("diary-assets")
            .join(&asset.relative_path);

        prune(&db, folder.path(), entry.id, Vec::new()).unwrap();

        assert!(!path.exists());
        assert!(db.diary_asset(ID).unwrap().is_none());
        delete(&db, folder.path(), ID).unwrap();
    }
}
