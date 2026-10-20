use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoTask {
    pub id: i64,
    pub title: String,
    pub note: Option<String>,
    pub list_id: Option<i64>,
    pub priority: i64,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub status: i64,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoList {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryEntry {
    pub id: i64,
    pub entry_date: String,
    pub title: String,
    pub content_html: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub tag_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryTag {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryData {
    pub entries: Vec<DiaryEntry>,
    pub tags: Vec<DiaryTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryAsset {
    pub id: String,
    pub entry_id: Option<i64>,
    pub relative_path: String,
    pub mime_type: String,
    pub pending_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDraft {
    pub title: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub list_id: Option<i64>,
    #[serde(default)]
    pub priority: i64,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub due_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppData {
    pub tasks: Vec<TodoTask>,
    pub lists: Vec<TodoList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkReminderSettings {
    pub enabled: bool,
    pub time: String,
    pub repeat: String,
    pub date: Option<String>,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSettings {
    pub lunch: WorkReminderSettings,
    #[serde(rename = "offWork")]
    pub off_work: WorkReminderSettings,
}

pub struct TodoDb {
    connection: Mutex<Connection>,
}

impl TodoDb {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(db_error)?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(db_error)?;
        connection
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS todo_list (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS todo (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                note TEXT,
                list_id INTEGER REFERENCES todo_list(id) ON DELETE SET NULL,
                priority INTEGER NOT NULL DEFAULT 0 CHECK (priority IN (0, 1)),
                due_date TEXT,
                due_time TEXT,
                status INTEGER NOT NULL DEFAULT 0 CHECK (status IN (0, 1)),
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                completed_at TEXT
            );
            CREATE TABLE IF NOT EXISTS app_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS diary_entry (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                entry_date TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                content_html TEXT NOT NULL DEFAULT '<p></p>',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                deleted_at TEXT
            );
            CREATE TABLE IF NOT EXISTS diary_tag (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS diary_entry_tag (
                entry_id INTEGER NOT NULL REFERENCES diary_entry(id) ON DELETE CASCADE,
                tag_id INTEGER NOT NULL REFERENCES diary_tag(id) ON DELETE CASCADE,
                PRIMARY KEY (entry_id, tag_id)
            );
            CREATE TABLE IF NOT EXISTS diary_asset (
                id TEXT PRIMARY KEY,
                entry_id INTEGER REFERENCES diary_entry(id) ON DELETE SET NULL,
                relative_path TEXT NOT NULL UNIQUE,
                mime_type TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                pending_delete INTEGER NOT NULL DEFAULT 0 CHECK (pending_delete IN (0, 1))
            );
            CREATE INDEX IF NOT EXISTS idx_todo_status_due ON todo(status, due_date, sort_order);
            CREATE INDEX IF NOT EXISTS idx_todo_list ON todo(list_id, status, sort_order);
            CREATE INDEX IF NOT EXISTS idx_diary_entry_date ON diary_entry(entry_date DESC, created_at DESC, id DESC);
            CREATE INDEX IF NOT EXISTS idx_diary_entry_tag_tag ON diary_entry_tag(tag_id, entry_id);
            CREATE INDEX IF NOT EXISTS idx_diary_asset_entry ON diary_asset(entry_id, pending_delete);
            ",
            )
            .map_err(db_error)?;

        let seeded_defaults = connection
            .execute(
                "INSERT OR IGNORE INTO app_meta (key, value) VALUES ('default_lists_seeded', '1')",
                [],
            )
            .map_err(db_error)?;
        if seeded_defaults > 0 {
            connection
                .execute_batch(
                    "INSERT INTO todo_list (name, sort_order) VALUES ('工作', 0), ('学习', 1), ('生活', 2);",
                )
                .map_err(db_error)?;
        }

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn app_data(&self) -> Result<AppData, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        Ok(AppData {
            tasks: read_tasks(&connection)?,
            lists: read_lists(&connection)?,
        })
    }

    pub fn work_settings(&self) -> Result<WorkSettings, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let stored: Option<String> = connection
            .query_row(
                "SELECT value FROM app_meta WHERE key='work_reminders_v1'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?;
        stored
            .map(|value| {
                serde_json::from_str(&value)
                    .map_err(|_| "工作时间设置已损坏，已恢复默认值".to_string())
            })
            .unwrap_or_else(|| Ok(default_work_settings()))
    }

    pub fn save_work_settings(&self, settings: WorkSettings) -> Result<(), String> {
        validate_work_settings(&settings)?;
        let value = serde_json::to_string(&settings).map_err(|error| error.to_string())?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        connection
            .execute(
                "INSERT INTO app_meta (key, value) VALUES ('work_reminders_v1', ?1)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                [&value],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn create_task(&self, draft: TaskDraft) -> Result<TodoTask, String> {
        let title = clean_title(&draft.title)?;
        validate_priority(draft.priority)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        connection
            .execute(
                "INSERT INTO todo (title, note, list_id, priority, due_date, due_time, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, COALESCE((SELECT MAX(sort_order) + 1 FROM todo), 0))",
                params![title, clean_optional(draft.note), draft.list_id, draft.priority, draft.due_date, draft.due_time],
            )
            .map_err(db_error)?;
        let id = connection.last_insert_rowid();
        read_task(&connection, id)?.ok_or_else(|| "新任务没有成功保存".to_string())
    }

    pub fn update_task(&self, id: i64, draft: TaskDraft) -> Result<TodoTask, String> {
        let title = clean_title(&draft.title)?;
        validate_priority(draft.priority)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let changed = connection
            .execute(
                "UPDATE todo SET title=?1, note=?2, list_id=?3, priority=?4, due_date=?5, due_time=?6, updated_at=CURRENT_TIMESTAMP WHERE id=?7",
                params![title, clean_optional(draft.note), draft.list_id, draft.priority, draft.due_date, draft.due_time, id],
            )
            .map_err(db_error)?;
        if changed == 0 {
            return Err("找不到这个任务，它可能已经被删除".to_string());
        }
        read_task(&connection, id)?.ok_or_else(|| "任务更新后无法读取".to_string())
    }

    pub fn toggle_task(&self, id: i64) -> Result<TodoTask, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let changed = connection
            .execute(
                "UPDATE todo SET status = CASE status WHEN 0 THEN 1 ELSE 0 END,
                    completed_at = CASE status WHEN 0 THEN CURRENT_TIMESTAMP ELSE NULL END,
                    updated_at=CURRENT_TIMESTAMP WHERE id=?1",
                [id],
            )
            .map_err(db_error)?;
        if changed == 0 {
            return Err("找不到这个任务，它可能已经被删除".to_string());
        }
        read_task(&connection, id)?.ok_or_else(|| "任务状态更新后无法读取".to_string())
    }

    pub fn delete_task(&self, id: i64) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let changed = connection
            .execute("DELETE FROM todo WHERE id=?1", [id])
            .map_err(db_error)?;
        if changed == 0 {
            return Err("找不到这个任务，它可能已经被删除".to_string());
        }
        Ok(())
    }

    pub fn reorder_tasks(&self, ids: Vec<i64>) -> Result<(), String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let transaction = connection.transaction().map_err(db_error)?;
        {
            let mut statement = transaction
                .prepare("UPDATE todo SET sort_order=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2")
                .map_err(db_error)?;
            for (index, id) in ids.iter().enumerate() {
                statement
                    .execute(params![index as i64, id])
                    .map_err(db_error)?;
            }
        }
        transaction.commit().map_err(db_error)
    }

    pub fn create_diary_entry(&self, entry_date: String) -> Result<DiaryEntry, String> {
        let date = NaiveDate::parse_from_str(&entry_date, "%Y-%m-%d")
            .map_err(|_| "日记日期无效".to_string())?;
        let entry_date = date.format("%Y-%m-%d").to_string();
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        connection
            .execute(
                "INSERT INTO diary_entry (entry_date) VALUES (?1)",
                [&entry_date],
            )
            .map_err(db_error)?;
        let id = connection.last_insert_rowid();
        read_diary_entry(&connection, id)?.ok_or_else(|| "新日记没有成功保存".to_string())
    }

    pub fn update_diary_entry(
        &self,
        id: i64,
        title: String,
        content_html: String,
        tag_ids: Vec<i64>,
    ) -> Result<DiaryEntry, String> {
        let title = title.trim().to_string();
        if title.chars().count() > 200 {
            return Err("日记标题不能超过 200 个字".to_string());
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let transaction = connection.transaction().map_err(db_error)?;
        let changed = transaction
            .execute(
                "UPDATE diary_entry SET title=?1, content_html=?2, updated_at=CURRENT_TIMESTAMP WHERE id=?3 AND deleted_at IS NULL",
                params![title, content_html, id],
            )
            .map_err(db_error)?;
        if changed == 0 {
            return Err("找不到这篇日记，它可能已被移入回收站".to_string());
        }
        transaction
            .execute("DELETE FROM diary_entry_tag WHERE entry_id=?1", [id])
            .map_err(db_error)?;
        let unique_tag_ids = tag_ids
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        for tag_id in unique_tag_ids {
            let exists = transaction
                .query_row("SELECT 1 FROM diary_tag WHERE id=?1", [tag_id], |_| Ok(()))
                .optional()
                .map_err(db_error)?
                .is_some();
            if !exists {
                return Err("选择的标签不存在".to_string());
            }
            transaction
                .execute(
                    "INSERT INTO diary_entry_tag (entry_id, tag_id) VALUES (?1, ?2)",
                    params![id, tag_id],
                )
                .map_err(db_error)?;
        }
        transaction.commit().map_err(db_error)?;
        read_diary_entry(&connection, id)?.ok_or_else(|| "日记更新后无法读取".to_string())
    }

    pub fn diary_entries(&self, include_deleted: bool) -> Result<Vec<DiaryEntry>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let ids = {
            let sql = if include_deleted {
                "SELECT id FROM diary_entry ORDER BY entry_date DESC, created_at DESC, id DESC"
            } else {
                "SELECT id FROM diary_entry WHERE deleted_at IS NULL ORDER BY entry_date DESC, created_at DESC, id DESC"
            };
            let mut statement = connection.prepare(sql).map_err(db_error)?;
            let rows = statement
                .query_map([], |row| row.get::<_, i64>(0))
                .map_err(db_error)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(db_error)?
        };
        ids.into_iter()
            .map(|id| {
                read_diary_entry(&connection, id)?
                    .ok_or_else(|| "读取日记时记录已不存在".to_string())
            })
            .collect()
    }

    pub fn create_diary_tag(&self, name: String) -> Result<DiaryTag, String> {
        let name = clean_diary_tag_name(&name)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        connection
            .execute("INSERT INTO diary_tag (name) VALUES (?1)", [&name])
            .map_err(map_diary_tag_insert_error)?;
        read_diary_tag(&connection, connection.last_insert_rowid())?
            .ok_or_else(|| "新标签没有成功保存".to_string())
    }

    pub fn diary_tags(&self) -> Result<Vec<DiaryTag>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let mut statement = connection
            .prepare("SELECT * FROM diary_tag ORDER BY created_at ASC, id ASC")
            .map_err(db_error)?;
        let rows = statement
            .query_map([], diary_tag_from_row)
            .map_err(db_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db_error)
    }

    pub fn diary_data(&self) -> Result<DiaryData, String> {
        Ok(DiaryData {
            entries: self.diary_entries(true)?,
            tags: self.diary_tags()?,
        })
    }

    pub fn rename_diary_tag(&self, id: i64, name: String) -> Result<DiaryTag, String> {
        let name = clean_diary_tag_name(&name)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let changed = connection
            .execute(
                "UPDATE diary_tag SET name=?1 WHERE id=?2",
                params![name, id],
            )
            .map_err(map_diary_tag_insert_error)?;
        if changed == 0 {
            return Err("找不到这个日记标签".to_string());
        }
        read_diary_tag(&connection, id)?.ok_or_else(|| "标签更新后无法读取".to_string())
    }

    pub fn delete_diary_tag(&self, id: i64) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let changed = connection
            .execute("DELETE FROM diary_tag WHERE id=?1", [id])
            .map_err(db_error)?;
        if changed == 0 {
            return Err("找不到这个日记标签".to_string());
        }
        Ok(())
    }

    pub fn trash_diary_entry(&self, id: i64) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let changed = connection
            .execute(
                "UPDATE diary_entry SET deleted_at=CURRENT_TIMESTAMP, updated_at=CURRENT_TIMESTAMP WHERE id=?1 AND deleted_at IS NULL",
                [id],
            )
            .map_err(db_error)?;
        if changed == 0 {
            return Err("找不到这篇日记".to_string());
        }
        Ok(())
    }

    pub fn restore_diary_entry(&self, id: i64) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let changed = connection
            .execute(
                "UPDATE diary_entry SET deleted_at=NULL, updated_at=CURRENT_TIMESTAMP WHERE id=?1 AND deleted_at IS NOT NULL",
                [id],
            )
            .map_err(db_error)?;
        if changed == 0 {
            return Err("回收站中找不到这篇日记".to_string());
        }
        Ok(())
    }

    pub fn prune_diary_assets(
        &self,
        entry_id: i64,
        keep_ids: Vec<String>,
    ) -> Result<Vec<DiaryAsset>, String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let transaction = connection.transaction().map_err(db_error)?;
        for id in &keep_ids {
            transaction
                .execute(
                    "UPDATE diary_asset SET pending_delete=0 WHERE id=?1 AND entry_id=?2",
                    params![id, entry_id],
                )
                .map_err(db_error)?;
        }
        let assets = read_diary_assets_for_entry(&transaction, entry_id)?;
        let removed = assets
            .into_iter()
            .filter(|asset| !keep_ids.contains(&asset.id))
            .collect::<Vec<_>>();
        for asset in &removed {
            transaction
                .execute(
                    "UPDATE diary_asset SET pending_delete=1 WHERE id=?1 AND entry_id=?2",
                    params![asset.id, entry_id],
                )
                .map_err(db_error)?;
        }
        transaction.commit().map_err(db_error)?;
        Ok(removed)
    }

    pub fn register_diary_asset(
        &self,
        id: String,
        entry_id: i64,
        relative_path: String,
        mime_type: String,
    ) -> Result<DiaryAsset, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        connection
            .execute(
                "INSERT INTO diary_asset (id, entry_id, relative_path, mime_type) VALUES (?1, ?2, ?3, ?4)",
                params![id, entry_id, relative_path, mime_type],
            )
            .map_err(db_error)?;
        read_diary_asset(&connection, &id)?.ok_or_else(|| "图片资源保存后无法读取".to_string())
    }

    pub fn diary_asset(&self, id: &str) -> Result<Option<DiaryAsset>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        read_diary_asset(&connection, id)
    }

    pub fn pending_diary_assets(&self) -> Result<Vec<DiaryAsset>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT * FROM diary_asset WHERE pending_delete=1 ORDER BY created_at ASC, id ASC",
            )
            .map_err(db_error)?;
        let rows = statement
            .query_map([], diary_asset_from_row)
            .map_err(db_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db_error)
    }

    pub fn remove_diary_asset_record(&self, id: &str) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        connection
            .execute("DELETE FROM diary_asset WHERE id=?1", [id])
            .map_err(db_error)?;
        Ok(())
    }

    pub fn purge_diary_entry(&self, id: i64) -> Result<Vec<DiaryAsset>, String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let transaction = connection.transaction().map_err(db_error)?;
        let assets = read_diary_assets_for_entry(&transaction, id)?;
        for asset in &assets {
            transaction
                .execute(
                    "UPDATE diary_asset SET pending_delete=1 WHERE id=?1",
                    [&asset.id],
                )
                .map_err(db_error)?;
        }
        let changed = transaction
            .execute("DELETE FROM diary_entry WHERE id=?1", [id])
            .map_err(db_error)?;
        if changed == 0 {
            return Err("找不到这篇日记".to_string());
        }
        transaction.commit().map_err(db_error)?;
        Ok(assets)
    }

    pub fn create_list(&self, name: String) -> Result<TodoList, String> {
        let name = clean_list_name(&name)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        connection
            .execute(
                "INSERT INTO todo_list (name, sort_order) VALUES (?1, COALESCE((SELECT MAX(sort_order) + 1 FROM todo_list), 0))",
                [&name],
            )
            .map_err(map_list_insert_error)?;
        read_list(&connection, connection.last_insert_rowid())?
            .ok_or_else(|| "新清单没有成功保存".to_string())
    }

    pub fn rename_list(&self, id: i64, name: String) -> Result<TodoList, String> {
        let name = clean_list_name(&name)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let changed = connection
            .execute(
                "UPDATE todo_list SET name=?1 WHERE id=?2",
                params![name, id],
            )
            .map_err(map_list_insert_error)?;
        if changed == 0 {
            return Err("找不到这个清单".to_string());
        }
        read_list(&connection, id)?.ok_or_else(|| "清单更新后无法读取".to_string())
    }

    pub fn delete_list(&self, id: i64) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "数据库暂时不可用".to_string())?;
        let changed = connection
            .execute("DELETE FROM todo_list WHERE id=?1", [id])
            .map_err(db_error)?;
        if changed == 0 {
            return Err("找不到这个清单".to_string());
        }
        Ok(())
    }
}

fn default_work_settings() -> WorkSettings {
    WorkSettings {
        lunch: WorkReminderSettings {
            enabled: false,
            time: "12:00".to_string(),
            repeat: "daily".to_string(),
            date: None,
            completed: false,
        },
        off_work: WorkReminderSettings {
            enabled: false,
            time: "18:00".to_string(),
            repeat: "daily".to_string(),
            date: None,
            completed: false,
        },
    }
}

fn validate_work_settings(settings: &WorkSettings) -> Result<(), String> {
    for reminder in [&settings.lunch, &settings.off_work] {
        if !["daily", "once"].contains(&reminder.repeat.as_str()) {
            return Err("提醒重复方式无效".to_string());
        }
        if reminder.time.len() != 5
            || !reminder.time.as_bytes()[2..3].eq(b":")
            || reminder.time[..2]
                .parse::<u8>()
                .map_or(true, |hour| hour > 23)
            || reminder.time[3..]
                .parse::<u8>()
                .map_or(true, |minute| minute > 59)
        {
            return Err("提醒时间格式无效".to_string());
        }
        if reminder.repeat == "once"
            && reminder
                .date
                .as_deref()
                .map_or(true, |date| date.len() != 10)
        {
            return Err("一次性提醒需要选择日期".to_string());
        }
    }
    Ok(())
}

fn task_from_row(row: &Row<'_>) -> rusqlite::Result<TodoTask> {
    Ok(TodoTask {
        id: row.get("id")?,
        title: row.get("title")?,
        note: row.get("note")?,
        list_id: row.get("list_id")?,
        priority: row.get("priority")?,
        due_date: row.get("due_date")?,
        due_time: row.get("due_time")?,
        status: row.get("status")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        completed_at: row.get("completed_at")?,
    })
}

fn list_from_row(row: &Row<'_>) -> rusqlite::Result<TodoList> {
    Ok(TodoList {
        id: row.get("id")?,
        name: row.get("name")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
    })
}

fn diary_entry_from_row(row: &Row<'_>) -> rusqlite::Result<DiaryEntry> {
    Ok(DiaryEntry {
        id: row.get("id")?,
        entry_date: row.get("entry_date")?,
        title: row.get("title")?,
        content_html: row.get("content_html")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        deleted_at: row.get("deleted_at")?,
        tag_ids: Vec::new(),
    })
}

fn diary_tag_from_row(row: &Row<'_>) -> rusqlite::Result<DiaryTag> {
    Ok(DiaryTag {
        id: row.get("id")?,
        name: row.get("name")?,
        created_at: row.get("created_at")?,
    })
}

fn diary_asset_from_row(row: &Row<'_>) -> rusqlite::Result<DiaryAsset> {
    Ok(DiaryAsset {
        id: row.get("id")?,
        entry_id: row.get("entry_id")?,
        relative_path: row.get("relative_path")?,
        mime_type: row.get("mime_type")?,
        pending_delete: row.get::<_, i64>("pending_delete")? != 0,
    })
}

fn read_tasks(connection: &Connection) -> Result<Vec<TodoTask>, String> {
    let mut statement = connection
        .prepare("SELECT * FROM todo ORDER BY sort_order, id")
        .map_err(db_error)?;
    let rows = statement.query_map([], task_from_row).map_err(db_error)?;
    let tasks = rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db_error);
    tasks
}

fn read_lists(connection: &Connection) -> Result<Vec<TodoList>, String> {
    let mut statement = connection
        .prepare("SELECT * FROM todo_list ORDER BY sort_order, id")
        .map_err(db_error)?;
    let rows = statement.query_map([], list_from_row).map_err(db_error)?;
    let lists = rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db_error);
    lists
}

fn read_task(connection: &Connection, id: i64) -> Result<Option<TodoTask>, String> {
    connection
        .query_row("SELECT * FROM todo WHERE id=?1", [id], task_from_row)
        .optional()
        .map_err(db_error)
}

fn read_list(connection: &Connection, id: i64) -> Result<Option<TodoList>, String> {
    connection
        .query_row("SELECT * FROM todo_list WHERE id=?1", [id], list_from_row)
        .optional()
        .map_err(db_error)
}

fn read_diary_entry(connection: &Connection, id: i64) -> Result<Option<DiaryEntry>, String> {
    let mut entry = connection
        .query_row(
            "SELECT * FROM diary_entry WHERE id=?1",
            [id],
            diary_entry_from_row,
        )
        .optional()
        .map_err(db_error)?;
    if let Some(entry) = &mut entry {
        let mut statement = connection
            .prepare("SELECT tag_id FROM diary_entry_tag WHERE entry_id=?1 ORDER BY tag_id")
            .map_err(db_error)?;
        let rows = statement
            .query_map([id], |row| row.get::<_, i64>(0))
            .map_err(db_error)?;
        entry.tag_ids = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
    }
    Ok(entry)
}

fn read_diary_tag(connection: &Connection, id: i64) -> Result<Option<DiaryTag>, String> {
    connection
        .query_row(
            "SELECT * FROM diary_tag WHERE id=?1",
            [id],
            diary_tag_from_row,
        )
        .optional()
        .map_err(db_error)
}

fn read_diary_asset(connection: &Connection, id: &str) -> Result<Option<DiaryAsset>, String> {
    connection
        .query_row(
            "SELECT * FROM diary_asset WHERE id=?1",
            [id],
            diary_asset_from_row,
        )
        .optional()
        .map_err(db_error)
}

fn read_diary_assets_for_entry(
    connection: &Connection,
    entry_id: i64,
) -> Result<Vec<DiaryAsset>, String> {
    let mut statement = connection
        .prepare("SELECT * FROM diary_asset WHERE entry_id=?1 AND pending_delete=0 ORDER BY id")
        .map_err(db_error)?;
    let rows = statement
        .query_map([entry_id], diary_asset_from_row)
        .map_err(db_error)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(db_error)
}

fn clean_title(title: &str) -> Result<String, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("任务内容不能为空".to_string());
    }
    if title.chars().count() > 300 {
        return Err("任务内容不能超过 300 个字".to_string());
    }
    Ok(title.to_string())
}

fn clean_list_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("清单名称不能为空".to_string());
    }
    if name.chars().count() > 40 {
        return Err("清单名称不能超过 40 个字".to_string());
    }
    Ok(name.to_string())
}

fn clean_diary_tag_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("标签名称不能为空".to_string());
    }
    if name.chars().count() > 40 {
        return Err("标签名称不能超过 40 个字".to_string());
    }
    Ok(name.to_string())
}

fn map_diary_tag_insert_error(error: rusqlite::Error) -> String {
    match &error {
        rusqlite::Error::SqliteFailure(code, _)
            if code.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            "已有同名日记标签，请换一个名称".to_string()
        }
        _ => db_error(error),
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let text = text.trim().to_string();
        (!text.is_empty()).then_some(text)
    })
}

fn validate_priority(priority: i64) -> Result<(), String> {
    if (0..=1).contains(&priority) {
        Ok(())
    } else {
        Err("优先级值无效".to_string())
    }
}

fn map_list_insert_error(error: rusqlite::Error) -> String {
    match &error {
        rusqlite::Error::SqliteFailure(code, _)
            if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
        {
            "已有同名清单，请换一个名称".to_string()
        }
        _ => db_error(error),
    }
}

fn db_error(error: rusqlite::Error) -> String {
    format!("本地数据库操作失败：{error}")
}

#[cfg(test)]
mod tests {
    use super::{TaskDraft, TodoDb};
    use rusqlite::Connection;
    use std::sync::Mutex;

    fn memory_db() -> TodoDb {
        TodoDb {
            connection: Mutex::new(Connection::open_in_memory().unwrap()),
        }
    }

    #[test]
    fn tasks_can_be_created_completed_and_reopened() {
        let db = memory_db();
        let connection = db.connection.lock().unwrap();
        connection.execute_batch(
            "CREATE TABLE todo_list (id INTEGER PRIMARY KEY, name TEXT NOT NULL, sort_order INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT '');
             CREATE TABLE todo (id INTEGER PRIMARY KEY, title TEXT NOT NULL, note TEXT, list_id INTEGER, priority INTEGER NOT NULL DEFAULT 0, due_date TEXT, due_time TEXT, status INTEGER NOT NULL DEFAULT 0, sort_order INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL DEFAULT '', completed_at TEXT);",
        ).unwrap();
        drop(connection);

        let task = db
            .create_task(TaskDraft {
                title: "  备份数据库  ".into(),
                note: None,
                list_id: None,
                priority: 1,
                due_date: None,
                due_time: None,
            })
            .unwrap();
        assert_eq!(task.title, "备份数据库");
        assert_eq!(task.status, 0);

        let completed = db.toggle_task(task.id).unwrap();
        assert_eq!(completed.status, 1);
        assert!(completed.completed_at.is_some());

        let reopened = db.toggle_task(task.id).unwrap();
        assert_eq!(reopened.status, 0);
        assert!(reopened.completed_at.is_none());
    }

    #[test]
    fn rejects_blank_titles() {
        let db = memory_db();
        let connection = db.connection.lock().unwrap();
        connection.execute_batch(
            "CREATE TABLE todo_list (id INTEGER PRIMARY KEY, name TEXT NOT NULL, sort_order INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT '');
             CREATE TABLE todo (id INTEGER PRIMARY KEY, title TEXT NOT NULL, note TEXT, list_id INTEGER, priority INTEGER NOT NULL DEFAULT 0, due_date TEXT, due_time TEXT, status INTEGER NOT NULL DEFAULT 0, sort_order INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL DEFAULT '', completed_at TEXT);",
        ).unwrap();
        drop(connection);
        let error = db
            .create_task(TaskDraft {
                title: "  ".into(),
                note: None,
                list_id: None,
                priority: 0,
                due_date: None,
                due_time: None,
            })
            .unwrap_err();
        assert_eq!(error, "任务内容不能为空");
    }

    #[test]
    fn list_deletion_keeps_tasks_and_default_lists_do_not_reappear() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("tinytodo.sqlite3");
        let db = TodoDb::open(&path).unwrap();
        let initial = db.app_data().unwrap();
        assert_eq!(
            initial
                .lists
                .iter()
                .map(|list| list.name.as_str())
                .collect::<Vec<_>>(),
            vec!["工作", "学习", "生活"]
        );

        let task = db
            .create_task(TaskDraft {
                title: "保留这条任务".into(),
                note: None,
                list_id: Some(initial.lists[0].id),
                priority: 0,
                due_date: None,
                due_time: None,
            })
            .unwrap();
        db.delete_list(initial.lists[0].id).unwrap();
        let after_delete = db.app_data().unwrap();
        assert_eq!(after_delete.tasks[0].id, task.id);
        assert_eq!(after_delete.tasks[0].list_id, None);

        db.delete_list(initial.lists[1].id).unwrap();
        db.delete_list(initial.lists[2].id).unwrap();
        drop(db);

        let reopened = TodoDb::open(&path).unwrap();
        let persisted = reopened.app_data().unwrap();
        assert!(persisted.lists.is_empty());
        assert_eq!(persisted.tasks[0].title, "保留这条任务");
    }

    #[test]
    fn work_settings_round_trip_through_app_meta() {
        let folder = tempfile::tempdir().unwrap();
        let db = TodoDb::open(&folder.path().join("tinytodo.sqlite3")).unwrap();
        let mut settings = db.work_settings().unwrap();
        settings.off_work.enabled = true;
        settings.off_work.time = "17:30".to_string();
        settings.off_work.repeat = "once".to_string();
        settings.off_work.date = Some("2026-09-21".to_string());
        db.save_work_settings(settings.clone()).unwrap();
        assert_eq!(db.work_settings().unwrap().off_work.time, "17:30");
        assert_eq!(
            db.work_settings().unwrap().off_work.date,
            settings.off_work.date
        );
    }

    #[test]
    fn diary_entries_support_multiple_entries_tags_and_trash_restore() {
        let folder = tempfile::tempdir().unwrap();
        let db = TodoDb::open(&folder.path().join("tinytodo.sqlite3")).unwrap();
        let tag = db.create_diary_tag("生活记录".into()).unwrap();
        let first = db.create_diary_entry("2026-10-20".into()).unwrap();
        db.update_diary_entry(
            first.id,
            "今天".into(),
            "<p>河边散步</p>".into(),
            vec![tag.id],
        )
        .unwrap();
        let second = db.create_diary_entry("2026-10-20".into()).unwrap();
        let active = db.diary_entries(false).unwrap();
        assert_eq!(active.len(), 2);
        assert_eq!(active[0].id, second.id);
        assert_eq!(active[1].id, first.id);
        assert_eq!(active[1].title, "今天");
        assert_eq!(active[1].tag_ids, vec![tag.id]);

        db.trash_diary_entry(first.id).unwrap();
        assert_eq!(db.diary_entries(false).unwrap().len(), 1);
        assert_eq!(db.diary_entries(true).unwrap().len(), 2);
        db.restore_diary_entry(first.id).unwrap();
        assert_eq!(db.diary_entries(false).unwrap().len(), 2);
    }

    #[test]
    fn diary_tags_are_unique_and_deleting_a_tag_keeps_entry_content() {
        let folder = tempfile::tempdir().unwrap();
        let db = TodoDb::open(&folder.path().join("tinytodo.sqlite3")).unwrap();
        let tag = db.create_diary_tag("工作".into()).unwrap();
        assert!(db.create_diary_tag(" 工作 ".into()).is_err());
        let entry = db.create_diary_entry("2026-10-20".into()).unwrap();
        db.update_diary_entry(
            entry.id,
            String::new(),
            "<p>周会记录</p>".into(),
            vec![tag.id],
        )
        .unwrap();
        db.delete_diary_tag(tag.id).unwrap();
        let stored = db.diary_entries(false).unwrap().remove(0);
        assert_eq!(stored.content_html, "<p>周会记录</p>");
        assert!(stored.title.is_empty());
        assert!(stored.tag_ids.is_empty());
    }

    #[test]
    fn restoring_a_reference_cancels_pending_asset_deletion() {
        let folder = tempfile::tempdir().unwrap();
        let db = TodoDb::open(&folder.path().join("tinytodo.sqlite3")).unwrap();
        let entry = db.create_diary_entry("2026-10-20".into()).unwrap();
        let asset_id = "01234567-89ab-cdef-0123-456789abcdef".to_string();
        db.register_diary_asset(
            asset_id.clone(),
            entry.id,
            format!("diary-assets/{asset_id}.png"),
            "image/png".into(),
        )
        .unwrap();

        db.prune_diary_assets(entry.id, vec![]).unwrap();
        assert_eq!(db.pending_diary_assets().unwrap().len(), 1);

        db.prune_diary_assets(entry.id, vec![asset_id.clone()])
            .unwrap();
        assert!(db.pending_diary_assets().unwrap().is_empty());
        assert_eq!(
            db.diary_asset(&asset_id).unwrap().unwrap().pending_delete,
            false
        );
    }

    #[test]
    fn diary_date_is_validated_and_purge_does_not_touch_tasks() {
        let folder = tempfile::tempdir().unwrap();
        let db = TodoDb::open(&folder.path().join("tinytodo.sqlite3")).unwrap();
        assert!(db.create_diary_entry("2026-02-30".into()).is_err());
        let task = db
            .create_task(TaskDraft {
                title: "保留的待办".into(),
                note: None,
                list_id: None,
                priority: 0,
                due_date: None,
                due_time: None,
            })
            .unwrap();
        let entry = db.create_diary_entry("2026-10-20".into()).unwrap();
        db.purge_diary_entry(entry.id).unwrap();
        assert!(db.diary_entries(true).unwrap().is_empty());
        assert_eq!(db.app_data().unwrap().tasks[0].id, task.id);
    }
}
