-- ============================================================================
-- Keystone-compatible system menu tables and seed data
-- Posts and departments are intentionally omitted per product scope.
-- ============================================================================

CREATE TABLE IF NOT EXISTS sys_menu
(
    menu_id     BIGSERIAL PRIMARY KEY,
    menu_name   VARCHAR(64)  NOT NULL,
    menu_type   SMALLINT     NOT NULL DEFAULT 0,
    router_name VARCHAR(255) NOT NULL DEFAULT '',
    parent_id   BIGINT       NOT NULL DEFAULT 0,
    path        VARCHAR(255) NOT NULL DEFAULT '',
    is_button   BOOLEAN      NOT NULL DEFAULT FALSE,
    permission  VARCHAR(128) NOT NULL DEFAULT '',
    meta_info   JSONB        NOT NULL DEFAULT '{}'::jsonb,
    status      SMALLINT     NOT NULL DEFAULT 0,
    remark      VARCHAR(256)          DEFAULT '',
    creator_id  BIGINT REFERENCES user_info (id),
    updater_id  BIGINT REFERENCES user_info (id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted     BOOLEAN      NOT NULL DEFAULT FALSE,
    CONSTRAINT ck_sys_menu_type CHECK (menu_type IN (0, 1, 2, 3, 4)),
    CONSTRAINT ck_sys_menu_status CHECK (status IN (0, 1)),
    CONSTRAINT ck_sys_menu_meta_object CHECK (jsonb_typeof(meta_info) = 'object')
);

COMMENT ON TABLE sys_menu IS '菜单权限表';
COMMENT ON COLUMN sys_menu.menu_type IS '菜单类型：0按钮，1页面，2目录，3内嵌Iframe，4外链跳转';
COMMENT ON COLUMN sys_menu.is_button IS '是否按钮权限';
COMMENT ON COLUMN sys_menu.permission IS '权限标识';
COMMENT ON COLUMN sys_menu.meta_info IS '前端路由元信息';

CREATE INDEX IF NOT EXISTS idx_sys_menu_parent ON sys_menu (parent_id);
CREATE INDEX IF NOT EXISTS idx_sys_menu_permission ON sys_menu (permission);
CREATE INDEX IF NOT EXISTS idx_sys_menu_status ON sys_menu (status);
CREATE UNIQUE INDEX IF NOT EXISTS uq_sys_menu_name_parent_active
    ON sys_menu (menu_name, parent_id)
    WHERE deleted = FALSE;

CREATE TABLE IF NOT EXISTS sys_role_menu
(
    role_id    BIGINT      NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    menu_id    BIGINT      NOT NULL REFERENCES sys_menu (menu_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (role_id, menu_id)
);

COMMENT ON TABLE sys_role_menu IS '角色和菜单关联表';

DO
$$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'update_updated_at') THEN
        IF EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'sys_menu_updated_at_trg') THEN
            DROP TRIGGER sys_menu_updated_at_trg ON sys_menu;
        END IF;
        CREATE TRIGGER sys_menu_updated_at_trg
            BEFORE UPDATE ON sys_menu
            FOR EACH ROW
        EXECUTE FUNCTION update_updated_at();

        IF EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'sys_role_menu_updated_at_trg') THEN
            DROP TRIGGER sys_role_menu_updated_at_trg ON sys_role_menu;
        END IF;
        CREATE TRIGGER sys_role_menu_updated_at_trg
            BEFORE UPDATE ON sys_role_menu
            FOR EACH ROW
        EXECUTE FUNCTION update_updated_at();
    END IF;
END
$$;

INSERT INTO sys_menu (
    menu_id, menu_name, menu_type, router_name, parent_id, path, is_button,
    permission, meta_info, status, remark, created_at, deleted
)
SELECT seed.menu_id, seed.menu_name, seed.menu_type, seed.router_name, seed.parent_id,
       seed.path, seed.is_button, seed.permission, seed.meta_info::jsonb,
       seed.status, seed.remark, seed.created_at::timestamptz, FALSE
FROM (VALUES
    (1, '系统管理', 2, '', 0, '/system', FALSE, '', '{"title":"系统管理","icon":"ep:management","showParent":true,"rank":1}', 1, '系统管理目录', '2022-05-21 08:30:54+00'),
    (2, '系统监控', 2, '', 0, '/monitor', FALSE, '', '{"title":"系统监控","icon":"ep:monitor","showParent":true,"rank":3}', 1, '系统监控目录', '2022-05-21 08:30:54+00'),
    (3, '系统工具', 2, '', 0, '/tool', FALSE, '', '{"title":"系统工具","icon":"ep:tools","showParent":true,"rank":2}', 1, '系统工具目录', '2022-05-21 08:30:54+00'),
    (4, 'Keystone官网', 3, 'KeystoneguanwangIframeRouter', 0, '/KeystoneguanwangIframeLink', FALSE, '', '{"title":"Keystone官网","icon":"ep:link","showParent":true,"frameSrc":"https://element-plus.org/zh-CN/","rank":8}', 1, 'Agileboot官网地址', '2022-05-21 08:30:54+00'),
    (5, '用户管理', 1, 'SystemUser', 1, '/system/user/index', FALSE, 'system:user:list', '{"title":"用户管理","icon":"ep:user-filled","showParent":true}', 1, '用户管理菜单', '2022-05-21 08:30:54+00'),
    (6, '角色管理', 1, 'SystemRole', 1, '/system/role/index', FALSE, 'system:role:list', '{"title":"角色管理","icon":"ep:user","showParent":true}', 1, '角色管理菜单', '2022-05-21 08:30:54+00'),
    (7, '菜单管理', 1, 'MenuManagement', 1, '/system/menu/index', FALSE, 'system:menu:list', '{"title":"菜单管理","icon":"ep:menu","showParent":true}', 1, '菜单管理菜单', '2022-05-21 08:30:54+00'),
    (10, '参数设置', 1, 'Config', 1, '/system/config/index', FALSE, 'system:config:list', '{"title":"参数设置","icon":"ep:setting","showParent":true}', 1, '参数设置菜单', '2022-05-21 08:30:54+00'),
    (11, '通知公告', 1, 'SystemNotice', 1, '/system/notice/index', FALSE, 'system:notice:list', '{"title":"通知公告","icon":"ep:notification","showParent":true}', 1, '通知公告菜单', '2022-05-21 08:30:54+00'),
    (12, '日志管理', 1, 'LogManagement', 1, '/system/logd', FALSE, '', '{"title":"日志管理","icon":"ep:document","showParent":true}', 1, '日志管理菜单', '2022-05-21 08:30:54+00'),
    (13, '在线用户', 1, 'OnlineUser', 2, '/system/monitor/onlineUser/index', FALSE, 'monitor:online:list', '{"title":"在线用户","icon":"fa-solid:users","showParent":true}', 1, '在线用户菜单', '2022-05-21 08:30:54+00'),
    (14, '数据监控', 1, 'DataMonitor', 2, '/system/monitor/druid/index', FALSE, 'monitor:druid:list', '{"title":"数据监控","icon":"fa:database","showParent":true,"frameSrc":"/druid/login.html","isFrameSrcInternal":true}', 1, '数据监控菜单', '2022-05-21 08:30:54+00'),
    (15, '服务监控', 1, 'ServerInfo', 2, '/system/monitor/server/index', FALSE, 'monitor:server:list', '{"title":"服务监控","icon":"fa:server","showParent":true}', 1, '服务监控菜单', '2022-05-21 08:30:54+00'),
    (16, '缓存监控', 1, 'CacheInfo', 2, '/system/monitor/cache/index', FALSE, 'monitor:cache:list', '{"title":"缓存监控","icon":"ep:reading","showParent":true}', 1, '缓存监控菜单', '2022-05-21 08:30:54+00'),
    (17, '系统接口', 1, 'SystemAPI', 3, '/tool/swagger/index', FALSE, 'tool:swagger:list', '{"title":"系统接口","icon":"ep:document-remove","showParent":true,"frameSrc":"/swagger-ui/index.html","isFrameSrcInternal":true}', 1, '系统接口菜单', '2022-05-21 08:30:54+00'),
    (18, '操作日志', 1, 'OperationLog', 12, '/system/log/operationLog/index', FALSE, 'monitor:operlog:list', '{"title":"操作日志"}', 1, '操作日志菜单', '2022-05-21 08:30:54+00'),
    (19, '登录日志', 1, 'LoginLog', 12, '/system/log/loginLog/index', FALSE, 'monitor:logininfor:list', '{"title":"登录日志"}', 1, '登录日志菜单', '2022-05-21 08:30:54+00'),
    (20, '用户查询', 0, ' ', 5, '', TRUE, 'system:user:query', '{"title":"用户查询"}', 1, '', '2022-05-21 08:30:54+00'),
    (21, '用户新增', 0, ' ', 5, '', TRUE, 'system:user:add', '{"title":"用户新增"}', 1, '', '2022-05-21 08:30:54+00'),
    (22, '用户修改', 0, ' ', 5, '', TRUE, 'system:user:edit', '{"title":"用户修改"}', 1, '', '2022-05-21 08:30:54+00'),
    (23, '用户删除', 0, ' ', 5, '', TRUE, 'system:user:remove', '{"title":"用户删除"}', 1, '', '2022-05-21 08:30:54+00'),
    (24, '用户导出', 0, ' ', 5, '', TRUE, 'system:user:export', '{"title":"用户导出"}', 1, '', '2022-05-21 08:30:54+00'),
    (25, '用户导入', 0, ' ', 5, '', TRUE, 'system:user:import', '{"title":"用户导入"}', 1, '', '2022-05-21 08:30:54+00'),
    (26, '重置密码', 0, ' ', 5, '', TRUE, 'system:user:resetPwd', '{"title":"重置密码"}', 1, '', '2022-05-21 08:30:54+00'),
    (27, '角色查询', 0, ' ', 6, '', TRUE, 'system:role:query', '{"title":"角色查询"}', 1, '', '2022-05-21 08:30:54+00'),
    (28, '角色新增', 0, ' ', 6, '', TRUE, 'system:role:add', '{"title":"角色新增"}', 1, '', '2022-05-21 08:30:54+00'),
    (29, '角色修改', 0, ' ', 6, '', TRUE, 'system:role:edit', '{"title":"角色修改"}', 1, '', '2022-05-21 08:30:54+00'),
    (30, '角色删除', 0, ' ', 6, '', TRUE, 'system:role:remove', '{"title":"角色删除"}', 1, '', '2022-05-21 08:30:54+00'),
    (31, '角色导出', 0, ' ', 6, '', TRUE, 'system:role:export', '{"title":"角色导出"}', 1, '', '2022-05-21 08:30:54+00'),
    (32, '菜单查询', 0, ' ', 7, '', TRUE, 'system:menu:query', '{"title":"菜单查询"}', 1, '', '2022-05-21 08:30:54+00'),
    (33, '菜单新增', 0, ' ', 7, '', TRUE, 'system:menu:add', '{"title":"菜单新增"}', 1, '', '2022-05-21 08:30:54+00'),
    (34, '菜单修改', 0, ' ', 7, '', TRUE, 'system:menu:edit', '{"title":"菜单修改"}', 1, '', '2022-05-21 08:30:54+00'),
    (35, '菜单删除', 0, ' ', 7, '', TRUE, 'system:menu:remove', '{"title":"菜单删除"}', 1, '', '2022-05-21 08:30:54+00'),
    (45, '参数查询', 0, ' ', 10, '', TRUE, 'system:config:query', '{"title":"参数查询"}', 1, '', '2022-05-21 08:30:54+00'),
    (46, '参数新增', 0, ' ', 10, '', TRUE, 'system:config:add', '{"title":"参数新增"}', 1, '', '2022-05-21 08:30:54+00'),
    (47, '参数修改', 0, ' ', 10, '', TRUE, 'system:config:edit', '{"title":"参数修改"}', 1, '', '2022-05-21 08:30:54+00'),
    (48, '参数删除', 0, ' ', 10, '', TRUE, 'system:config:remove', '{"title":"参数删除"}', 1, '', '2022-05-21 08:30:54+00'),
    (49, '参数导出', 0, ' ', 10, '', TRUE, 'system:config:export', '{"title":"参数导出"}', 1, '', '2022-05-21 08:30:54+00'),
    (50, '公告查询', 0, ' ', 11, '', TRUE, 'system:notice:query', '{"title":"公告查询"}', 1, '', '2022-05-21 08:30:54+00'),
    (51, '公告新增', 0, ' ', 11, '', TRUE, 'system:notice:add', '{"title":"公告新增"}', 1, '', '2022-05-21 08:30:54+00'),
    (52, '公告修改', 0, ' ', 11, '', TRUE, 'system:notice:edit', '{"title":"公告修改"}', 1, '', '2022-05-21 08:30:54+00'),
    (53, '公告删除', 0, ' ', 11, '', TRUE, 'system:notice:remove', '{"title":"公告删除"}', 1, '', '2022-05-21 08:30:54+00'),
    (54, '操作查询', 0, ' ', 18, '', TRUE, 'monitor:operlog:query', '{"title":"操作查询"}', 1, '', '2022-05-21 08:30:54+00'),
    (55, '操作删除', 0, ' ', 18, '', TRUE, 'monitor:operlog:remove', '{"title":"操作删除"}', 1, '', '2022-05-21 08:30:54+00'),
    (56, '日志导出', 0, ' ', 18, '', TRUE, 'monitor:operlog:export', '{"title":"日志导出"}', 1, '', '2022-05-21 08:30:54+00'),
    (57, '登录查询', 0, ' ', 19, '', TRUE, 'monitor:logininfor:query', '{"title":"登录查询"}', 1, '', '2022-05-21 08:30:54+00'),
    (58, '登录删除', 0, ' ', 19, '', TRUE, 'monitor:logininfor:remove', '{"title":"登录删除"}', 1, '', '2022-05-21 08:30:54+00'),
    (59, '日志导出', 0, ' ', 19, '', TRUE, 'monitor:logininfor:export', '{"title":"日志导出","rank":22}', 1, '', '2022-05-21 08:30:54+00'),
    (60, '在线查询', 0, ' ', 13, '', TRUE, 'monitor:online:query', '{"title":"在线查询"}', 1, '', '2022-05-21 08:30:54+00'),
    (61, '批量强退', 0, ' ', 13, '', TRUE, 'monitor:online:batchLogout', '{"title":"批量强退"}', 1, '', '2022-05-21 08:30:54+00'),
    (62, '单条强退', 0, ' ', 13, '', TRUE, 'monitor:online:forceLogout', '{"title":"单条强退"}', 1, '', '2022-05-21 08:30:54+00'),
    (63, 'Keystone Github地址', 4, 'https://github.com/bruceblink/Keystone', 0, '/external', FALSE, '', '{"title":"Keystone Github地址","icon":"fa-solid:external-link-alt","showParent":true,"rank":9}', 1, 'Keystone github地址', '2022-05-21 08:30:54+00')
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
