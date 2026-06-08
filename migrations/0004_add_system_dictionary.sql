-- ============================================================================
-- Keystone-compatible system dictionary tables and seed data
-- ============================================================================

CREATE TABLE IF NOT EXISTS sys_dict_type
(
    dict_id    BIGSERIAL PRIMARY KEY,
    dict_name  VARCHAR(100) NOT NULL DEFAULT '',
    dict_type  VARCHAR(100) NOT NULL DEFAULT '',
    status     SMALLINT     NOT NULL DEFAULT 1,
    remark     VARCHAR(500),
    created_at TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_sys_dict_type_type UNIQUE (dict_type),
    CONSTRAINT ck_sys_dict_type_status CHECK (status IN (0, 1))
);

COMMENT ON TABLE sys_dict_type IS '字典类型表';
COMMENT ON COLUMN sys_dict_type.dict_name IS '字典名称';
COMMENT ON COLUMN sys_dict_type.dict_type IS '字典类型';
COMMENT ON COLUMN sys_dict_type.status IS '状态：1正常，0停用';

CREATE TABLE IF NOT EXISTS sys_dict_data
(
    dict_code  BIGSERIAL PRIMARY KEY,
    dict_type  VARCHAR(100) NOT NULL DEFAULT '',
    dict_label VARCHAR(100) NOT NULL DEFAULT '',
    dict_value VARCHAR(100) NOT NULL DEFAULT '',
    dict_sort  INTEGER      NOT NULL DEFAULT 0,
    is_default SMALLINT     NOT NULL DEFAULT 0,
    css_class  VARCHAR(100),
    list_class VARCHAR(100),
    status     SMALLINT     NOT NULL DEFAULT 1,
    remark     VARCHAR(500),
    created_at TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT ck_sys_dict_data_status CHECK (status IN (0, 1)),
    CONSTRAINT ck_sys_dict_data_is_default CHECK (is_default IN (0, 1))
);

COMMENT ON TABLE sys_dict_data IS '字典数据表';
COMMENT ON COLUMN sys_dict_data.dict_type IS '字典类型';
COMMENT ON COLUMN sys_dict_data.dict_label IS '字典标签';
COMMENT ON COLUMN sys_dict_data.dict_value IS '字典键值';
COMMENT ON COLUMN sys_dict_data.list_class IS '表格回显样式，对应 /getConfig 的 cssTag';

CREATE INDEX IF NOT EXISTS idx_sys_dict_data_type ON sys_dict_data (dict_type);
CREATE INDEX IF NOT EXISTS idx_sys_dict_data_type_sort ON sys_dict_data (dict_type, dict_sort);

DO
$$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'update_updated_at') THEN
        IF EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'sys_dict_type_updated_at_trg') THEN
            DROP TRIGGER sys_dict_type_updated_at_trg ON sys_dict_type;
        END IF;
        CREATE TRIGGER sys_dict_type_updated_at_trg
            BEFORE UPDATE ON sys_dict_type
            FOR EACH ROW
        EXECUTE FUNCTION update_updated_at();

        IF EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'sys_dict_data_updated_at_trg') THEN
            DROP TRIGGER sys_dict_data_updated_at_trg ON sys_dict_data;
        END IF;
        CREATE TRIGGER sys_dict_data_updated_at_trg
            BEFORE UPDATE ON sys_dict_data
            FOR EACH ROW
        EXECUTE FUNCTION update_updated_at();
    END IF;
END
$$;

INSERT INTO sys_dict_type (dict_name, dict_type, status, remark)
VALUES ('系统是否', 'common.yesOrNo', 1, '系统是否列表'),
       ('通用状态', 'common.status', 1, '通用状态列表'),
       ('用户性别', 'sysUser.sex', 1, '用户性别列表'),
       ('用户状态', 'sysUser.status', 1, '用户状态列表'),
       ('菜单显示状态', 'sysMenu.isVisible', 1, '菜单显示状态列表'),
       ('通知类型', 'sysNotice.noticeType', 1, '通知类型列表'),
       ('通知状态', 'sysNotice.status', 1, '通知状态列表'),
       ('操作业务类型', 'sysOperationLog.businessType', 1, '操作业务类型列表'),
       ('操作状态', 'sysOperationLog.status', 1, '操作状态列表'),
       ('操作者类型', 'sysOperationLog.operatorType', 1, '操作者类型列表'),
       ('登录状态', 'sysLoginLog.status', 1, '登录状态列表'),
       ('任务状态', 'sysJob.status', 1, '任务状态列表')
ON CONFLICT (dict_type) DO UPDATE
SET dict_name = EXCLUDED.dict_name,
    status = EXCLUDED.status,
    remark = EXCLUDED.remark;

INSERT INTO sys_dict_data (dict_type, dict_label, dict_value, dict_sort, is_default, list_class, status, remark)
SELECT seed.dict_type, seed.dict_label, seed.dict_value, seed.dict_sort, seed.is_default,
       seed.list_class, seed.status, seed.remark
FROM (VALUES
    ('common.yesOrNo', '是', '1', 1, 0, '', 1, '是'),
    ('common.yesOrNo', '否', '0', 2, 0, 'danger', 1, '否'),
    ('common.status', '正常', '1', 1, 0, '', 1, '正常状态'),
    ('common.status', '停用', '0', 2, 0, 'danger', 1, '停用状态'),
    ('sysUser.sex', '女', '0', 1, 0, '', 1, '性别女'),
    ('sysUser.sex', '男', '1', 2, 0, '', 1, '性别男'),
    ('sysUser.sex', '未知', '2', 3, 0, '', 1, '性别未知'),
    ('sysUser.status', '正常', '1', 1, 0, '', 1, '用户正常'),
    ('sysUser.status', '禁用', '2', 2, 0, 'danger', 1, '用户禁用'),
    ('sysUser.status', '冻结', '3', 3, 0, 'warning', 1, '用户冻结'),
    ('sysMenu.isVisible', '显示', '1', 1, 0, '', 1, '显示菜单'),
    ('sysMenu.isVisible', '隐藏', '0', 2, 0, 'danger', 1, '隐藏菜单'),
    ('sysNotice.noticeType', '通知', '1', 1, 0, 'warning', 1, '通知'),
    ('sysNotice.noticeType', '公告', '2', 2, 0, 'success', 1, '公告'),
    ('sysNotice.status', '正常', '1', 1, 0, '', 1, '正常状态'),
    ('sysNotice.status', '关闭', '0', 2, 0, 'danger', 1, '关闭状态'),
    ('sysOperationLog.businessType', '其他操作', '0', 1, 0, 'info', 1, '其他操作'),
    ('sysOperationLog.businessType', '添加', '1', 2, 0, '', 1, '添加操作'),
    ('sysOperationLog.businessType', '修改', '2', 3, 0, '', 1, '修改操作'),
    ('sysOperationLog.businessType', '删除', '3', 4, 0, 'danger', 1, '删除操作'),
    ('sysOperationLog.businessType', '授权', '4', 5, 0, '', 1, '授权操作'),
    ('sysOperationLog.businessType', '导出', '5', 6, 0, 'warning', 1, '导出操作'),
    ('sysOperationLog.businessType', '导入', '6', 7, 0, 'warning', 1, '导入操作'),
    ('sysOperationLog.businessType', '强退', '7', 8, 0, 'danger', 1, '强退操作'),
    ('sysOperationLog.businessType', '清空', '8', 9, 0, 'danger', 1, '清空操作'),
    ('sysOperationLog.status', '成功', '1', 1, 0, '', 1, '操作成功'),
    ('sysOperationLog.status', '失败', '0', 2, 0, 'danger', 1, '操作失败'),
    ('sysOperationLog.operatorType', '其他', '1', 1, 0, '', 1, '其他操作者'),
    ('sysOperationLog.operatorType', 'Web用户', '2', 2, 0, '', 1, 'Web用户'),
    ('sysOperationLog.operatorType', '手机端用户', '3', 3, 0, '', 1, '手机端用户'),
    ('sysLoginLog.status', '登录成功', '1', 1, 0, 'success', 1, '登录成功'),
    ('sysLoginLog.status', '退出成功', '2', 2, 0, 'info', 1, '退出成功'),
    ('sysLoginLog.status', '注册', '3', 3, 0, '', 1, '注册'),
    ('sysLoginLog.status', '登录失败', '0', 4, 0, 'danger', 1, '登录失败'),
    ('sysJob.status', '正常', '1', 1, 0, '', 1, '任务正常'),
    ('sysJob.status', '暂停', '0', 2, 0, 'danger', 1, '任务暂停')
) AS seed(dict_type, dict_label, dict_value, dict_sort, is_default, list_class, status, remark)
WHERE NOT EXISTS (
    SELECT 1
    FROM sys_dict_data existing
    WHERE existing.dict_type = seed.dict_type
      AND existing.dict_value = seed.dict_value
);

INSERT INTO permissions (name, description)
VALUES ('system:dict:list', '查询字典列表'),
       ('system:dict:query', '查询字典详情'),
       ('system:dict:add', '新增字典'),
       ('system:dict:edit', '修改字典'),
       ('system:dict:remove', '删除字典')
ON CONFLICT (name) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
         JOIN permissions p ON p.name IN (
             'system:dict:list',
             'system:dict:query',
             'system:dict:add',
             'system:dict:edit',
             'system:dict:remove'
         )
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;
