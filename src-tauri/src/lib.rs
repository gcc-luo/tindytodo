mod db;

use db::{TaskDraft, TodoDb};
use serde::Serialize;
use std::sync::Mutex;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, State, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[derive(Clone, Serialize)]
struct ShortcutInfo {
    label: String,
    registered: bool,
}

struct HotkeyState {
    info: Mutex<ShortcutInfo>,
}

fn shortcut_candidates() -> Vec<(Shortcut, &'static str)> {
    vec![
        (
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space),
            "Ctrl + Shift + Space",
        ),
        (
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space),
            "Ctrl + Alt + Space",
        ),
        (
            Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyT),
            "Alt + Shift + T",
        ),
    ]
}

#[tauri::command]
fn get_app_data(db: State<'_, TodoDb>) -> Result<db::AppData, String> {
    db.app_data()
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

#[tauri::command]
fn hide_quick_add(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|error| error.to_string())
}

#[tauri::command]
fn get_shortcut_info(state: State<'_, HotkeyState>) -> Result<ShortcutInfo, String> {
    state
        .info
        .lock()
        .map(|info| info.clone())
        .map_err(|_| "快捷键设置暂时不可用".to_string())
}

#[tauri::command]
fn set_shortcut(
    app: tauri::AppHandle,
    state: State<'_, HotkeyState>,
    label: String,
) -> Result<ShortcutInfo, String> {
    let (shortcut, label) = shortcut_candidates()
        .into_iter()
        .find(|(_, candidate_label)| *candidate_label == label)
        .ok_or_else(|| "不支持这个快捷键".to_string())?;
    let previous = state
        .info
        .lock()
        .map_err(|_| "快捷键设置暂时不可用".to_string())?
        .clone();

    app.global_shortcut()
        .unregister_all()
        .map_err(|error| error.to_string())?;
    if let Err(error) = app.global_shortcut().register(shortcut) {
        if previous.registered {
            if let Some((previous_shortcut, _)) = shortcut_candidates()
                .into_iter()
                .find(|(_, candidate_label)| *candidate_label == previous.label)
            {
                let _ = app.global_shortcut().register(previous_shortcut);
            }
        }
        return Err(format!("这个快捷键无法注册：{error}"));
    }

    let next = ShortcutInfo {
        label: label.to_string(),
        registered: true,
    };
    *state
        .info
        .lock()
        .map_err(|_| "快捷键设置暂时不可用".to_string())? = next.clone();
    Ok(next)
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_quick_add(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("quick-add") {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("quick-add:show", ());
    }
}

pub fn run() {
    let trigger_shortcuts: Vec<Shortcut> = shortcut_candidates()
        .into_iter()
        .map(|(shortcut, _)| shortcut)
        .collect();

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, pressed, event| {
                    if trigger_shortcuts.contains(pressed)
                        && event.state() == ShortcutState::Pressed
                    {
                        show_quick_add(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let database =
                TodoDb::open(&data_dir.join("tinytodo.sqlite3")).map_err(std::io::Error::other)?;
            app.manage(database);

            let mut shortcut_info = ShortcutInfo {
                label: "未注册".to_string(),
                registered: false,
            };
            for (shortcut, label) in shortcut_candidates() {
                match app.global_shortcut().register(shortcut) {
                    Ok(()) => {
                        shortcut_info = ShortcutInfo {
                            label: label.to_string(),
                            registered: true,
                        };
                        break;
                    }
                    Err(error) => eprintln!("快捷键 {label} 不可用：{error}"),
                }
            }
            app.manage(HotkeyState {
                info: Mutex::new(shortcut_info),
            });

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
                    "quit" => app.exit(0),
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
            create_task,
            update_task,
            toggle_task,
            delete_task,
            reorder_tasks,
            create_list,
            rename_list,
            delete_list,
            hide_quick_add,
            get_shortcut_info,
            set_shortcut
        ])
        .run(tauri::generate_context!())
        .expect("error while running TinyTodo");
}
