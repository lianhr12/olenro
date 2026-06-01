#![allow(non_snake_case)]

use tauri::AppHandle;

fn merge_settings_for_save(
    mut incoming: crate::settings::AppSettings,
    existing: &crate::settings::AppSettings,
) -> crate::settings::AppSettings {
    match (&mut incoming.git_sync, &existing.git_sync) {
        // incoming 没有 git_sync → 保留现有
        (None, _) => {
            incoming.git_sync = existing.git_sync.clone();
        }
        // incoming 有 git_sync 但 token 为空，且现有有 token → 填回现有 token
        // （get_settings_for_frontend 总是清空 token，所以通过 save_settings
        //   传入的空 token 意味着"保持现有"而非"用户主动清空"）
        (Some(incoming_sync), Some(existing_sync))
            if incoming_sync.token.is_empty() && !existing_sync.token.is_empty() =>
        {
            incoming_sync.token = existing_sync.token.clone();
        }
        _ => {}
    }
    if incoming.local_migrations.is_none() {
        incoming.local_migrations = existing.local_migrations.clone();
    } else if let (Some(incoming_migrations), Some(existing_migrations)) =
        (&mut incoming.local_migrations, &existing.local_migrations)
    {
        if incoming_migrations
            .codex_third_party_history_provider_bucket_v1
            .is_none()
        {
            incoming_migrations.codex_third_party_history_provider_bucket_v1 = existing_migrations
                .codex_third_party_history_provider_bucket_v1
                .clone();
        }
        if incoming_migrations.codex_provider_template_v1.is_none() {
            incoming_migrations.codex_provider_template_v1 =
                existing_migrations.codex_provider_template_v1.clone();
        }
    }
    incoming
}

/// 获取设置
#[tauri::command]
pub async fn get_settings() -> Result<crate::settings::AppSettings, String> {
    Ok(crate::settings::get_settings_for_frontend())
}

/// 保存设置
#[tauri::command]
pub async fn save_settings(settings: crate::settings::AppSettings) -> Result<bool, String> {
    let existing = crate::settings::get_settings();
    let merged = merge_settings_for_save(settings, &existing);
    crate::settings::update_settings(merged).map_err(|e| e.to_string())?;
    Ok(true)
}

/// 重启应用程序（当 app_config_dir 变更后使用）
#[tauri::command]
pub async fn restart_app(app: AppHandle) -> Result<bool, String> {
    crate::save_window_state_before_exit(&app);

    // 在后台延迟重启，让函数有时间返回响应
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        app.restart();
    });
    Ok(true)
}

/// 获取 app_config_dir 覆盖配置 (从 Store)
#[tauri::command]
pub async fn get_app_config_dir_override(app: AppHandle) -> Result<Option<String>, String> {
    Ok(crate::app_store::refresh_app_config_dir_override(&app)
        .map(|p| p.to_string_lossy().to_string()))
}

/// 设置 app_config_dir 覆盖配置 (到 Store)
#[tauri::command]
pub async fn set_app_config_dir_override(
    app: AppHandle,
    path: Option<String>,
) -> Result<bool, String> {
    crate::app_store::set_app_config_dir_to_store(&app, path.as_deref())?;
    Ok(true)
}

/// 设置开机自启
#[tauri::command]
pub async fn set_auto_launch(enabled: bool) -> Result<bool, String> {
    if enabled {
        crate::auto_launch::enable_auto_launch().map_err(|e| format!("启用开机自启失败: {e}"))?;
    } else {
        crate::auto_launch::disable_auto_launch().map_err(|e| format!("禁用开机自启失败: {e}"))?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::merge_settings_for_save;
    use crate::settings::{
        AppSettings, CodexProviderTemplateMigration, CodexThirdPartyHistoryProviderBucketMigration,
        GitSyncSettings, LocalMigrations,
    };

    #[test]
    fn save_settings_should_preserve_existing_git_sync_when_payload_omits_it() {
        let existing = AppSettings {
            git_sync: Some(GitSyncSettings {
                repo_url: "https://github.com/owner/repo.git".to_string(),
                token: "secret".to_string(),
                ..GitSyncSettings::default()
            }),
            ..AppSettings::default()
        };

        let incoming = AppSettings::default();
        let merged = merge_settings_for_save(incoming, &existing);

        assert!(merged.git_sync.is_some());
        assert_eq!(
            merged.git_sync.as_ref().map(|v| v.repo_url.as_str()),
            Some("https://github.com/owner/repo.git")
        );
    }

    #[test]
    fn save_settings_should_keep_incoming_git_sync_when_present() {
        let existing = AppSettings {
            git_sync: Some(GitSyncSettings {
                repo_url: "https://github.com/owner/old.git".to_string(),
                token: "old-token".to_string(),
                ..GitSyncSettings::default()
            }),
            ..AppSettings::default()
        };

        let incoming = AppSettings {
            git_sync: Some(GitSyncSettings {
                repo_url: "https://github.com/owner/new.git".to_string(),
                token: "new-token".to_string(),
                ..GitSyncSettings::default()
            }),
            ..AppSettings::default()
        };

        let merged = merge_settings_for_save(incoming, &existing);

        assert_eq!(
            merged.git_sync.as_ref().map(|v| v.repo_url.as_str()),
            Some("https://github.com/owner/new.git")
        );
    }

    /// Regression test: frontend always receives empty token from
    /// get_settings_for_frontend(). If a component accidentally spreads
    /// the full settings object into save_settings, the empty token
    /// must NOT overwrite the existing one.
    #[test]
    fn save_settings_should_preserve_token_when_incoming_has_empty_token() {
        let existing = AppSettings {
            git_sync: Some(GitSyncSettings {
                repo_url: "https://github.com/owner/repo.git".to_string(),
                token: "secret".to_string(),
                ..GitSyncSettings::default()
            }),
            ..AppSettings::default()
        };

        // Simulate frontend sending settings with cleared token
        let incoming = AppSettings {
            git_sync: Some(GitSyncSettings {
                repo_url: "https://github.com/owner/repo.git".to_string(),
                token: "".to_string(),
                ..GitSyncSettings::default()
            }),
            ..AppSettings::default()
        };

        let merged = merge_settings_for_save(incoming, &existing);

        assert_eq!(
            merged.git_sync.as_ref().map(|v| v.token.as_str()),
            Some("secret"),
            "empty token from frontend must not overwrite existing token"
        );
    }

    /// When both incoming and existing have no token, merge should
    /// work without panicking and keep the empty state.
    #[test]
    fn save_settings_should_handle_both_empty_tokens() {
        let existing = AppSettings {
            git_sync: Some(GitSyncSettings {
                repo_url: "https://github.com/owner/repo.git".to_string(),
                token: "".to_string(),
                ..GitSyncSettings::default()
            }),
            ..AppSettings::default()
        };

        let incoming = AppSettings {
            git_sync: Some(GitSyncSettings {
                repo_url: "https://github.com/owner/repo.git".to_string(),
                token: "".to_string(),
                ..GitSyncSettings::default()
            }),
            ..AppSettings::default()
        };

        let merged = merge_settings_for_save(incoming, &existing);

        assert_eq!(merged.git_sync.as_ref().map(|v| v.token.as_str()), Some(""));
    }

    #[test]
    fn save_settings_should_preserve_local_migrations_when_payload_omits_it() {
        let existing = AppSettings {
            local_migrations: Some(LocalMigrations {
                codex_third_party_history_provider_bucket_v1: Some(
                    CodexThirdPartyHistoryProviderBucketMigration {
                        completed_at: "2026-05-20T00:00:00Z".to_string(),
                        target_provider_id: "custom".to_string(),
                        source_provider_ids: vec!["rightcode".to_string()],
                        migrated_jsonl_files: 2,
                        migrated_state_rows: 3,
                        scanned_history_files: true,
                    },
                ),
                codex_provider_template_v1: Some(CodexProviderTemplateMigration {
                    completed_at: "2026-05-20T00:01:00Z".to_string(),
                    migrated_provider_ids: vec!["legacy".to_string()],
                }),
            }),
            ..AppSettings::default()
        };

        let incoming = AppSettings::default();
        let merged = merge_settings_for_save(incoming, &existing);

        let migration = merged
            .local_migrations
            .as_ref()
            .and_then(|migrations| {
                migrations
                    .codex_third_party_history_provider_bucket_v1
                    .as_ref()
            })
            .expect("local migration marker should be preserved");
        assert_eq!(migration.target_provider_id, "custom");
        assert_eq!(migration.migrated_jsonl_files, 2);
        assert_eq!(migration.migrated_state_rows, 3);

        let template_migration = merged
            .local_migrations
            .as_ref()
            .and_then(|migrations| migrations.codex_provider_template_v1.as_ref())
            .expect("template migration marker should be preserved");
        assert_eq!(
            template_migration.migrated_provider_ids,
            vec!["legacy".to_string()]
        );
    }
}

/// 获取开机自启状态
#[tauri::command]
pub async fn get_auto_launch_status() -> Result<bool, String> {
    crate::auto_launch::is_auto_launch_enabled().map_err(|e| format!("获取开机自启状态失败: {e}"))
}

/// 获取整流器配置
#[tauri::command]
pub async fn get_rectifier_config(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::proxy::types::RectifierConfig, String> {
    state.db.get_rectifier_config().map_err(|e| e.to_string())
}

/// 设置整流器配置
#[tauri::command]
pub async fn set_rectifier_config(
    state: tauri::State<'_, crate::AppState>,
    config: crate::proxy::types::RectifierConfig,
) -> Result<bool, String> {
    state
        .db
        .set_rectifier_config(&config)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 获取优化器配置
#[tauri::command]
pub async fn get_optimizer_config(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::proxy::types::OptimizerConfig, String> {
    state.db.get_optimizer_config().map_err(|e| e.to_string())
}

/// 设置优化器配置
#[tauri::command]
pub async fn set_optimizer_config(
    state: tauri::State<'_, crate::AppState>,
    config: crate::proxy::types::OptimizerConfig,
) -> Result<bool, String> {
    // Validate cache_ttl: only allow known values
    match config.cache_ttl.as_str() {
        "5m" | "1h" => {}
        other => {
            return Err(format!(
                "Invalid cache_ttl value: '{other}'. Allowed values: '5m', '1h'"
            ))
        }
    }
    state
        .db
        .set_optimizer_config(&config)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 获取 Copilot 优化器配置
#[tauri::command]
pub async fn get_copilot_optimizer_config(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::proxy::types::CopilotOptimizerConfig, String> {
    state
        .db
        .get_copilot_optimizer_config()
        .map_err(|e| e.to_string())
}

/// 设置 Copilot 优化器配置
#[tauri::command]
pub async fn set_copilot_optimizer_config(
    state: tauri::State<'_, crate::AppState>,
    config: crate::proxy::types::CopilotOptimizerConfig,
) -> Result<bool, String> {
    state
        .db
        .set_copilot_optimizer_config(&config)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 获取日志配置
#[tauri::command]
pub async fn get_log_config(
    state: tauri::State<'_, crate::AppState>,
) -> Result<crate::proxy::types::LogConfig, String> {
    state.db.get_log_config().map_err(|e| e.to_string())
}

/// 设置日志配置
#[tauri::command]
pub async fn set_log_config(
    state: tauri::State<'_, crate::AppState>,
    config: crate::proxy::types::LogConfig,
) -> Result<bool, String> {
    state
        .db
        .set_log_config(&config)
        .map_err(|e| e.to_string())?;
    log::set_max_level(config.to_level_filter());
    log::info!(
        "日志配置已更新: enabled={}, level={}",
        config.enabled,
        config.level
    );
    Ok(true)
}
