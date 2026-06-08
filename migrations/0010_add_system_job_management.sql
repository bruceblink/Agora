-- Keystone-compatible scheduled job management.
-- Reuses Agora's scheduled_tasks table so /system/jobs and the existing scheduler share one source of truth.

ALTER TABLE scheduled_tasks
    ADD COLUMN IF NOT EXISTS job_group VARCHAR(64) NOT NULL DEFAULT 'DEFAULT',
    ADD COLUMN IF NOT EXISTS invoke_target VARCHAR(255) NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS concurrent SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS status SMALLINT NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS remark VARCHAR(500) NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE scheduled_tasks
SET job_group = COALESCE(NULLIF(params ->> 'jobGroup', ''), NULLIF(job_group, ''), 'DEFAULT'),
    invoke_target = COALESCE(
        NULLIF(params ->> 'invokeTarget', ''),
        CASE
            WHEN NULLIF(params ->> 'cmd', '') IS NULL THEN invoke_target
            ELSE 'timerTask.' || (params ->> 'cmd') || '()'
        END,
        ''
    ),
    status = CASE WHEN is_enabled THEN 1 ELSE 0 END,
    concurrent = COALESCE(concurrent, 0),
    remark = COALESCE(remark, '')
WHERE deleted = FALSE;

DO
$$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'ck_scheduled_tasks_job_status'
    ) THEN
        ALTER TABLE scheduled_tasks
            ADD CONSTRAINT ck_scheduled_tasks_job_status CHECK (status IN (0, 1));
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'ck_scheduled_tasks_concurrent'
    ) THEN
        ALTER TABLE scheduled_tasks
            ADD CONSTRAINT ck_scheduled_tasks_concurrent CHECK (concurrent IN (0, 1));
    END IF;
END
$$;

CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_job_group ON scheduled_tasks (job_group);
CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_job_status ON scheduled_tasks (status);
CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_deleted ON scheduled_tasks (deleted);

INSERT INTO sys_menu (
    menu_id, menu_name, menu_type, router_name, parent_id, path, is_button,
    permission, meta_info, status, remark, created_at, deleted
)
SELECT seed.menu_id, seed.menu_name, seed.menu_type, seed.router_name, seed.parent_id,
       seed.path, seed.is_button, seed.permission, seed.meta_info::jsonb,
       seed.status, seed.remark, now(), FALSE
FROM (VALUES
    (67, '定时任务', 1, 'SystemJob', 2, '/system/job/index', FALSE, 'system:job:list', '{"title":"定时任务","icon":"ep:timer","showParent":true}', 1, '定时任务菜单'),
    (72, '任务查询', 0, ' ', 67, '', TRUE, 'system:job:query', '{"title":"任务查询"}', 1, ''),
    (73, '任务新增', 0, ' ', 67, '', TRUE, 'system:job:add', '{"title":"任务新增"}', 1, ''),
    (74, '任务修改', 0, ' ', 67, '', TRUE, 'system:job:edit', '{"title":"任务修改"}', 1, ''),
    (75, '任务删除', 0, ' ', 67, '', TRUE, 'system:job:remove', '{"title":"任务删除"}', 1, ''),
    (76, '任务状态修改', 0, ' ', 67, '', TRUE, 'system:job:changeStatus', '{"title":"任务状态修改"}', 1, ''),
    (77, '任务执行', 0, ' ', 67, '', TRUE, 'system:job:run', '{"title":"任务执行"}', 1, '')
) AS seed(menu_id, menu_name, menu_type, router_name, parent_id, path, is_button, permission, meta_info, status, remark)
ON CONFLICT (menu_id) DO UPDATE
SET menu_name = EXCLUDED.menu_name,
    menu_type = EXCLUDED.menu_type,
    router_name = EXCLUDED.router_name,
    parent_id = EXCLUDED.parent_id,
    path = EXCLUDED.path,
    is_button = EXCLUDED.is_button,
    permission = EXCLUDED.permission,
    meta_info = EXCLUDED.meta_info,
    status = EXCLUDED.status,
    remark = EXCLUDED.remark,
    deleted = FALSE;

SELECT setval(
    pg_get_serial_sequence('sys_menu', 'menu_id'),
    GREATEST((SELECT COALESCE(MAX(menu_id), 1) FROM sys_menu), 1),
    TRUE
);

INSERT INTO permissions (name, description)
SELECT DISTINCT permission, menu_name
FROM sys_menu
WHERE permission LIKE 'system:job:%'
  AND deleted = FALSE
ON CONFLICT (name) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
         JOIN permissions p ON p.name IN (
             SELECT permission
             FROM sys_menu
             WHERE permission LIKE 'system:job:%'
               AND deleted = FALSE
         )
WHERE r.name IN ('admin', 'common')
ON CONFLICT DO NOTHING;

INSERT INTO sys_role_menu (role_id, menu_id)
SELECT r.id, m.menu_id
FROM roles r
         JOIN sys_menu m ON m.menu_id IN (67, 72, 73, 74, 75, 76, 77)
WHERE r.name = 'common'
  AND m.deleted = FALSE
ON CONFLICT DO NOTHING;
