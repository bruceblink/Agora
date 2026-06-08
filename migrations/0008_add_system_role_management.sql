-- ============================================================================
-- Keystone-compatible system role metadata and menu grants
-- Department and post models remain out of scope; dept_id_set is kept as a
-- compatibility field only.
-- ============================================================================

ALTER TABLE roles ADD COLUMN IF NOT EXISTS role_name VARCHAR(32);
ALTER TABLE roles ADD COLUMN IF NOT EXISTS role_key VARCHAR(128);
ALTER TABLE roles ADD COLUMN IF NOT EXISTS role_sort INTEGER NOT NULL DEFAULT 0;
ALTER TABLE roles ADD COLUMN IF NOT EXISTS data_scope SMALLINT NOT NULL DEFAULT 1;
ALTER TABLE roles ADD COLUMN IF NOT EXISTS dept_id_set VARCHAR(1024) NOT NULL DEFAULT '';
ALTER TABLE roles ADD COLUMN IF NOT EXISTS status SMALLINT NOT NULL DEFAULT 1;
ALTER TABLE roles ADD COLUMN IF NOT EXISTS remark VARCHAR(512);
ALTER TABLE roles ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE roles
SET role_name = CASE name
                    WHEN 'admin' THEN '超级管理员'
                    WHEN 'editor' THEN '编辑用户'
                    WHEN 'user' THEN '普通用户'
                    ELSE COALESCE(NULLIF(role_name, ''), name)
                END,
    role_key = COALESCE(NULLIF(role_key, ''), name),
    role_sort = CASE name
                    WHEN 'admin' THEN 1
                    WHEN 'editor' THEN 2
                    WHEN 'user' THEN 4
                    ELSE COALESCE(NULLIF(role_sort, 0), id::INTEGER)
                END,
    data_scope = CASE name
                     WHEN 'admin' THEN 1
                     ELSE COALESCE(NULLIF(data_scope, 0), 2)
                 END,
    status = COALESCE(NULLIF(status, 0), 1),
    remark = COALESCE(remark, description)
WHERE role_name IS NULL
   OR role_name = ''
   OR role_key IS NULL
   OR role_key = '';

ALTER TABLE roles ALTER COLUMN role_name SET NOT NULL;
ALTER TABLE roles ALTER COLUMN role_key SET NOT NULL;

DO
$$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'ck_roles_status'
    ) THEN
        ALTER TABLE roles ADD CONSTRAINT ck_roles_status CHECK (status IN (0, 1));
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'ck_roles_data_scope'
    ) THEN
        ALTER TABLE roles ADD CONSTRAINT ck_roles_data_scope CHECK (data_scope BETWEEN 1 AND 5);
    END IF;
END
$$;

CREATE UNIQUE INDEX IF NOT EXISTS uq_roles_role_name_active
    ON roles (role_name)
    WHERE deleted = FALSE;

CREATE UNIQUE INDEX IF NOT EXISTS uq_roles_role_key_active
    ON roles (role_key)
    WHERE deleted = FALSE;

INSERT INTO roles (
    name, role_name, role_key, role_sort, data_scope, dept_id_set,
    status, description, remark, deleted, created_at
)
VALUES ('common', '普通角色', 'common', 3, 2, '', 1, '普通角色', '普通角色', FALSE, '2022-05-21 08:30:54+00'),
       ('unused', '闲置角色', 'unused', 5, 2, '', 0, '未使用的角色', '未使用的角色', FALSE, '2022-05-21 08:30:54+00')
ON CONFLICT (name) DO UPDATE
SET role_name = EXCLUDED.role_name,
    role_key = EXCLUDED.role_key,
    role_sort = EXCLUDED.role_sort,
    data_scope = EXCLUDED.data_scope,
    dept_id_set = EXCLUDED.dept_id_set,
    status = EXCLUDED.status,
    description = EXCLUDED.description,
    remark = EXCLUDED.remark,
    deleted = FALSE;

INSERT INTO permissions (name, description)
VALUES ('system:role:list', '查询角色列表'),
       ('system:role:query', '查询角色详情'),
       ('system:role:add', '新增角色'),
       ('system:role:edit', '修改角色'),
       ('system:role:remove', '删除角色'),
       ('system:role:export', '导出角色')
ON CONFLICT (name) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
         JOIN permissions p ON p.name IN (
             'system:role:list',
             'system:role:query',
             'system:role:add',
             'system:role:edit',
             'system:role:remove',
             'system:role:export'
         )
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;

INSERT INTO sys_role_menu (role_id, menu_id)
SELECT r.id, m.menu_id
FROM roles r
         CROSS JOIN sys_menu m
WHERE r.name = 'common'
  AND m.deleted = FALSE
ON CONFLICT DO NOTHING;

INSERT INTO permissions (name, description)
SELECT DISTINCT m.permission, m.menu_name
FROM sys_role_menu rm
         JOIN roles r ON r.id = rm.role_id
         JOIN sys_menu m ON m.menu_id = rm.menu_id
WHERE r.name = 'common'
  AND m.permission <> ''
ON CONFLICT (name) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT DISTINCT r.id, p.id
FROM roles r
         JOIN sys_role_menu rm ON rm.role_id = r.id
         JOIN sys_menu m ON m.menu_id = rm.menu_id
         JOIN permissions p ON p.name = m.permission
WHERE r.name = 'common'
  AND m.permission <> ''
ON CONFLICT DO NOTHING;
