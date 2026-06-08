-- ============================================================================
-- Keystone-compatible system configuration table and seed data
-- ============================================================================

CREATE TABLE IF NOT EXISTS sys_config
(
    config_id       BIGSERIAL PRIMARY KEY,
    config_name     VARCHAR(128) NOT NULL DEFAULT '',
    config_key      VARCHAR(128) NOT NULL DEFAULT '',
    config_options  JSONB        NOT NULL DEFAULT '[]'::jsonb,
    config_value    VARCHAR(256) NOT NULL DEFAULT '',
    is_allow_change BOOLEAN      NOT NULL,
    remark          VARCHAR(128),
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_sys_config_key UNIQUE (config_key),
    CONSTRAINT ck_sys_config_options_array CHECK (jsonb_typeof(config_options) = 'array')
);

COMMENT ON TABLE sys_config IS '参数配置表';
COMMENT ON COLUMN sys_config.config_name IS '配置名称';
COMMENT ON COLUMN sys_config.config_key IS '配置键名';
COMMENT ON COLUMN sys_config.config_options IS '可选的配置值列表';
COMMENT ON COLUMN sys_config.config_value IS '配置值';
COMMENT ON COLUMN sys_config.is_allow_change IS '是否允许修改';

DO
$$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'update_updated_at') THEN
        IF EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'sys_config_updated_at_trg') THEN
            DROP TRIGGER sys_config_updated_at_trg ON sys_config;
        END IF;
        CREATE TRIGGER sys_config_updated_at_trg
            BEFORE UPDATE ON sys_config
            FOR EACH ROW
        EXECUTE FUNCTION update_updated_at();
    END IF;
END
$$;

INSERT INTO sys_config (config_name, config_key, config_options, config_value, is_allow_change, remark)
VALUES ('主框架页-默认皮肤样式名称', 'sys.index.skinName',
        '["skin-blue","skin-green","skin-purple","skin-red","skin-yellow"]'::jsonb,
        'skin-blue', true, '蓝色 skin-blue、绿色 skin-green、紫色 skin-purple、红色 skin-red、黄色 skin-yellow'),
       ('用户管理-账号初始密码', 'sys.user.initPassword',
        '[]'::jsonb, '123456', true, '初始化密码 123456'),
       ('主框架页-侧边栏主题', 'sys.index.sideTheme',
        '["theme-dark","theme-light"]'::jsonb, 'theme-dark', true, '深色主题theme-dark，浅色主题theme-light'),
       ('账号自助-验证码开关', 'sys.account.captchaOnOff',
        '["true","false"]'::jsonb, 'false', false, '是否开启验证码功能（true开启，false关闭）'),
       ('账号自助-是否开启用户注册功能', 'sys.account.registerUser',
        '["true","false"]'::jsonb, 'true', false, '是否开启注册用户功能（true开启，false关闭）')
ON CONFLICT (config_key) DO UPDATE
SET config_name = EXCLUDED.config_name,
    config_options = EXCLUDED.config_options,
    config_value = EXCLUDED.config_value,
    is_allow_change = EXCLUDED.is_allow_change,
    remark = EXCLUDED.remark;

INSERT INTO permissions (name, description)
VALUES ('system:config:list', '查询参数配置列表'),
       ('system:config:query', '查询参数配置详情'),
       ('system:config:edit', '修改参数配置'),
       ('system:config:remove', '刷新参数配置缓存')
ON CONFLICT (name) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
         JOIN permissions p ON p.name IN (
             'system:config:list',
             'system:config:query',
             'system:config:edit',
             'system:config:remove'
         )
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;
