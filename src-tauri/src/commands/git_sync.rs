#![allow(non_snake_case)]

use serde_json::{json, Value};
use tauri::State;

use crate::commands::sync_support::{
    attach_warning, post_sync_warning_from_result, run_post_import_sync,
};
use crate::error::AppError;
use crate::services::git_sync as git_sync_service;
use crate::settings::{self, GitSyncSettings};
use crate::store::AppState;

fn git_not_configured_error() -> String {
    AppError::localized(
        "git.sync.not_configured",
        "未配置 Git 同步",
        "Git sync is not configured.",
    )
    .to_string()
}

fn git_sync_disabled_error() -> String {
    AppError::localized(
        "git.sync.disabled",
        "Git 同步未启用",
        "Git sync is disabled.",
    )
    .to_string()
}

fn require_enabled_git_settings() -> Result<GitSyncSettings, String> {
    let settings = settings::get_git_sync_settings().ok_or_else(git_not_configured_error)?;
    if !settings.enabled {
        return Err(git_sync_disabled_error());
    }
    Ok(settings)
}

/// 保存设置时，若前端未重新输入 token，则保留已有 token。
fn resolve_token_for_request(
    mut incoming: GitSyncSettings,
    existing: Option<GitSyncSettings>,
    preserve_empty_token: bool,
) -> GitSyncSettings {
    if let Some(existing_settings) = existing {
        if preserve_empty_token && incoming.token.is_empty() {
            incoming.token = existing_settings.token;
        }
    }
    incoming
}

fn persist_sync_error(error: &AppError, source: &str) {
    if let Some(mut settings) = settings::get_git_sync_settings() {
        settings.status.last_error = Some(error.to_string());
        settings.status.last_error_source = Some(source.to_string());
        let _ = settings::update_git_sync_status(settings.status);
    }
}

#[tauri::command]
pub async fn git_sync_save_settings(
    settings: GitSyncSettings,
    tokenTouched: Option<bool>,
) -> Result<Value, String> {
    let token_touched = tokenTouched.unwrap_or(false);
    let existing = settings::get_git_sync_settings();
    let mut sync_settings = resolve_token_for_request(settings, existing.clone(), !token_touched);

    // 保留服务端管理的状态字段
    if let Some(existing_settings) = existing {
        sync_settings.status = existing_settings.status;
    }

    sync_settings.normalize();
    sync_settings.validate().map_err(|e| e.to_string())?;
    settings::set_git_sync_settings(Some(sync_settings)).map_err(|e| e.to_string())?;
    Ok(json!({ "success": true }))
}

#[tauri::command]
pub async fn git_test_connection(
    settings: GitSyncSettings,
    tokenTouched: Option<bool>,
) -> Result<Value, String> {
    let token_touched = tokenTouched.unwrap_or(false);
    let mut resolved =
        resolve_token_for_request(settings, settings::get_git_sync_settings(), !token_touched);
    resolved.normalize();
    resolved.validate().map_err(|e| e.to_string())?;

    tauri::async_runtime::spawn_blocking(move || git_sync_service::check_connection(&resolved))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(json!({ "success": true, "message": "Git connection ok" }))
}

#[tauri::command]
pub async fn git_sync_asset_catalog() -> Result<Value, String> {
    Ok(Value::Array(git_sync_service::asset_catalog()))
}

#[tauri::command]
pub async fn git_sync_fetch_remote_info() -> Result<Value, String> {
    let settings = require_enabled_git_settings()?;
    tauri::async_runtime::spawn_blocking(move || git_sync_service::fetch_remote_info(&settings))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn git_sync_push(state: State<'_, AppState>) -> Result<Value, String> {
    run_push(state, false).await
}

#[tauri::command]
pub async fn git_sync_force_push(state: State<'_, AppState>) -> Result<Value, String> {
    run_push(state, true).await
}

async fn run_push(state: State<'_, AppState>, force: bool) -> Result<Value, String> {
    let db = state.db.clone();
    let mut settings = require_enabled_git_settings()?;

    let result = git_sync_service::run_with_sync_lock(async move {
        tauri::async_runtime::spawn_blocking(move || {
            let outcome = if force {
                git_sync_service::force_push(&db, &mut settings)
            } else {
                git_sync_service::push(&db, &mut settings)
            };
            outcome
        })
        .await
        .map_err(|e| AppError::Config(e.to_string()))?
    })
    .await;

    match result {
        Ok(value) => Ok(value),
        Err(err) => {
            persist_sync_error(&err, "manual");
            Err(err.to_string())
        }
    }
}

#[tauri::command]
pub async fn git_sync_pull(state: State<'_, AppState>) -> Result<Value, String> {
    let db = state.db.clone();
    let db_for_sync = db.clone();
    let mut settings = require_enabled_git_settings()?;

    let result = git_sync_service::run_with_sync_lock(async move {
        tauri::async_runtime::spawn_blocking(move || git_sync_service::pull(&db, &mut settings))
            .await
            .map_err(|e| AppError::Config(e.to_string()))?
    })
    .await;

    let mut value = match result {
        Ok(value) => value,
        Err(err) => {
            persist_sync_error(&err, "manual");
            return Err(err.to_string());
        }
    };

    // 拉取成功后，重新把供应商分发到 live 配置（best-effort）。
    let is_empty = value
        .get("empty")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !is_empty {
        let warning = post_sync_warning_from_result(
            tauri::async_runtime::spawn_blocking(move || run_post_import_sync(db_for_sync))
                .await
                .map_err(|e| e.to_string()),
        );
        if let Some(msg) = warning.as_ref() {
            log::warn!("[Git] post-pull sync warning: {msg}");
        }
        value = attach_warning(value, warning);
    }

    Ok(value)
}
