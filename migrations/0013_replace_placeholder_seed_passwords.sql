-- Replace early placeholder seed passwords with bcrypt hashes that can be used
-- by the AgileBoot login page. Existing user-changed passwords are untouched.

UPDATE user_info
SET password = CASE username
                   WHEN 'admin' THEN '$2b$12$eAPqBRlds7cMT26pyiJvvOJWHj.PZ6SqFvWq3n2wbsQgT2uxPb5fy'
                   WHEN 'editor' THEN '$2b$12$qDkIlI7dBVhz7VDqFCiyaOfPR7k5lbHTIiY4Cs8CKsLEmNRALPfbS'
                   WHEN 'user' THEN '$2b$12$8Zz4BxpTMvfvthPMhFqxp.1r.QyUb9wFjy1SqMnbJ4YOfDecSkRqG'
                   ELSE password
               END
WHERE (username = 'admin' AND password = 'hashed_password_admin')
   OR (username = 'editor' AND password = 'hashed_password_editor')
   OR (username = 'user' AND password = 'hashed_password_user');
