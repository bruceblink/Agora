-- ============================================================================
-- Keystone-compatible system notice table and seed data
-- ============================================================================

CREATE TABLE IF NOT EXISTS sys_notice
(
    notice_id      BIGSERIAL PRIMARY KEY,
    notice_title   VARCHAR(64) NOT NULL,
    notice_type    SMALLINT    NOT NULL,
    notice_content TEXT,
    status         SMALLINT    NOT NULL DEFAULT 1,
    creator_id     BIGINT      REFERENCES user_info (id),
    remark         VARCHAR(255)         DEFAULT '',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT ck_sys_notice_type CHECK (notice_type IN (1, 2)),
    CONSTRAINT ck_sys_notice_status CHECK (status IN (0, 1))
);

COMMENT ON TABLE sys_notice IS '通知公告表';
COMMENT ON COLUMN sys_notice.notice_title IS '公告标题';
COMMENT ON COLUMN sys_notice.notice_type IS '公告类型：1通知，2公告';
COMMENT ON COLUMN sys_notice.status IS '公告状态：1正常，0关闭';

CREATE INDEX IF NOT EXISTS idx_sys_notice_type ON sys_notice (notice_type);
CREATE INDEX IF NOT EXISTS idx_sys_notice_status ON sys_notice (status);
CREATE INDEX IF NOT EXISTS idx_sys_notice_created_at ON sys_notice (created_at DESC);

DO
$$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'update_updated_at') THEN
        IF EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'sys_notice_updated_at_trg') THEN
            DROP TRIGGER sys_notice_updated_at_trg ON sys_notice;
        END IF;
        CREATE TRIGGER sys_notice_updated_at_trg
            BEFORE UPDATE ON sys_notice
            FOR EACH ROW
        EXECUTE FUNCTION update_updated_at();
    END IF;
END
$$;

INSERT INTO sys_notice (
    notice_title, notice_type, notice_content, status, creator_id, remark
)
VALUES ('温馨提醒：2018-07-01 Keystone新版本发布啦',
        2, '新版本内容~~~~~~~~~~', 1, (SELECT id FROM user_info WHERE username = 'admin'), '管理员'),
       ('维护通知：2018-07-01 Keystone系统凌晨维护',
        1, '维护内容', 1, (SELECT id FROM user_info WHERE username = 'admin'), '管理员')
ON CONFLICT DO NOTHING;

INSERT INTO permissions (name, description)
VALUES ('system:notice:list', '查询通知公告列表'),
       ('system:notice:query', '查询通知公告详情'),
       ('system:notice:add', '新增通知公告'),
       ('system:notice:edit', '修改通知公告'),
       ('system:notice:remove', '删除通知公告')
ON CONFLICT (name) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
         JOIN permissions p ON p.name IN (
             'system:notice:list',
             'system:notice:query',
             'system:notice:add',
             'system:notice:edit',
             'system:notice:remove'
         )
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;
