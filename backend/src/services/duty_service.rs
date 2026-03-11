use crate::models::duty::{DutyRotation, DutySchedule};
use crate::services::AlertService;
use chrono::{Duration as ChronoDuration, Local, NaiveDate};
use serde_json;
use sqlx::{MySqlPool, query_as};
use std::collections::HashSet;
use std::sync::Arc;

pub struct DutyService {
    pool: MySqlPool,
    alert_service: Arc<AlertService>,
}

impl DutyService {
    pub fn new(pool: MySqlPool, alert_service: Arc<AlertService>) -> Self {
        Self { pool, alert_service }
    }

    pub async fn start_monitor_loop(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // Check every minute
        loop {
            interval.tick().await;

            // 1. 自动转动过期轮换
            if let Err(e) = self.check_and_rotate_expired_rotations().await {
                tracing::error!("Error in duty rotation rotation: {}", e);
            }

            // 2. 检查并发送自动通知
            if let Err(e) = self.check_and_notify_upcoming_duties().await {
                tracing::error!("Error in duty monitor loop: {}", e);
            }
        }
    }

    /// 自动转动到期的值班轮换（修改日期、人员顺序并生成新排班）
    pub async fn check_and_rotate_expired_rotations(
        &self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = Local::now().date_naive();
        let rotations: Vec<DutyRotation> = query_as("SELECT * FROM duty_rotation")
            .fetch_all(&self.pool)
            .await?;

        for rotation in rotations {
            if now > rotation.end_date {
                tracing::info!("Rotating expired duty rotation: {}", rotation.name);

                let mut members: Vec<i32> =
                    serde_json::from_str(&rotation.personnel_ids).unwrap_or_default();
                if members.is_empty() {
                    continue;
                }

                // 旋转数组：将当前第一个人员移到末尾，使第二个人员排在第一位
                members.rotate_left(1);
                let new_personnel_ids_json = serde_json::to_string(&members).unwrap();

                let new_start = rotation.end_date + ChronoDuration::days(1);
                // 修改逻辑：只排当前这一个人的周期（即一个 period_days），不再排满所有人的全周期
                let total_days = rotation.period_days.unwrap_or(7);
                let new_end = new_start + ChronoDuration::days((total_days - 1) as i64);

                let mut tx = self.pool.begin().await?;

                // 1. 更新轮换配置
                sqlx::query("UPDATE duty_rotation SET personnel_ids = ?, start_date = ?, end_date = ?, last_notified_date = NULL WHERE id = ?")
                    .bind(&new_personnel_ids_json)
                    .bind(new_start)
                    .bind(new_end)
                    .bind(rotation.id)
                    .execute(&mut *tx)
                    .await?;

                // 2. 清理旧排班并生成新周期的排班
                sqlx::query("DELETE FROM duty_schedule WHERE duty_platform = ? AND country = ?")
                    .bind(&rotation.name)
                    .bind(rotation.country.as_deref().unwrap_or("all"))
                    .execute(&mut *tx)
                    .await?;

                let mut current_date = new_start;
                let mut personnel_idx = 0;
                while current_date <= new_end {
                    let pid = members[personnel_idx % members.len()];
                    for _ in 0..rotation.period_days.unwrap_or(7) {
                        if current_date > new_end {
                            break;
                        }
                        sqlx::query("INSERT INTO duty_schedule (duty_date, country, duty_platform, shift_type, personnel_id) VALUES (?, ?, ?, 'All Day', ?)")
                            .bind(current_date)
                            .bind(rotation.country.as_deref().unwrap_or("all"))
                            .bind(&rotation.name)
                            .bind(pid)
                            .execute(&mut *tx)
                            .await?;
                        current_date += ChronoDuration::days(1);
                    }
                    personnel_idx += 1;
                }

                tx.commit().await?;
                tracing::info!(
                    "Successfully rotated platform {} to next cycle: {} ~ {}",
                    rotation.name,
                    new_start,
                    new_end
                );
            }
        }
        Ok(())
    }

    /// 获取手动通知的消息内容，根据是否达到通知时间返回当前或下一个周期
    pub async fn get_manual_notification_message(
        &self,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let now = Local::now();
        let rotations: Vec<DutyRotation> = query_as("SELECT * FROM duty_rotation")
            .fetch_all(&self.pool)
            .await?;

        let mut all_schedules = Vec::new();

        for rotation in rotations {
            let end_date_time = rotation.end_date.and_hms_opt(23, 59, 59).unwrap();
            let notify_time = end_date_time
                - ChronoDuration::hours(rotation.notify_advance_hours.unwrap_or(7) as i64);

            if now.naive_local() >= notify_time {
                // 已达到通知时间，发送下个周期
                let next_schedules = self.get_schedules_for_next_cycle(&rotation).await?;
                all_schedules.extend(next_schedules);
            } else {
                // 未达到通知时间，发送当前周期
                let current_schedules: Vec<DutySchedule> = query_as(
                    r#"
                    SELECT s.*, p.name as personnel_name, p.email as personnel_email, p.duty_platform 
                    FROM duty_schedule s
                    JOIN duty_personnel p ON s.personnel_id = p.id
                    WHERE s.duty_date >= ? AND s.duty_date <= ? AND s.duty_platform = ?
                    ORDER BY s.duty_date ASC
                    "#,
                )
                .bind(rotation.start_date)
                .bind(rotation.end_date)
                .bind(&rotation.name)
                .fetch_all(&self.pool)
                .await?;

                if !current_schedules.is_empty() {
                    all_schedules.extend(current_schedules);
                } else {
                    // 如果当前排班为空（可能已被意外删除），尝试预测当前周期
                    // 逻辑略，此处简单跳过或同样调用预测逻辑
                }
            }
        }

        if all_schedules.is_empty() {
            return Ok("📅 近期暂无值班信息".to_string());
        }

        Ok(self.build_duty_message(&all_schedules, false))
    }

    /// 获取手动通知的所有 Mention 人员邮箱
    pub async fn get_manual_notification_mentions(
        &self,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let now = Local::now();
        let rotations: Vec<DutyRotation> = query_as("SELECT * FROM duty_rotation")
            .fetch_all(&self.pool)
            .await?;

        let mut all_mentions = HashSet::new();

        for rotation in rotations {
            let end_date_time = rotation.end_date.and_hms_opt(23, 59, 59).unwrap();
            let notify_time = end_date_time
                - ChronoDuration::hours(rotation.notify_advance_hours.unwrap_or(7) as i64);

            let schedules = if now.naive_local() >= notify_time {
                self.get_schedules_for_next_cycle(&rotation).await?
            } else {
                query_as(
                    "SELECT s.*, p.name as personnel_name, p.email as personnel_email, p.duty_platform 
                     FROM duty_schedule s JOIN duty_personnel p ON s.personnel_id = p.id 
                     WHERE s.duty_date >= ? AND s.duty_date <= ? AND s.duty_platform = ?",
                )
                .bind(rotation.start_date)
                .bind(rotation.end_date)
                .bind(&rotation.name)
                .fetch_all(&self.pool)
                .await?
            };

            for s in schedules {
                if let Some(email) = s.personnel_email.filter(|e| !e.is_empty()) {
                    all_mentions.insert(email);
                }
            }
        }

        Ok(all_mentions.into_iter().collect())
    }

    async fn check_and_notify_upcoming_duties(&self) -> Result<(), Box<dyn std::error::Error>> {
        let now = Local::now();

        // 1. 获取所有开启自动通知的轮换配置
        let rotations: Vec<DutyRotation> =
            query_as("SELECT * FROM duty_rotation WHERE auto_notify = 1")
                .fetch_all(&self.pool)
                .await?;

        // 2. 筛选出需要通知的轮换，并按 Bot IDs 分组
        let mut groups: std::collections::HashMap<String, Vec<DutyRotation>> =
            std::collections::HashMap::new();

        for rotation in rotations {
            if let Some(bot_ids_str) = &rotation.bot_ids {
                let bot_ids_trimmed = bot_ids_str.trim();
                if bot_ids_trimmed.is_empty() {
                    continue;
                }

                let end_date_time = rotation.end_date.and_hms_opt(23, 59, 59).unwrap();
                let notify_time = end_date_time
                    - ChronoDuration::hours(rotation.notify_advance_hours.unwrap_or(7) as i64);

                // 只有当：当前时间到了通知时间 AND 周期尚未结束 (截止日期 >= 今天) AND (从未通知过 OR 最后通知的日期不等于当前周期结束日期)
                let needs_notify = now.naive_local() >= notify_time
                    && rotation.end_date >= now.date_naive()
                    && (rotation.last_notified_date.is_none()
                        || rotation.last_notified_date.unwrap() != rotation.end_date);

                if needs_notify {
                    // 归一化 Bot IDs 以便作为分组 Key
                    let mut ids: Vec<String> = bot_ids_trimmed
                        .split([',', '\n', ' ', '，'])
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect();
                    ids.sort();
                    let key = ids.join(",");
                    groups.entry(key).or_default().push(rotation);
                }
            }
        }

        // 3. 对每个分组发送聚合后的通知
        for (bot_ids_key, rotations_in_group) in groups {
            let mut all_schedules = Vec::new();

            for rotation in &rotations_in_group {
                let schedules = self.get_schedules_for_next_cycle(rotation).await?;
                all_schedules.extend(schedules);
            }

            if all_schedules.is_empty() {
                continue;
            }

            let message = self.build_duty_message(&all_schedules, true);
            let mentions = self.collect_mentions(&all_schedules);

            tracing::info!(
                "Sending aggregated duty notification for platforms: {:?} to bot_ids: {}",
                rotations_in_group
                    .iter()
                    .map(|r| &r.name)
                    .collect::<Vec<_>>(),
                bot_ids_key
            );

            // 4. 先更新数据库中的 last_notified_date 标识，防止发送成功但更新失败导致的重复发送
            for rotation in &rotations_in_group {
                sqlx::query("UPDATE duty_rotation SET last_notified_date = ? WHERE id = ?")
                    .bind(rotation.end_date)
                    .bind(rotation.id)
                    .execute(&self.pool)
                    .await?;
            }

            // 5. 进行通知发送
            let bot_ids: Vec<&str> = bot_ids_key.split(',').collect();
            for bot_id in bot_ids {
                let _ = self
                    .alert_service
                    .send_notification(bot_id, &message, mentions.clone())
                    .await;
            }
        }
        Ok(())
    }

    async fn get_schedules_for_next_cycle(
        &self,
        rotation: &DutyRotation,
    ) -> Result<Vec<DutySchedule>, Box<dyn std::error::Error>> {
        let start = rotation.end_date + ChronoDuration::days(1);
        let end = start + ChronoDuration::days(6);

        let mut schedules: Vec<DutySchedule> = query_as(
            r#"
            SELECT s.*, p.name as personnel_name, p.email as personnel_email, p.duty_platform 
            FROM duty_schedule s
            JOIN duty_personnel p ON s.personnel_id = p.id
            WHERE s.duty_date >= ? AND s.duty_date <= ? AND s.duty_platform = ?
            ORDER BY s.duty_date ASC
            "#,
        )
        .bind(start)
        .bind(end)
        .bind(&rotation.name)
        .fetch_all(&self.pool)
        .await?;

        // 如果未生成正式排班，则尝试预测
        if schedules.is_empty() {
            tracing::warn!(
                "No schedules found for next cycle of platform: {} ({} to {}). Predicting next personnel.",
                rotation.name,
                start,
                end
            );

            let members: Vec<i32> =
                serde_json::from_str(&rotation.personnel_ids).unwrap_or_default();
            if members.is_empty() {
                return Ok(vec![]);
            }

            let current_duty: Option<DutySchedule> = query_as(
                "SELECT s.*, p.name as personnel_name, p.email as personnel_email, p.duty_platform 
                 FROM duty_schedule s 
                 JOIN duty_personnel p ON s.personnel_id = p.id 
                 WHERE s.duty_date = ? AND s.duty_platform = ?",
            )
            .bind(rotation.end_date)
            .bind(&rotation.name)
            .fetch_optional(&self.pool)
            .await?;

            let next_pid = if let Some(current) = current_duty {
                let current_idx = members.iter().position(|&id| id == current.personnel_id);
                if let Some(idx) = current_idx {
                    members[(idx + 1) % members.len()]
                } else {
                    members[0]
                }
            } else {
                members[0]
            };

            let next_person: Option<crate::models::duty::DutyPersonnel> =
                query_as("SELECT * FROM duty_personnel WHERE id = ?")
                    .bind(next_pid)
                    .fetch_optional(&self.pool)
                    .await?;

            if let Some(p) = next_person {
                // 构造 7 天的预测数据，以便 build_duty_message 能将其聚合为范围展示
                for i in 0..7 {
                    let date = start + ChronoDuration::days(i as i64);
                    schedules.push(DutySchedule {
                        id: 0,
                        duty_date: date,
                        country: rotation
                            .country
                            .clone()
                            .unwrap_or_else(|| "all".to_string()),
                        duty_platform: Some(rotation.name.clone()),
                        shift_type: "All Day".to_string(),
                        personnel_id: p.id,
                        personnel_name: Some(p.name.clone()),
                        personnel_email: if p.email.is_empty() {
                            None
                        } else {
                            Some(p.email.clone())
                        },
                        created_at: None,
                        updated_at: None,
                    });
                }
            }
        }
        Ok(schedules)
    }

    fn build_duty_message(&self, data: &[DutySchedule], is_auto: bool) -> String {
        use std::collections::BTreeMap;
        let mut grouped: BTreeMap<NaiveDate, Vec<String>> = BTreeMap::new();

        for item in data {
            if let Some(name) = &item.personnel_name {
                let platform = item.duty_platform.as_deref().unwrap_or("通用");
                let person = format!("{}({})", name, platform);
                grouped.entry(item.duty_date).or_default().push(person);
            }
        }

        // 排序每个日期的内容，确保聚合结果一致
        for persons in grouped.values_mut() {
            persons.sort();
        }

        let dates: Vec<NaiveDate> = grouped.keys().cloned().collect();
        let mut merged: Vec<String> = Vec::new();

        if !dates.is_empty() {
            let mut start = dates[0];
            let mut content = grouped[&start].join(", ");

            for i in 1..=dates.len() {
                let current = if i < dates.len() { Some(dates[i]) } else { None };
                let prev = dates[i - 1];

                let is_consecutive = current.is_some_and(|c| (c - prev).num_days() == 1);
                let is_same_content = current.is_some_and(|c| grouped[&c].join(", ") == content);

                if is_consecutive && is_same_content {
                    continue;
                } else {
                    let range_end = prev;
                    let s_disp = start.format("%m-%d").to_string();
                    let e_disp = range_end.format("%m-%d").to_string();

                    if start == range_end {
                        merged.push(format!("• {}: {}", s_disp, content));
                    } else {
                        merged.push(format!("• {} ~ {}: {}", s_disp, e_disp, content));
                    }

                    if let Some(c) = current {
                        start = c;
                        content = grouped[&c].join(", ");
                    }
                }
            }
        }

        let header = if is_auto { "📅 值班自动提醒：" } else { "📅 值班安排：" };

        let mut message = format!("{}\n", header);
        message.push_str(&merged.join("\n"));

        message
    }

    fn collect_mentions(&self, data: &[DutySchedule]) -> Vec<String> {
        let mut mentions = HashSet::new();
        for item in data {
            if let Some(email) = &item.personnel_email {
                mentions.insert(email.clone());
            }
        }
        mentions.into_iter().collect()
    }
}
