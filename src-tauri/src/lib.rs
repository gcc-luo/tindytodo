mod db;
mod diary_assets;

use db::{TaskDraft, TodoDb};
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, State};

#[tauri::command]
fn get_app_data(db: State<'_, TodoDb>) -> Result<db::AppData, String> {
    db.app_data()
}

#[tauri::command]
fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn get_diary_data(db: State<'_, TodoDb>) -> Result<db::DiaryData, String> {
    db.diary_data()
}

#[tauri::command]
fn put_diary_asset(
    app: tauri::AppHandle,
    db: State<'_, TodoDb>,
    id: String,
    entry_id: i64,
    mime_type: String,
    bytes: Vec<u8>,
) -> Result<db::DiaryAsset, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    diary_assets::save_bytes(&db, &data_dir, id, entry_id, mime_type, &bytes)
}

#[tauri::command]
fn import_diary_asset(
    app: tauri::AppHandle,
    db: State<'_, TodoDb>,
    id: String,
    entry_id: i64,
    path: String,
) -> Result<db::DiaryAsset, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    diary_assets::import_file(&db, &data_dir, id, entry_id, std::path::Path::new(&path))
}

#[tauri::command]
fn resolve_diary_asset(
    app: tauri::AppHandle,
    db: State<'_, TodoDb>,
    id: String,
) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    diary_assets::resolve(&db, &data_dir, &id)
}

#[tauri::command]
fn delete_diary_asset(
    app: tauri::AppHandle,
    db: State<'_, TodoDb>,
    id: String,
) -> Result<(), String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    diary_assets::delete(&db, &data_dir, &id)
}

#[tauri::command]
fn prune_diary_assets(
    app: tauri::AppHandle,
    db: State<'_, TodoDb>,
    entry_id: i64,
    keep_ids: Vec<String>,
) -> Result<(), String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    diary_assets::prune(&db, &data_dir, entry_id, keep_ids)
}

#[tauri::command]
fn create_diary_entry(db: State<'_, TodoDb>, entry_date: String) -> Result<db::DiaryEntry, String> {
    db.create_diary_entry(entry_date)
}

#[tauri::command]
fn save_diary_entry(
    db: State<'_, TodoDb>,
    id: i64,
    title: String,
    content_html: String,
    tag_ids: Vec<i64>,
) -> Result<db::DiaryEntry, String> {
    db.update_diary_entry(id, title, content_html, tag_ids)
}

#[tauri::command]
fn create_diary_tag(db: State<'_, TodoDb>, name: String) -> Result<db::DiaryTag, String> {
    db.create_diary_tag(name)
}

#[tauri::command]
fn rename_diary_tag(db: State<'_, TodoDb>, id: i64, name: String) -> Result<db::DiaryTag, String> {
    db.rename_diary_tag(id, name)
}

#[tauri::command]
fn delete_diary_tag(db: State<'_, TodoDb>, id: i64) -> Result<(), String> {
    db.delete_diary_tag(id)
}

#[tauri::command]
fn trash_diary_entry(db: State<'_, TodoDb>, id: i64) -> Result<(), String> {
    db.trash_diary_entry(id)
}

#[tauri::command]
fn restore_diary_entry(db: State<'_, TodoDb>, id: i64) -> Result<(), String> {
    db.restore_diary_entry(id)
}

#[tauri::command]
fn purge_diary_entry(app: tauri::AppHandle, db: State<'_, TodoDb>, id: i64) -> Result<(), String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let assets = db.purge_diary_entry(id)?;
    diary_assets::delete_many(&db, &data_dir, assets)
}

#[tauri::command]
fn get_work_settings(db: State<'_, TodoDb>) -> Result<db::WorkSettings, String> {
    db.work_settings()
}

#[tauri::command]
fn save_work_settings(db: State<'_, TodoDb>, settings: db::WorkSettings) -> Result<(), String> {
    db.save_work_settings(settings)
}

#[tauri::command]
fn create_task(db: State<'_, TodoDb>, draft: TaskDraft) -> Result<db::TodoTask, String> {
    db.create_task(draft)
}

#[tauri::command]
fn update_task(db: State<'_, TodoDb>, id: i64, draft: TaskDraft) -> Result<db::TodoTask, String> {
    db.update_task(id, draft)
}

#[tauri::command]
fn toggle_task(db: State<'_, TodoDb>, id: i64) -> Result<db::TodoTask, String> {
    db.toggle_task(id)
}

#[tauri::command]
fn delete_task(db: State<'_, TodoDb>, id: i64) -> Result<(), String> {
    db.delete_task(id)
}

#[tauri::command]
fn reorder_tasks(db: State<'_, TodoDb>, ids: Vec<i64>) -> Result<(), String> {
    db.reorder_tasks(ids)
}

#[tauri::command]
fn create_list(db: State<'_, TodoDb>, name: String) -> Result<db::TodoList, String> {
    db.create_list(name)
}

#[tauri::command]
fn rename_list(db: State<'_, TodoDb>, id: i64, name: String) -> Result<db::TodoList, String> {
    db.rename_list(id, name)
}

#[tauri::command]
fn delete_list(db: State<'_, TodoDb>, id: i64) -> Result<(), String> {
    db.delete_list(id)
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let database =
                TodoDb::open(&data_dir.join("tinytodo.sqlite3")).map_err(std::io::Error::other)?;
            if let Err(error) = diary_assets::purge_pending(&database, &data_dir) {
                eprintln!("清理待删除的日记图片失败，将在下次启动时重试：{error}");
            }
            app.manage(database);

            let show = MenuItem::with_id(app, "show", "打开 TinyTodo", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出 TinyTodo", true, None::<&str>)?;
            let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

            TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .expect("configured application icon")
                        .clone(),
                )
                .tooltip("TinyTodo")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_main(app),
                    "quit" => {
                        let _ = app.emit("app:quit-requested", ());
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_data,
            exit_app,
            get_diary_data,
            put_diary_asset,
            import_diary_asset,
            resolve_diary_asset,
            delete_diary_asset,
            prune_diary_assets,
            create_diary_entry,
            save_diary_entry,
            create_diary_tag,
            rename_diary_tag,
            delete_diary_tag,
            trash_diary_entry,
            restore_diary_entry,
            purge_diary_entry,
            get_work_settings,
            save_work_settings,
            create_task,
            update_task,
            toggle_task,
            delete_task,
            reorder_tasks,
            create_list,
            rename_list,
            delete_list,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TinyTodo");
}
