-- Keystone-compatible login and operation log management.
-- Department fields here are passive compatibility columns from Keystone's operation log DTO;
-- this migration does not create department/post modules or seed their menus.

CREATE TABLE IF NOT EXISTS sys_login_info
(
    info_id          BIGSERIAL PRIMARY KEY,
    username         VARCHAR(50)  NOT NULL DEFAULT '',
    ip_address       VARCHAR(128) NOT NULL DEFAULT '',
    login_location   VARCHAR(255) NOT NULL DEFAULT '',
    browser          VARCHAR(50)  NOT NULL DEFAULT '',
    operation_system VARCHAR(50)  NOT NULL DEFAULT '',
    status           SMALLINT     NOT NULL DEFAULT 0,
    msg              VARCHAR(255) NOT NULL DEFAULT '',
    login_time       TIMESTAMPTZ  NOT NULL DEFAULT now(),
    deleted          BOOLEAN      NOT NULL DEFAULT FALSE
);

COMMENT ON TABLE sys_login_info IS '系统访问记录，兼容 Keystone 登录日志';
COMMENT ON COLUMN sys_login_info.info_id IS '访问ID';
COMMENT ON COLUMN sys_login_info.username IS '用户账号';
COMMENT ON COLUMN sys_login_info.ip_address IS '登录IP地址';
COMMENT ON COLUMN sys_login_info.login_location IS '登录地点';
COMMENT ON COLUMN sys_login_info.browser IS '浏览器类型';
COMMENT ON COLUMN sys_login_info.operation_system IS '操作系统';
COMMENT ON COLUMN sys_login_info.status IS '登录状态（1成功 2退出 3注册 0失败）';
COMMENT ON COLUMN sys_login_info.msg IS '提示消息';
COMMENT ON COLUMN sys_login_info.login_time IS '访问时间';
COMMENT ON COLUMN sys_login_info.deleted IS '逻辑删除';

CREATE INDEX IF NOT EXISTS idx_sys_login_info_username ON sys_login_info (username);
CREATE INDEX IF NOT EXISTS idx_sys_login_info_status ON sys_login_info (status);
CREATE INDEX IF NOT EXISTS idx_sys_login_info_login_time ON sys_login_info (login_time DESC);
CREATE INDEX IF NOT EXISTS idx_sys_login_info_active ON sys_login_info (deleted);

CREATE TABLE IF NOT EXISTS sys_operation_log
(
    operation_id      BIGSERIAL PRIMARY KEY,
    business_type     SMALLINT      NOT NULL DEFAULT 0,
    request_method    SMALLINT      NOT NULL DEFAULT -1,
    request_module    VARCHAR(64)   NOT NULL DEFAULT '',
    request_url       VARCHAR(256)  NOT NULL DEFAULT '',
    called_method     VARCHAR(128)  NOT NULL DEFAULT '',
    operator_type     SMALLINT      NOT NULL DEFAULT 1,
    user_id           BIGINT        NULL DEFAULT 0,
    username          VARCHAR(32)   NULL DEFAULT '',
    operator_ip       VARCHAR(128)  NULL DEFAULT '',
    operator_location VARCHAR(256)  NULL DEFAULT '',
    dept_id           BIGINT        NULL DEFAULT 0,
    dept_name         VARCHAR(64)   NULL DEFAULT '',
    operation_param   VARCHAR(2048) NULL DEFAULT '',
    operation_result  VARCHAR(2048) NULL DEFAULT '',
    status            SMALLINT      NOT NULL DEFAULT 1,
    error_stack       VARCHAR(2048) NULL DEFAULT '',
    operation_time    TIMESTAMPTZ   NOT NULL DEFAULT now(),
    deleted           BOOLEAN       NOT NULL DEFAULT FALSE
);

COMMENT ON TABLE sys_operation_log IS '操作日志记录，兼容 Keystone 操作日志';
COMMENT ON COLUMN sys_operation_log.operation_id IS '日志主键';
COMMENT ON COLUMN sys_operation_log.business_type IS '业务类型（0其它 1新增 2修改 3删除 4授权 5导出 6导入 7强退 8清空）';
COMMENT ON COLUMN sys_operation_log.request_method IS '请求方式（1 GET 2 POST 3 PUT 4 DELETE -1 UNKNOWN）';
COMMENT ON COLUMN sys_operation_log.request_module IS '请求模块';
COMMENT ON COLUMN sys_operation_log.request_url IS '请求URL';
COMMENT ON COLUMN sys_operation_log.called_method IS '调用方法';
COMMENT ON COLUMN sys_operation_log.operator_type IS '操作类别（1其它 2Web用户 3手机端用户）';
COMMENT ON COLUMN sys_operation_log.user_id IS '用户ID';
COMMENT ON COLUMN sys_operation_log.username IS '操作人员';
COMMENT ON COLUMN sys_operation_log.operator_ip IS '操作人员IP';
COMMENT ON COLUMN sys_operation_log.operator_location IS '操作地点';
COMMENT ON COLUMN sys_operation_log.dept_id IS '部门ID兼容字段';
COMMENT ON COLUMN sys_operation_log.dept_name IS '部门名称兼容字段';
COMMENT ON COLUMN sys_operation_log.operation_param IS '请求参数';
COMMENT ON COLUMN sys_operation_log.operation_result IS '返回参数';
COMMENT ON COLUMN sys_operation_log.status IS '操作状态（1正常 0异常）';
COMMENT ON COLUMN sys_operation_log.error_stack IS '错误消息';
COMMENT ON COLUMN sys_operation_log.operation_time IS '操作时间';
COMMENT ON COLUMN sys_operation_log.deleted IS '逻辑删除';

CREATE INDEX IF NOT EXISTS idx_sys_operation_log_username ON sys_operation_log (username);
CREATE INDEX IF NOT EXISTS idx_sys_operation_log_business_type ON sys_operation_log (business_type);
CREATE INDEX IF NOT EXISTS idx_sys_operation_log_status ON sys_operation_log (status);
CREATE INDEX IF NOT EXISTS idx_sys_operation_log_operation_time ON sys_operation_log (operation_time DESC);
CREATE INDEX IF NOT EXISTS idx_sys_operation_log_active ON sys_operation_log (deleted);

INSERT INTO sys_login_info (
    info_id, username, ip_address, login_location, browser, operation_system,
    status, msg, login_time, deleted
)
VALUES
    (415, 'admin', '127.0.0.1', '内网IP', 'Chrome', 'Mac OS X', 1, '登录成功', '2023-06-29 22:49:37+00', FALSE),
    (416, 'admin', '127.0.0.1', '内网IP', 'Chrome', 'Mac OS X', 1, '登录成功', '2023-07-02 22:12:30+00', FALSE),
    (417, 'admin', '127.0.0.1', '内网IP', 'Chrome', 'Mac OS X', 0, '验证码过期', '2023-07-02 22:16:06+00', FALSE)
ON CONFLICT (info_id) DO NOTHING;

INSERT INTO sys_operation_log (
    operation_id, business_type, request_method, request_module, request_url,
    called_method, operator_type, user_id, username, operator_ip, operator_location,
    dept_id, dept_name, operation_param, operation_result, status, error_stack,
    operation_time, deleted
)
VALUES (
    561, 1, 2, '菜单管理', '/system/menus',
    'app.keystone.admin.controller.system.SysMenuController.add()', 1,
    0, 'admin', '127.0.0.1', '内网IP',
    0, '', '{"menuName":"","permission":"","parentId":2035,"path":"","isButton":false}',
    '', 1, '', '2023-07-22 17:06:57+00', FALSE
)
ON CONFLICT (operation_id) DO NOTHING;

SELECT setval(
    pg_get_serial_sequence('sys_login_info', 'info_id'),
    GREATEST((SELECT COALESCE(MAX(info_id), 1) FROM sys_login_info), 1),
    TRUE
);

SELECT setval(
    pg_get_serial_sequence('sys_operation_log', 'operation_id'),
    GREATEST((SELECT COALESCE(MAX(operation_id), 1) FROM sys_operation_log), 1),
    TRUE
);
