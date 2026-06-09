-- Keystone-compatible department, post and user management.
-- Keeps Agora's user_info table as the login source while adding Keystone-style
-- organization fields and management endpoints.

CREATE TABLE IF NOT EXISTS sys_dept
(
    dept_id     BIGSERIAL PRIMARY KEY,
    parent_id   BIGINT       NOT NULL DEFAULT 0,
    ancestors   VARCHAR(512) NOT NULL DEFAULT '0',
    dept_name   VARCHAR(64)  NOT NULL,
    order_num   INTEGER      NOT NULL DEFAULT 0,
    leader_id   BIGINT,
    leader_name VARCHAR(64),
    phone       VARCHAR(32),
    email       VARCHAR(128),
    status      SMALLINT     NOT NULL DEFAULT 1,
    creator_id  BIGINT REFERENCES user_info (id),
    updater_id  BIGINT REFERENCES user_info (id),
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted     BOOLEAN      NOT NULL DEFAULT FALSE,
    CONSTRAINT ck_sys_dept_status CHECK (status IN (0, 1))
);

COMMENT ON TABLE sys_dept IS '部门表，兼容 Keystone sys_dept';
COMMENT ON COLUMN sys_dept.status IS '状态：1正常，0停用';

CREATE INDEX IF NOT EXISTS idx_sys_dept_parent ON sys_dept (parent_id);
CREATE INDEX IF NOT EXISTS idx_sys_dept_status ON sys_dept (status);
CREATE UNIQUE INDEX IF NOT EXISTS uq_sys_dept_name_parent_active
    ON sys_dept (dept_name, parent_id)
    WHERE deleted = FALSE;

CREATE TABLE IF NOT EXISTS sys_post
(
    post_id    BIGSERIAL PRIMARY KEY,
    post_code  VARCHAR(64)  NOT NULL,
    post_name  VARCHAR(64)  NOT NULL,
    post_sort  INTEGER      NOT NULL DEFAULT 0,
    status     SMALLINT     NOT NULL DEFAULT 1,
    remark     VARCHAR(512),
    creator_id BIGINT REFERENCES user_info (id),
    updater_id BIGINT REFERENCES user_info (id),
    created_at TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted    BOOLEAN      NOT NULL DEFAULT FALSE,
    CONSTRAINT ck_sys_post_status CHECK (status IN (0, 1))
);

COMMENT ON TABLE sys_post IS '岗位信息表，兼容 Keystone sys_post';
COMMENT ON COLUMN sys_post.status IS '状态：1正常，0停用';

CREATE INDEX IF NOT EXISTS idx_sys_post_status ON sys_post (status);
CREATE UNIQUE INDEX IF NOT EXISTS uq_sys_post_code_active
    ON sys_post (post_code)
    WHERE deleted = FALSE;
CREATE UNIQUE INDEX IF NOT EXISTS uq_sys_post_name_active
    ON sys_post (post_name)
    WHERE deleted = FALSE;

DO
$$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'update_updated_at') THEN
        IF EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'sys_dept_updated_at_trg') THEN
            DROP TRIGGER sys_dept_updated_at_trg ON sys_dept;
        END IF;
        CREATE TRIGGER sys_dept_updated_at_trg
            BEFORE UPDATE ON sys_dept
            FOR EACH ROW
        EXECUTE FUNCTION update_updated_at();

        IF EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'sys_post_updated_at_trg') THEN
            DROP TRIGGER sys_post_updated_at_trg ON sys_post;
        END IF;
        CREATE TRIGGER sys_post_updated_at_trg
            BEFORE UPDATE ON sys_post
            FOR EACH ROW
        EXECUTE FUNCTION update_updated_at();
    END IF;
END
$$;

INSERT INTO sys_dept (
    dept_id, parent_id, ancestors, dept_name, order_num, leader_name, phone,
    email, status, created_at, deleted
)
SELECT seed.dept_id, seed.parent_id, seed.ancestors, seed.dept_name,
       seed.order_num, seed.leader_name, seed.phone, seed.email,
       seed.status, seed.created_at::timestamptz, FALSE
FROM (VALUES
    (1, 0, '0', 'Keystone科技', 0, 'valarchie', '15888888888', 'valarchie@163.com', 1, '2022-05-21 08:30:54+00'),
    (2, 1, '0,1', '深圳总公司', 1, 'valarchie', '15888888888', 'valarchie@163.com', 1, '2022-05-21 08:30:54+00'),
    (3, 1, '0,1', '长沙分公司', 2, 'valarchie', '15888888888', 'valarchie@163.com', 1, '2022-05-21 08:30:54+00'),
    (4, 2, '0,1,2', '研发部门', 1, 'valarchie', '15888888888', 'valarchie@163.com', 1, '2022-05-21 08:30:54+00'),
    (5, 2, '0,1,2', '市场部门', 2, 'valarchie', '15888888888', 'valarchie@163.com', 0, '2022-05-21 08:30:54+00'),
    (6, 2, '0,1,2', '测试部门', 3, 'valarchie', '15888888888', 'valarchie@163.com', 1, '2022-05-21 08:30:54+00'),
    (7, 2, '0,1,2', '财务部门', 4, 'valarchie', '15888888888', 'valarchie@163.com', 1, '2022-05-21 08:30:54+00'),
    (8, 2, '0,1,2', '运维部门', 5, 'valarchie', '15888888888', 'valarchie@163.com', 1, '2022-05-21 08:30:54+00'),
    (9, 3, '0,1,3', '市场部', 1, 'valarchie', '15888188888', 'valarchie@163.com', 0, '2022-05-21 08:30:54+00'),
    (10, 3, '0,1,3', '财务部', 2, 'valarchie', '15888888888', 'valarchie@163.com', 0, '2022-05-21 08:30:54+00')
) AS seed(dept_id, parent_id, ancestors, dept_name, order_num, leader_name, phone, email, status, created_at)
ON CONFLICT (dept_id) DO UPDATE
SET parent_id = EXCLUDED.parent_id,
    ancestors = EXCLUDED.ancestors,
    dept_name = EXCLUDED.dept_name,
    order_num = EXCLUDED.order_num,
    leader_name = EXCLUDED.leader_name,
    phone = EXCLUDED.phone,
    email = EXCLUDED.email,
    status = EXCLUDED.status,
    deleted = FALSE;

SELECT setval(
    pg_get_serial_sequence('sys_dept', 'dept_id'),
    GREATEST((SELECT COALESCE(MAX(dept_id), 1) FROM sys_dept), 1),
    TRUE
);

INSERT INTO sys_post (
    post_id, post_code, post_name, post_sort, status, remark, created_at, deleted
)
SELECT seed.post_id, seed.post_code, seed.post_name, seed.post_sort,
       seed.status, seed.remark, seed.created_at::timestamptz, FALSE
FROM (VALUES
    (1, 'ceo', '董事长', 1, 1, '', '2022-05-21 08:30:54+00'),
    (2, 'se', '项目经理', 2, 1, '', '2022-05-21 08:30:54+00'),
    (3, 'hr', '人力资源', 3, 1, '', '2022-05-21 08:30:54+00'),
    (4, 'user', '普通员工', 5, 0, '', '2022-05-21 08:30:54+00')
) AS seed(post_id, post_code, post_name, post_sort, status, remark, created_at)
ON CONFLICT (post_id) DO UPDATE
SET post_code = EXCLUDED.post_code,
    post_name = EXCLUDED.post_name,
    post_sort = EXCLUDED.post_sort,
    status = EXCLUDED.status,
    remark = EXCLUDED.remark,
    deleted = FALSE;

SELECT setval(
    pg_get_serial_sequence('sys_post', 'post_id'),
    GREATEST((SELECT COALESCE(MAX(post_id), 1) FROM sys_post), 1),
    TRUE
);

ALTER TABLE user_info ADD COLUMN IF NOT EXISTS post_id BIGINT REFERENCES sys_post (post_id);
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS dept_id BIGINT REFERENCES sys_dept (dept_id);
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS nickname VARCHAR(128);
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS user_type SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS phone_number VARCHAR(32);
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS sex SMALLINT NOT NULL DEFAULT 2;
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS avatar TEXT;
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS user_status SMALLINT NOT NULL DEFAULT 1;
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS login_ip VARCHAR(64);
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS login_date TIMESTAMPTZ;
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS is_admin BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS external_subject VARCHAR(255);
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS external_user_id VARCHAR(128);
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS remark VARCHAR(512);
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS creator_id BIGINT REFERENCES user_info (id);
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS updater_id BIGINT REFERENCES user_info (id);
ALTER TABLE user_info ADD COLUMN IF NOT EXISTS deleted BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE user_info
SET nickname = COALESCE(nickname, NULLIF(display_name, ''), username),
    avatar = COALESCE(avatar, avatar_url),
    user_status = CASE status
                      WHEN 'active' THEN 1
                      WHEN 'inactive' THEN 0
                      WHEN 'suspended' THEN 0
                      WHEN 'frozen' THEN 0
                      ELSE COALESCE(user_status, 1)
                  END,
    is_admin = CASE username WHEN 'admin' THEN TRUE ELSE is_admin END,
    dept_id = COALESCE(dept_id, CASE username
                                    WHEN 'admin' THEN 1
                                    WHEN 'editor' THEN 4
                                    WHEN 'user' THEN 5
                                    WHEN 'gh_user' THEN 6
                                    ELSE 1
                                END),
    post_id = COALESCE(post_id, CASE username
                                    WHEN 'admin' THEN 1
                                    WHEN 'editor' THEN 2
                                    WHEN 'user' THEN 4
                                    WHEN 'gh_user' THEN 4
                                    ELSE 4
                                END),
    remark = COALESCE(remark, '')
WHERE deleted = FALSE;

DO
$$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'ck_user_info_user_status'
    ) THEN
        ALTER TABLE user_info ADD CONSTRAINT ck_user_info_user_status CHECK (user_status IN (0, 1, 2, 3));
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'ck_user_info_sex'
    ) THEN
        ALTER TABLE user_info ADD CONSTRAINT ck_user_info_sex CHECK (sex IN (0, 1, 2));
    END IF;
END
$$;

CREATE INDEX IF NOT EXISTS idx_user_info_dept_id ON user_info (dept_id);
CREATE INDEX IF NOT EXISTS idx_user_info_post_id ON user_info (post_id);
CREATE INDEX IF NOT EXISTS idx_user_info_user_status ON user_info (user_status);
CREATE INDEX IF NOT EXISTS idx_user_info_deleted ON user_info (deleted);
CREATE UNIQUE INDEX IF NOT EXISTS uq_user_info_phone_active
    ON user_info (phone_number)
    WHERE phone_number IS NOT NULL AND phone_number <> '' AND deleted = FALSE;
CREATE UNIQUE INDEX IF NOT EXISTS uq_user_info_external_user_id_active
    ON user_info (external_user_id)
    WHERE external_user_id IS NOT NULL AND external_user_id <> '';

INSERT INTO sys_menu (
    menu_id, menu_name, menu_type, router_name, parent_id, path, is_button,
    permission, meta_info, status, remark, created_at, deleted
)
SELECT seed.menu_id, seed.menu_name, seed.menu_type, seed.router_name, seed.parent_id,
       seed.path, seed.is_button, seed.permission, seed.meta_info::jsonb,
       seed.status, seed.remark, seed.created_at::timestamptz, FALSE
FROM (VALUES
    (8, '部门管理', 1, 'Department', 1, '/system/dept/index', FALSE, 'system:dept:list', '{"title":"部门管理","icon":"fa-solid:code-branch","showParent":true}', 1, '部门管理菜单', '2022-05-21 08:30:54+00'),
    (9, '岗位管理', 1, 'Post', 1, '/system/post/index', FALSE, 'system:post:list', '{"title":"岗位管理","icon":"ep:postcard","showParent":true}', 1, '岗位管理菜单', '2022-05-21 08:30:54+00'),
    (36, '部门查询', 0, ' ', 8, '', TRUE, 'system:dept:query', '{"title":"部门查询"}', 1, '', '2022-05-21 08:30:54+00'),
    (37, '部门新增', 0, ' ', 8, '', TRUE, 'system:dept:add', '{"title":"部门新增"}', 1, '', '2022-05-21 08:30:54+00'),
    (38, '部门修改', 0, ' ', 8, '', TRUE, 'system:dept:edit', '{"title":"部门修改"}', 1, '', '2022-05-21 08:30:54+00'),
    (39, '部门删除', 0, ' ', 8, '', TRUE, 'system:dept:remove', '{"title":"部门删除"}', 1, '', '2022-05-21 08:30:54+00'),
    (40, '岗位查询', 0, ' ', 9, '', TRUE, 'system:post:query', '{"title":"岗位查询"}', 1, '', '2022-05-21 08:30:54+00'),
    (41, '岗位新增', 0, ' ', 9, '', TRUE, 'system:post:add', '{"title":"岗位新增"}', 1, '', '2022-05-21 08:30:54+00'),
    (42, '岗位修改', 0, ' ', 9, '', TRUE, 'system:post:edit', '{"title":"岗位修改"}', 1, '', '2022-05-21 08:30:54+00'),
    (43, '岗位删除', 0, ' ', 9, '', TRUE, 'system:post:remove', '{"title":"岗位删除"}', 1, '', '2022-05-21 08:30:54+00'),
    (44, '岗位导出', 0, ' ', 9, '', TRUE, 'system:post:export', '{"title":"岗位导出"}', 1, '', '2022-05-21 08:30:54+00')
) AS seed(menu_id, menu_name, menu_type, router_name, parent_id, path, is_button, permission, meta_info, status, remark, created_at)
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
WHERE permission <> ''
  AND deleted = FALSE
ON CONFLICT (name) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
         JOIN permissions p ON p.name IN (
             SELECT permission FROM sys_menu WHERE permission <> '' AND deleted = FALSE
         )
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;

INSERT INTO sys_role_menu (role_id, menu_id)
SELECT r.id, m.menu_id
FROM roles r
         CROSS JOIN sys_menu m
WHERE r.name = 'admin'
  AND m.deleted = FALSE
ON CONFLICT DO NOTHING;

INSERT INTO sys_role_menu (role_id, menu_id)
SELECT r.id, m.menu_id
FROM roles r
         CROSS JOIN sys_menu m
WHERE r.name = 'common'
  AND m.menu_id IN (1, 5, 8, 9, 20, 21, 22, 23, 26, 36, 37, 38, 39, 40, 41, 42, 43, 44)
  AND m.deleted = FALSE
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT DISTINCT r.id, p.id
FROM roles r
         JOIN sys_role_menu rm ON rm.role_id = r.id
         JOIN sys_menu m ON m.menu_id = rm.menu_id
         JOIN permissions p ON p.name = m.permission
WHERE r.name IN ('admin', 'common')
  AND m.permission <> ''
ON CONFLICT DO NOTHING;
