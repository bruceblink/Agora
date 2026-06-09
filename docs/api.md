# API 文档

> 基础路径：`/`  
> 所有需要认证的接口须在 Cookie 中携带 `access_token`（JWT）。  
> Keystone 兼容管理接口同时支持原始根路径与 `/api` 前缀路径，例如 `/system/users` 与 `/api/system/users` 等价；本文示例保留 `/api` 形式，便于兼容现有 Agora 客户端。
> 统一响应格式见 [响应结构](#响应结构) 章节。

---

## 目录

- [响应结构](#响应结构)
- [认证 / 鉴权](#认证--鉴权)
  - [首页重定向](#get-root)
  - [健康检查](#get-health)
  - [登录](#post-login)
  - [Keylo Token 登录兼容入口](#post-loginkeylo)
  - [验证码](#get-captchaimage)
  - [登录 RSA 公钥](#get-loginrsa-public-key)
  - [刷新 Keystone Token](#post-refresh-token)
  - [通过 Refresh Token 登出](#post-logout-refresh-token)
  - [当前登录用户](#get-getloginuserinfo)
  - [动态路由](#get-getrouters)
  - [注册](#post-register)
  - [登出](#post-logout)
  - [刷新 Token](#post-authtokenrefresh)
  - [GitHub OAuth 登录](#get-authoauthgithublogin)
  - [GitHub OAuth 回调](#get-authoauthgithubcallback)
- [用户信息](#用户信息)
  - [获取当前用户信息](#get-apime)
  - [获取用户配置](#get-apisyncme)
  - [保存用户配置](#post-apisyncme)
- [通用文件](#通用文件)
  - [下载文件](#get-apifiledownload)
  - [单文件上传](#post-apifileupload)
  - [多文件上传](#post-apifileuploads)
- [系统配置 / 字典](#系统配置--字典)
  - [获取登录配置与字典](#get-getconfig)
  - [分页查询系统配置](#get-apisystemconfigs)
  - [系统配置详情](#get-apisystemconfigconfigid)
  - [更新系统配置](#put-apisystemconfigconfigid)
  - [刷新系统配置缓存](#delete-apisystemconfigscache)
  - [分页查询通知公告](#get-apisystemnotices)
  - [从库查询通知公告](#get-apisystemnoticesdatabaseslave)
  - [通知公告详情](#get-apisystemnoticesnoticeid)
  - [新增通知公告](#post-apisystemnotices)
  - [更新通知公告](#put-apisystemnoticesnoticeid)
  - [删除通知公告](#delete-apisystemnotices)
  - [查询菜单列表](#get-apisystemmenus)
  - [菜单详情](#get-apisystemmenusmenuid)
  - [菜单下拉树](#get-apisystemmenusdropdown)
  - [新增菜单](#post-apisystemmenus)
  - [更新菜单](#put-apisystemmenusmenuid)
  - [删除菜单](#delete-apisystemmenusmenuid)
  - [分页查询角色](#get-apisystemrolelist)
  - [导出角色列表](#post-apisystemroleexport)
  - [角色详情](#get-apisystemroleroleid)
  - [新增角色](#post-apisystemrole)
  - [更新角色](#put-apisystemrole)
  - [更新角色状态](#put-apisystemroleroleidstatus)
  - [更新角色数据范围](#put-apisystemroleroleiddatascope)
  - [删除角色](#delete-apisystemroleroleids)
  - [已分配角色用户](#get-apisystemroleroleidallocatedlist)
  - [未分配角色用户](#get-apisystemroleroleidunallocatedlist)
  - [批量授权角色用户](#post-apisystemroleroleidusersuseridsgrantbulk)
  - [批量取消角色授权](#delete-apisystemroleusersuseridsgrantbulk)
  - [查询部门列表](#get-apisystemdepts)
  - [部门下拉树](#get-apisystemdeptsdropdown)
  - [部门详情](#get-apisystemdeptdeptid)
  - [新增部门](#post-apisystemdept)
  - [更新部门](#put-apisystemdeptdeptid)
  - [删除部门](#delete-apisystemdeptdeptid)
  - [分页查询岗位](#get-apisystempostlist)
  - [导出岗位列表](#get-apisystempostexcel)
  - [岗位详情](#get-apisystempostpostid)
  - [新增岗位](#post-apisystempost)
  - [更新岗位](#put-apisystempost)
  - [删除岗位](#delete-apisystempost)
  - [分页查询系统用户](#get-apisystemusers)
  - [导出系统用户列表](#get-apisystemusersexcel)
  - [下载系统用户导入模板](#get-apisystemusersexceltemplate)
  - [导入系统用户](#post-apisystemusersexcel)
  - [系统用户详情](#get-apisystemusersuserid)
  - [新增系统用户](#post-apisystemusers)
  - [更新系统用户](#put-apisystemusersuserid)
  - [重置系统用户密码](#put-apisystemusersuseridpassword)
  - [更新系统用户状态](#put-apisystemusersuseridstatus)
  - [删除系统用户](#delete-apisystemusersuserids)
  - [分页查询字典类型](#get-apisystemdicttypes)
  - [字典类型详情](#get-apisystemdicttypedictid)
  - [新增字典类型](#post-apisystemdicttype)
  - [更新字典类型](#put-apisystemdicttypedictid)
  - [删除字典类型](#delete-apisystemdicttypedictid)
  - [分页查询字典数据](#get-apisystemdictdatalist)
  - [按类型查询字典数据](#get-apisystemdictdatatypedicttype)
  - [字典数据详情](#get-apisystemdictdatadictcode)
  - [新增字典数据](#post-apisystemdictdata)
  - [更新字典数据](#put-apisystemdictdatadictcode)
  - [删除字典数据](#delete-apisystemdictdatadictcode)
  - [批量删除系统定时任务](#delete-apisystemjobs)
- [日志管理](#日志管理)
  - [分页查询登录日志](#get-apilogsloginlogs)
  - [导出登录日志](#get-apilogsloginlogsexcel)
  - [删除登录日志](#delete-apilogsloginlogs)
  - [分页查询操作日志](#get-apilogsoperationlogs)
  - [新增操作日志](#post-apilogsoperationlogs)
  - [导出操作日志](#get-apilogsoperationlogsexcel)
  - [删除操作日志](#delete-apilogsoperationlogs)
- [系统监控](#系统监控)
  - [缓存监控信息](#get-apimonitorcacheinfo)
  - [服务器监控信息](#get-apimonitorserverinfo)
  - [在线用户列表](#get-apimonitoronlineusers)
  - [强退在线用户](#delete-apimonitoronlineusertokenid)
- [番剧信息](#番剧信息)
  - [分页查询番剧列表](#get-apianis)
  - [查询单条番剧](#get-apianisid)
- [番剧收藏](#番剧收藏)
  - [分页查询收藏列表](#get-apianiscollect)
  - [添加收藏](#post-apianiscollect)
  - [取消收藏](#delete-apianiscollectid)
  - [标记观看状态](#patch-apianiscollectidwatched)
- [新闻信息](#新闻信息)
  - [分页查询新闻列表](#get-apinews)
  - [分页查询新闻条目](#get-apinewsitems)
  - [分页查询热点事件](#get-apinewsevents)
  - [查询事件下的新闻条目](#get-apinewseventsidItems)
- [定时任务](#定时任务)
  - [分页查询定时任务](#get-apischeduledtasks)
  - [创建定时任务](#post-apischeduledtasks)
  - [更新定时任务](#put-apischeduledtasksid)
  - [切换启停状态](#patch-apischeduledtasksidstatus)
  - [删除定时任务](#delete-apischeduledtasksid)
  - [同步任务数据源](#post-apisynctask_source)
- [管理接口](#管理接口)
  - [重载任务调度器](#post-admintaskreload)
- [图片代理](#图片代理)
  - [代理图片请求](#get-apiproxyimage)

---

## 响应结构

### 成功响应

```json
{
  "code": 0,
  "msg": "操作成功",
  "status": "ok",
  "data": { }
}
```

### 失败响应

```json
{
  "code": 1,
  "msg": "错误描述",
  "status": "error",
  "message": "错误描述"
}
```

`code` / `msg` 用于兼容 Keystone / AgileBoot 前端；`status` / `message` 保留给既有 Agora 客户端。

### 分页数据结构 `PageData<T>`

```json
{
  "status": "ok",
  "data": {
    "items": [],
    "rows": [],
    "totalCount": 100,
    "total": 100,
    "page": 1,
    "pageSize": 20,
    "totalPages": 5
  }
}
```

`items` / `totalCount` 是 Agora 原生字段；`rows` / `total` 是 Keystone 前端兼容字段，内容保持一致。

### 分页查询参数（通用）

所有分页接口均支持以下 Query 参数（`camelCase`）：

| 参数       | 类型     | 说明                 | 默认值 |
|----------|--------|--------------------| ---- |
| `page`   | number | 页码，从 1 开始         | 1    |
| `pageSize` | number | 每页条数             | 20   |
| `filter` | object | 过滤条件（各接口不同，见下文） | —    |

> `filter` 字段以 flat 形式展开在 Query 字符串中，例如：  
> `?page=1&pageSize=20&title=进击的巨人&platform=bilibili`

---

## 认证 / 鉴权

<a id="get-root"></a>

### GET `/`

Keystone 兼容首页入口，直接重定向到前端页面。重定向地址优先读取 `KEYSTONE_FRONTEND_URL`，其次兼容 `FRONTEND_URL`，未配置时使用 Keystone 默认值 `http://localhost:80`。

**无需认证**

**响应** `302 Found`

| Header | 说明 |
| --- | --- |
| `Location` | 前端访问地址 |

---

### GET `/health`

Keystone 兼容健康检查。

**无需认证**

**响应** `200 OK`

```json
{
  "code": 0,
  "msg": "操作成功",
  "status": "ok",
  "data": "is alive"
}
```

---

### POST `/login`

本地账号登录。兼容 AgileBoot：密码可为 `/login/rsa-public-key` 返回公钥加密后的 RSA 密文，也兼容明文密码。

**无需认证**

开发种子账号：`admin/admin123`、`editor/editor123`、`user/user1234`。

**请求体** `application/json`

```json
{
  "username": "admin",
  "password": "rsa-or-plain-password",
  "captchaCode": "",
  "captchaCodeKey": "",
  "forceLogin": false
}
```

**响应** `200 OK`，同时写入 `access_token` / `refresh_token` Cookie。

```json
{
  "code": 0,
  "msg": "操作成功",
  "status": "ok",
  "data": {
    "token": "jwt",
    "refreshToken": "refresh-token",
    "expiresIn": 7200,
    "refreshExpiresIn": 2592000,
    "currentUser": {
      "roleKey": "admin",
      "permissions": ["system:user:list"],
      "userInfo": {
        "userId": 1,
        "username": "admin"
      }
    }
  }
}
```

---

### POST `/login/keylo`

Keystone 历史 Keylo token 登录入口。Agora 当前未配置 Keylo token verifier，该兼容端点会返回 `400 Bad Request` 并提示使用 `/login`。

**无需认证**

**请求体** `application/json`

```json
{
  "accessToken": "keylo-access-token",
  "forceLogin": false
}
```

---

### GET `/captchaImage`

返回登录验证码信息。当前 Agora 默认关闭验证码，保留该接口用于前端启动兼容。

**无需认证**

```json
{
  "code": 0,
  "msg": "操作成功",
  "data": {
    "isCaptchaOn": false,
    "captchaCodeKey": "",
    "captchaCodeImg": ""
  }
}
```

---

### GET `/login/rsa-public-key`

获取登录 RSA 公钥。返回值为 X.509 SubjectPublicKeyInfo DER 的 base64 字符串，供前端 `JSEncrypt` 加密密码。

**无需认证**

```json
{
  "code": 0,
  "msg": "操作成功",
  "data": {
    "publicKey": "MIIBIjANBgkq..."
  }
}
```

---

### POST `/refresh-token`

使用请求体或 Cookie 中的 refresh token 换取新的 Keystone 兼容 token 响应。

**无需认证**

```json
{
  "refreshToken": "refresh-token"
}
```

**响应** `200 OK`，`data` 为 `TokenDTO`。刷新响应不包含 `currentUser`，前端会复用已缓存用户信息。

---

### POST `/logout-refresh-token`

在 access token 已失效时，通过 refresh token 主动释放后端会话。

**无需认证**

```json
{
  "refreshToken": "refresh-token"
}
```

**响应** `200 OK`

---

### GET `/getLoginUserInfo`

获取当前登录用户信息。

**需要认证**

**请求头** `Authorization: Bearer <token>`

**响应** `200 OK`，`data` 为 `CurrentLoginUserDTO`。

---

### GET `/getRouters`

获取当前用户可访问的动态路由树。按钮权限不会作为路由节点返回，会写入对应页面的 `meta.auths`。

**需要认证**

**请求头** `Authorization: Bearer <token>`

**响应** `200 OK`，`data` 为 `RouterDTO[]`。

---

### POST `/register`

Keystone 兼容注册入口。Keystone 当前保留该路由但返回“不支持的操作”，Agora 同步该行为，不创建本地账号、不写入 Token Cookie。

**无需认证**

**响应** `200 OK`

```json
{
  "code": 10002,
  "msg": "不支持的操作",
  "status": "error",
  "message": "不支持的操作"
}
```

---

### POST `/logout`

登出，使当前 refresh token 失效，并清空 Cookie。

**无需认证**（依赖 Cookie 中的 `refresh_token`）

**请求体** 无

**响应** `200 OK`，同时清除 `access_token` / `refresh_token` Cookie。

---

### POST `/auth/token/refresh`

使用 Cookie 中的 `refresh_token` 换取新的 `access_token`。

**无需认证**

**请求体** 无（refresh token 从 Cookie 读取）

**响应** `200 OK`，通过 Set-Cookie 写入新的 `access_token`。

---

### GET `/auth/oauth/github/login`

发起 GitHub OAuth2 授权流程，重定向到 GitHub 授权页。

**无需认证**（需要在服务端配置 GitHub OAuth 应用）

**Query 参数**

| 参数            | 类型     | 必填 | 说明              |
|---------------|--------|----|-----------------|
| `redirect_uri` | string | ✓  | 授权成功后的回调前端地址   |

**响应** `302 Redirect` → GitHub 授权页

---

### GET `/auth/oauth/github/callback`

GitHub OAuth2 授权回调，由 GitHub 重定向至此。

**无需认证**

**Query 参数**

| 参数      | 类型     | 必填 | 说明                |
|---------|--------|----|-------------------|
| `code`  | string | ✓  | GitHub 返回的授权码    |
| `state` | string | ✓  | CSRF 防护 state 参数 |

**响应** `302 Redirect` → 前端回调地址，同时通过 Set-Cookie 写入 `access_token` / `refresh_token`。

---

## 用户信息

> 以下接口需认证（Cookie 携带 `access_token`）。

---

### GET `/api/me`

获取当前登录用户的基本信息。

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": {
    "id": 1,
    "username": "alice",
    "email": "alice@example.com",
    "avatar": "https://avatars.githubusercontent.com/...",
    "roles": ["user"],
    "permissions": []
  }
}
```

---

### GET `/api/sync/me`

查询当前用户的指定类型配置。

**Query 参数**

| 参数            | 类型     | 必填 | 说明                         |
|---------------|--------|----|----------------------------|
| `settingType` | string | ✓  | 配置类型，例如 `"theme"` `"layout"` |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": {
    "data": { },
    "updatedTime": 1700000000
  }
}
```

> `data` 为任意 JSON 对象，`updatedTime` 为 Unix 时间戳（秒）。

---

### POST `/api/sync/me`

保存当前用户的指定类型配置（upsert）。

**请求体** `application/json`

```json
{
  "settingType": "theme",
  "data": {
    "colorScheme": "dark"
  }
}
```

| 字段            | 类型     | 必填 | 说明         |
|---------------|--------|----|------------|
| `settingType` | string | ✓  | 配置类型       |
| `data`        | object | ✓  | 任意 JSON 配置 |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": "数据同步成功"
}
```

---

## 个人资料

> 以下接口需认证（Cookie 携带 `access_token`）。对应 Keystone `/system/user/profile` 模块，同时支持 `/system/user/profile` 与 `/api/system/user/profile`。

### GET `/api/system/user/profile`

获取当前登录用户的个人资料、角色名称和岗位名称。

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": {
    "user": {
      "userId": 1,
      "username": "admin",
      "nickname": "系统管理员",
      "email": "admin@example.com",
      "phoneNumber": "15888888888",
      "sex": 2,
      "avatar": "/uploads/avatar/user-1.png",
      "roleName": "超级管理员",
      "postName": "董事长"
    },
    "roleName": "超级管理员",
    "postName": "董事长"
  }
}
```

---

### PUT `/api/system/user/profile`

修改当前登录用户个人资料。省略字段会保留原值，传空字符串会清空可选字段。

```json
{
  "nickName": "管理员",
  "phoneNumber": "15888888888",
  "email": "admin@example.com",
  "sex": 2
}
```

---

### PUT `/api/system/user/profile/password`

修改当前登录用户密码，会校验旧密码并递增 `token_version` 使旧 access token 失效。

```json
{
  "oldPassword": "old-password",
  "newPassword": "new-password"
}
```

---

### POST `/api/system/user/profile/avatar`

上传当前登录用户头像。请求体为 `multipart/form-data`，文件字段名与 Keystone 一致为 `avatarfile`；支持 `jpeg` / `png` / `gif` / `webp`，最大 5MB。

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": {
    "imgUrl": "/uploads/avatar/user-1-1710000000000-123.png"
  }
}
```

---

## 通用文件

> 以下接口需认证。对应 Keystone `/file` 模块，同时支持 `/file/*` 与 `/api/file/*`。上传后的静态资源通过 `/profile/**` 访问。

### GET `/api/file/download`

下载 `uploads/profile/download` 下的文件。`fileName` 不允许目录穿越，仅允许 Keystone 白名单扩展名。

**查询参数**

| 字段       | 类型     | 必填 | 说明   |
|----------|--------|----|------|
| fileName | string | ✓  | 文件名  |

**响应** `200 OK`，返回 `application/octet-stream` 文件流。

非法 `fileName` 与 Keystone 一致仍返回 `200 OK`，body 为业务失败响应：

```json
{
  "code": 10004,
  "msg": "文件名称(../readme.txt)非法，不允许下载",
  "status": "error",
  "message": "文件名称(../readme.txt)非法，不允许下载"
}
```

---

### POST `/api/file/upload`

通用单文件上传。请求体为 `multipart/form-data`，文件字段名为 `file`。允许图片、Office、文本、压缩包、视频和 PDF 等 Keystone 白名单扩展名，最大 50MB。

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": {
    "url": "http://localhost:8000/profile/upload/20260609093000_report_abc.xlsx",
    "fileName": "/profile/upload/20260609093000_report_abc.xlsx",
    "newFileName": "20260609093000_report_abc.xlsx",
    "originalFilename": "report.xlsx"
  }
}
```

---

### POST `/api/file/uploads`

通用多文件上传。请求体为 `multipart/form-data`，文件字段名为 `file` 或 `files`，响应 `data` 为 `UploadDTO[]`。

---

## 系统配置 / 字典

### GET `/getConfig`

获取登录页配置与 Keystone 兼容字典聚合。

**无需认证**

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": {
    "isCaptchaOn": false,
    "dictionary": {
      "common.yesOrNo": [
        { "label": "是", "value": 1, "cssTag": "" },
        { "label": "否", "value": 0, "cssTag": "danger" }
      ]
    }
  }
}
```

---

### GET `/api/system/configs`

分页查询系统参数配置。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 | 默认值 |
| --- | --- | --- | --- |
| `page` / `pageNum` | number | 页码，从 1 开始 | 1 |
| `pageSize` | number | 每页条数 | 20 |
| `configName` | string | 配置名称，模糊匹配 | — |
| `configKey` | string | 配置键名，精确匹配 | — |
| `isAllowChange` | boolean | 是否允许修改 | — |

**响应** `200 OK` → `PageData<SystemConfigDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "configId": "4",
        "configName": "账号自助-验证码开关",
        "configKey": "sys.account.captchaOnOff",
        "configValue": "false",
        "configOptions": ["true", "false"],
        "isAllowChange": 0,
        "isAllowChangeStr": "否",
        "remark": "是否开启验证码功能（true开启，false关闭）",
        "createTime": "2026-06-08T13:00:00Z"
      }
    ],
    "totalCount": 5,
    "page": 1,
    "pageSize": 20,
    "totalPages": 1
  }
}
```

---

### GET `/api/system/config/{configId}`

系统配置详情。

**需要认证，仅管理员**

---

### PUT `/api/system/config/{configId}`

更新系统配置值。`configValue` 不能为空；若该配置存在 `configOptions`，值必须在候选项内。

**需要认证，仅管理员**

```json
{
  "configValue": "true"
}
```

---

### DELETE `/api/system/configs/cache`

刷新系统配置缓存。Agora 当前未启用本地配置缓存，该接口保留为 Keystone 兼容 no-op。

**需要认证，仅管理员**

---

### GET `/api/system/notices`

分页查询通知公告。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 | 默认值 |
| --- | --- | --- | --- |
| `page` | number | 页码，从 1 开始 | 1 |
| `pageSize` | number | 每页条数 | 20 |
| `noticeTitle` | string | 公告标题，模糊匹配 | — |
| `noticeType` | string | 公告类型：`1`=通知 / `2`=公告 | — |
| `creatorName` | string | 创建人用户名，模糊匹配 | — |

**响应** `200 OK` → `PageData<NoticeDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "noticeId": "1",
        "noticeTitle": "维护通知：2018-07-01 Keystone系统凌晨维护",
        "noticeType": 1,
        "noticeContent": "维护内容",
        "status": 1,
        "createTime": "2026-06-08T13:00:00Z",
        "creatorName": "admin"
      }
    ],
    "totalCount": 2,
    "page": 1,
    "pageSize": 20,
    "totalPages": 1
  }
}
```

---

### GET `/api/system/notices/database/slave`

Keystone 主从库示例接口的兼容别名。Query 参数和响应结构同 `/api/system/notices`；Agora 当前使用同一数据库连接池返回列表。

**需要认证，仅管理员**

---

### GET `/api/system/notices/{noticeId}`

通知公告详情。

**需要认证，仅管理员**

---

### POST `/api/system/notices`

新增通知公告。

**需要认证，仅管理员**

```json
{
  "noticeTitle": "维护通知",
  "noticeType": "1",
  "noticeContent": "系统将在凌晨维护",
  "status": "1"
}
```

---

### PUT `/api/system/notices/{noticeId}`

更新通知公告。

**需要认证，仅管理员**

---

### DELETE `/api/system/notices`

批量删除通知公告。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `noticeIds` | string | ✓ | 逗号分隔 ID，例如 `?noticeIds=1,2` |

---

### GET `/api/system/menus`

查询 Keystone 兼容菜单列表，包含用户、角色、菜单、部门、岗位、参数、公告、日志和定时任务等系统管理菜单。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `isButton` | boolean | 是否只查询按钮权限 |

**响应** `200 OK` → `Vec<MenuDTO>`

```json
{
  "status": "ok",
  "data": [
    {
      "id": 1,
      "parentId": 0,
      "menuName": "系统管理",
      "routerName": "",
      "path": "/system",
      "rank": 1,
      "menuType": 2,
      "menuTypeStr": "目录",
      "isButton": false,
      "status": 1,
      "statusStr": "正常",
      "createTime": "2022-05-21T08:30:54Z",
      "icon": "ep:management"
    }
  ]
}
```

---

### GET `/api/system/menus/{menuId}`

菜单详情，包含 `permission` 与完整 `meta`。

**需要认证，仅管理员**

---

### GET `/api/system/menus/dropdown`

菜单下拉树，用于新增/编辑菜单时选择父级。

**需要认证，仅管理员**

---

### POST `/api/system/menus`

新增菜单。

**需要认证，仅管理员**

```json
{
  "parentId": 1,
  "menuName": "示例菜单",
  "routerName": "Example",
  "path": "/system/example/index",
  "status": 1,
  "menuType": 1,
  "isButton": false,
  "permission": "system:example:list",
  "meta": {
    "title": "示例菜单",
    "icon": "ep:menu",
    "showParent": true
  }
}
```

---

### PUT `/api/system/menus/{menuId}`

更新菜单。与 Keystone 一致，非按钮菜单不允许修改菜单类型；父级不能选择自身。

**需要认证，仅管理员**

---

### DELETE `/api/system/menus/{menuId}`

删除菜单。存在子菜单或已分配给角色时会返回请求错误。

**需要认证，仅管理员**

---

### GET `/api/system/role/list`

分页查询 Keystone 兼容角色列表。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 | 默认值 |
| --- | --- | --- | --- |
| `page` | number | 页码，从 1 开始 | 1 |
| `pageSize` | number | 每页条数 | 20 |
| `roleName` | string | 角色名称，模糊匹配 | — |
| `roleKey` | string | 角色标识，精确匹配 | — |
| `status` | string | `1`=正常 / `0`=停用 | — |

**响应** `200 OK` → `PageData<RoleDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "roleId": 1,
        "roleName": "超级管理员",
        "roleKey": "admin",
        "roleSort": 1,
        "status": 1,
        "remark": "超级管理员",
        "createTime": "2022-05-21T08:30:54Z",
        "dataScope": 1,
        "selectedMenuList": [],
        "selectedDeptList": []
      }
    ],
    "totalCount": 5,
    "page": 1,
    "pageSize": 20,
    "totalPages": 1
  }
}
```

---

### POST `/api/system/role/export`

导出 Keystone 兼容角色列表 xlsx 文件。Query 参数同分页查询角色接口，当前单次导出最多 500 条。

**需要认证，仅管理员**

**响应** `200 OK` → `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`

---

### GET `/api/system/role/{roleId}`

角色详情，包含 `selectedMenuList` 和 `selectedDeptList`。

**需要认证，仅管理员**

---

### POST `/api/system/role`

新增角色，同时保存菜单授权并同步到当前 RBAC `role_permissions`。

**需要认证，仅管理员**

```json
{
  "roleName": "审计员",
  "roleKey": "auditor",
  "roleSort": 10,
  "remark": "只读审计角色",
  "dataScope": "1",
  "status": "1",
  "menuIds": [1, 7, 32]
}
```

---

### PUT `/api/system/role`

更新角色基础信息和菜单授权。

**需要认证，仅管理员**

```json
{
  "roleId": 4,
  "roleName": "审计员",
  "roleKey": "auditor",
  "roleSort": 10,
  "remark": "只读审计角色",
  "dataScope": "1",
  "status": "1",
  "menuIds": [1, 7, 32]
}
```

---

### PUT `/api/system/role/{roleId}/status`

更新角色状态。

**需要认证，仅管理员**

```json
{
  "status": 0
}
```

---

### PUT `/api/system/role/{roleId}/dataScope`

更新角色数据范围，`deptIds` 保存为 Keystone 兼容的部门 ID 集合。

**需要认证，仅管理员**

```json
{
  "dataScope": 2,
  "deptIds": [100, 200]
}
```

---

### DELETE `/api/system/role/{roleIds}`

删除一个或多个角色；已分配给用户的角色不能删除。多个 ID 使用逗号分隔，例如 `/api/system/role/4,5`。

**需要认证，仅管理员**

---

### GET `/api/system/role/{roleId}/allocated/list`

分页查询已关联该角色的用户列表。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 | 默认值 |
| --- | --- | --- | --- |
| `page` / `pageNum` | number | 页码，从 1 开始 | 1 |
| `pageSize` | number | 每页条数 | 20 |
| `username` | string | 用户名，模糊匹配 | — |
| `phoneNumber` | string | 手机号，模糊匹配 | — |

**响应** `200 OK` → `PageData<SystemUserDTO>`

---

### GET `/api/system/role/{roleId}/unallocated/list`

分页查询未关联该角色的用户列表。

**需要认证，仅管理员**

**Query 参数**同已分配用户列表。

**响应** `200 OK` → `PageData<SystemUserDTO>`

---

### POST `/api/system/role/{roleId}/users/{userIds}/grant/bulk`

批量为用户授予角色。多个用户 ID 使用逗号分隔，例如 `/api/system/role/2/users/3,4/grant/bulk`。

**需要认证，仅管理员**

---

### DELETE `/api/system/role/users/{userIds}/grant/bulk`

批量解除用户与角色的关联。Keystone 该接口不携带 `roleId`，Agora 按同等语义删除指定用户的全部角色关联。多个用户 ID 使用逗号分隔。

**需要认证，仅管理员**

---

### GET `/api/system/depts`

查询 Keystone 兼容部门列表。返回一维数组，前端可按 `id` / `parentId` 组装树。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `deptId` | number | 部门 ID |
| `parentId` | number | 上级部门 ID |
| `deptName` | string | 部门名称，模糊匹配 |
| `status` | number | `1`=正常 / `0`=停用 |

---

### GET `/api/system/depts/dropdown`

部门下拉树数据，返回 Keystone 兼容 `Tree<Long>` 结构：`id`、`parentId`、`label`、`children`。

**需要认证，仅管理员**

---

### GET `/api/system/dept/{deptId}`

部门详情。

**需要认证，仅管理员**

---

### POST `/api/system/dept`

新增部门。

**需要认证，仅管理员**

```json
{
  "parentId": 1,
  "deptName": "研发平台",
  "orderNum": 10,
  "leaderName": "alice",
  "phone": "15800000000",
  "email": "alice@example.com",
  "status": 1
}
```

---

### PUT `/api/system/dept/{deptId}`

更新部门。父级不能选择自身，存在同级同名部门时返回请求错误。

**需要认证，仅管理员**

---

### DELETE `/api/system/dept/{deptId}`

删除部门。存在子部门或已分配用户时不允许删除。

**需要认证，仅管理员**

---

### GET `/api/system/post/list`

分页查询 Keystone 兼容岗位列表。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 | 默认值 |
| --- | --- | --- | --- |
| `page` / `pageNum` | number | 页码，从 1 开始 | 1 |
| `pageSize` | number | 每页条数 | 10 |
| `postCode` | string | 岗位编码，精确匹配 | — |
| `postName` | string | 岗位名称，模糊匹配 | — |
| `status` | number | `1`=正常 / `0`=停用 | — |
| `orderColumn` | string | `postSort` / `createTime` 等 | `postSort` |
| `orderDirection` | string | `ascending` / `descending` | `ascending` |

**响应** `200 OK` → `PageData<PostDTO>`

---

### GET `/api/system/post/excel`

导出 Keystone 兼容岗位列表 xlsx 文件。Query 参数同分页查询岗位接口，当前单次导出最多 500 条。

**需要认证，仅管理员**

**响应** `200 OK` → `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`

---

### GET `/api/system/post/{postId}`

岗位详情。

**需要认证，仅管理员**

---

### POST `/api/system/post`

新增岗位。

**需要认证，仅管理员**

```json
{
  "postCode": "rd",
  "postName": "研发工程师",
  "postSort": 10,
  "status": "1",
  "remark": "研发岗位"
}
```

---

### PUT `/api/system/post`

更新岗位。

**需要认证，仅管理员**

```json
{
  "postId": 5,
  "postCode": "rd",
  "postName": "研发工程师",
  "postSort": 10,
  "status": "1",
  "remark": "研发岗位"
}
```

---

### DELETE `/api/system/post`

删除一个或多个岗位，Query 使用 `ids=1,2`。已分配给用户的岗位不能删除。

**需要认证，仅管理员**

---

### GET `/api/system/users`

分页查询 Keystone 兼容系统用户列表。Agora 仍使用 `user_info` 作为登录主体表，并扩展部门、岗位、用户状态等管理字段。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 | 默认值 |
| --- | --- | --- | --- |
| `page` / `pageNum` | number | 页码，从 1 开始 | 1 |
| `pageSize` | number | 每页条数 | 10 |
| `userId` | number | 用户 ID | — |
| `username` | string | 用户名，模糊匹配 | — |
| `phoneNumber` | string | 手机号，模糊匹配 | — |
| `deptId` | number | 部门 ID，包含子部门 | — |
| `status` | number | `1`=正常 / `0`=停用 | — |

**响应** `200 OK` → `PageData<SystemUserDTO>`

---

### GET `/api/system/users/excel`

导出 Keystone 兼容系统用户列表 xlsx 文件。Query 参数同分页查询系统用户接口，当前单次导出最多 500 条。

**需要认证，仅管理员**

**响应** `200 OK` → `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`

---

### GET `/api/system/users/excelTemplate`

下载系统用户批量导入 xlsx 模板，表头与 Keystone `AddUserCommand` 导入列保持一致。

**需要认证，仅管理员**

**响应** `200 OK` → `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`

---

### POST `/api/system/users/excel`

用户 Excel 导入入口。使用 `multipart/form-data` 上传字段名 `file`，表头需与下载模板一致；导入时逐行创建系统用户。

**需要认证，仅管理员**

**响应** `200 OK` → `ResponseDTO<Void>`

---

### GET `/api/system/users/{userId}`

系统用户详情，返回 `user`、`roleOptions`、`postOptions`、`roleId`、`postId` 和权限列表。

**需要认证，仅管理员**

---

### POST `/api/system/users`

新增系统用户，并同步单角色关系到 Agora 的 `user_roles`。

**需要认证，仅管理员**

```json
{
  "deptId": 4,
  "username": "alice",
  "nickname": "Alice",
  "email": "alice@example.com",
  "phoneNumber": "15800000000",
  "sex": 2,
  "password": "password123",
  "status": 1,
  "roleId": 2,
  "postId": 4,
  "remark": ""
}
```

---

### PUT `/api/system/users/{userId}`

更新系统用户资料、部门、岗位和单角色关系。

**需要认证，仅管理员**

---

### PUT `/api/system/users/{userId}/password`

重置系统用户密码。密码会用 bcrypt 重新哈希，并递增 `tokenVersion` 使旧 JWT 失效。

**需要认证，仅管理员**

```json
{
  "userId": 2,
  "password": "newPassword123"
}
```

---

### PUT `/api/system/users/{userId}/status`

更新系统用户状态，并同步 Agora 登录状态：`1` 映射为 `active`，其他状态映射为不可登录。

**需要认证，仅管理员**

```json
{
  "status": 0
}
```

---

### DELETE `/api/system/users/{userIds}`

删除一个或多个用户，多个 ID 使用逗号分隔，例如 `/api/system/users/2,3`。当前登录用户和超级管理员不允许删除。

**需要认证，仅管理员**

---

### GET `/api/system/dict/types`

分页查询字典类型。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 | 默认值 |
| --- | --- | --- | --- |
| `page` | number | 页码，从 1 开始 | 1 |
| `pageSize` | number | 每页条数 | 20 |
| `dictName` | string | 字典名称，模糊匹配 | — |
| `dictType` | string | 字典类型，模糊匹配 | — |
| `status` | number | `1`=正常 / `0`=停用 | — |

**响应** `200 OK` → `PageData<DictTypeDTO>`

---

### GET `/api/system/dict/type/{dictId}`

字典类型详情。

**需要认证，仅管理员**

---

### POST `/api/system/dict/type`

新增字典类型。

**需要认证，仅管理员**

```json
{
  "dictName": "任务状态",
  "dictType": "sysJob.status",
  "status": 1,
  "remark": "任务状态列表"
}
```

---

### PUT `/api/system/dict/type/{dictId}`

更新字典类型。若 `dictType` 变更，会同步更新对应字典数据的 `dictType`。

**需要认证，仅管理员**

---

### DELETE `/api/system/dict/type/{dictId}`

删除字典类型。若该类型下仍存在字典数据，将返回请求错误。

**需要认证，仅管理员**

---

### GET `/api/system/dict/data/list`

分页查询字典数据。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 | 默认值 |
| --- | --- | --- | --- |
| `page` | number | 页码，从 1 开始 | 1 |
| `pageSize` | number | 每页条数 | 20 |
| `dictType` | string | 字典类型，精确匹配 | — |
| `dictLabel` | string | 字典标签，模糊匹配 | — |
| `status` | number | `1`=正常 / `0`=停用 | — |

**响应** `200 OK` → `PageData<DictDataDTO>`

---

### GET `/api/system/dict/data/type/{dictType}`

按字典类型查询字典数据，供前端下拉框使用。

**需要认证**

---

### GET `/api/system/dict/data/{dictCode}`

字典数据详情。

**需要认证，仅管理员**

---

### POST `/api/system/dict/data`

新增字典数据。

**需要认证，仅管理员**

```json
{
  "dictType": "sysJob.status",
  "dictLabel": "正常",
  "dictValue": "1",
  "dictSort": 1,
  "isDefault": 0,
  "cssClass": null,
  "listClass": "",
  "status": 1,
  "remark": "任务正常"
}
```

---

### PUT `/api/system/dict/data/{dictCode}`

更新字典数据。

**需要认证，仅管理员**

---

### DELETE `/api/system/dict/data/{dictCode}`

删除字典数据。

**需要认证，仅管理员**

---

### DELETE `/api/system/jobs`

批量删除 Keystone 兼容定时任务。删除成功后会刷新调度器配置。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `jobIds` | string | ✓ | 逗号分隔 ID，例如 `?jobIds=1,2` |

---

## 日志管理

### GET `/api/logs/loginLogs`

分页查询 Keystone 兼容登录日志。`pageNum` 与 `page` 都可作为页码参数，未传时默认第 1 页、每页 10 条。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 | 默认值 |
| --- | --- | --- | --- |
| `page` / `pageNum` | number | 页码，从 1 开始 | 1 |
| `pageSize` | number | 每页条数，最大 500 | 10 |
| `ipAddress` | string | 登录 IP，模糊匹配 | — |
| `status` | string | `1`=登录成功 / `2`=退出成功 / `3`=注册 / `0`=登录失败 | — |
| `username` | string | 用户名，模糊匹配 | — |
| `beginTime` | date | 起始日期，格式 `YYYY-MM-DD` | — |
| `endTime` | date | 截止日期，格式 `YYYY-MM-DD` | — |
| `orderColumn` | string | 排序字段，例如 `loginTime` | — |
| `orderDirection` | string | `ascending` / `descending` | `descending` |

**响应** `200 OK` → `PageData<LoginLogDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "logId": "415",
        "username": "admin",
        "ipAddress": "127.0.0.1",
        "loginLocation": "内网IP",
        "operationSystem": "Mac OS X",
        "browser": "Chrome",
        "status": 1,
        "statusStr": "登录成功",
        "msg": "登录成功",
        "loginTime": "2023-06-29T22:49:37Z"
      }
    ],
    "totalCount": 3,
    "page": 1,
    "pageSize": 10,
    "totalPages": 1
  }
}
```

本地 `/login` 成功/失败与 `/logout` 会写入该表；`status=3` 保留为 Keystone 注册日志枚举值，但 `/register` 当前返回“不支持的操作”。

---

### GET `/api/logs/loginLogs/excel`

导出 Keystone 兼容登录日志 xlsx 文件。Query 参数同分页查询登录日志接口，当前单次导出最多 500 条。

**需要认证，仅管理员**

**响应** `200 OK` → `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`

---

### DELETE `/api/logs/loginLogs`

软删除登录日志。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `ids` | string | ✓ | 逗号分隔 ID，例如 `?ids=415,416` |

---

### GET `/api/logs/operationLogs`

分页查询 Keystone 兼容操作日志。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 | 默认值 |
| --- | --- | --- | --- |
| `page` / `pageNum` | number | 页码，从 1 开始 | 1 |
| `pageSize` | number | 每页条数，最大 500 | 10 |
| `businessType` | string | 业务类型：`0`其它、`1`新增、`2`修改、`3`删除、`4`授权、`5`导出、`6`导入、`7`强退、`8`清空 | — |
| `status` | string | `1`=成功 / `0`=失败 | — |
| `username` | string | 操作用户名，模糊匹配 | — |
| `requestModule` | string | 请求模块，模糊匹配 | — |
| `beginTime` | date | 起始日期，格式 `YYYY-MM-DD` | — |
| `endTime` | date | 截止日期，格式 `YYYY-MM-DD` | — |
| `orderColumn` | string | 排序字段，例如 `operationTime` | — |
| `orderDirection` | string | `ascending` / `descending` | `descending` |

**响应** `200 OK` → `PageData<OperationLogDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "operationId": 561,
        "businessType": 1,
        "businessTypeStr": "添加",
        "requestMethod": "POST",
        "requestModule": "菜单管理",
        "requestUrl": "/system/menus",
        "calledMethod": "app.keystone.admin.controller.system.SysMenuController.add()",
        "operatorType": 1,
        "operatorTypeStr": "其他",
        "userId": 0,
        "username": "admin",
        "operatorIp": "127.0.0.1",
        "operatorLocation": "内网IP",
        "deptId": 0,
        "deptName": "",
        "operationParam": "{\"menuName\":\"\"}",
        "operationResult": "",
        "status": 1,
        "statusStr": "成功",
        "errorStack": "",
        "operationTime": "2023-07-22T17:06:57Z"
      }
    ],
    "totalCount": 1,
    "page": 1,
    "pageSize": 10,
    "totalPages": 1
  }
}
```

---

### POST `/api/logs/operationLogs`

客户端主动写入操作日志。未传 `requestMethod` / `requestUrl` / `operatorType` / `status` 时，会使用当前请求的 HTTP 方法、路径、Web 用户类型和成功状态作为默认值。`deptId` / `deptName` 是 Keystone 兼容回显字段，当前不接入部门模块。

**需要认证，仅管理员**

```json
{
  "businessType": 1,
  "requestModule": "菜单管理",
  "calledMethod": "frontend.audit.createMenu",
  "operationParam": "{\"menuName\":\"示例菜单\"}",
  "operationResult": "",
  "status": 1,
  "operationTime": "2026-06-09 10:11:12"
}
```

---

### GET `/api/logs/operationLogs/excel`

导出 Keystone 兼容操作日志 xlsx 文件。Query 参数同分页查询操作日志接口，当前单次导出最多 500 条。

**需要认证，仅管理员**

**响应** `200 OK` → `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`

---

### DELETE `/api/logs/operationLogs`

软删除操作日志。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `operationIds` | string | ✓ | 逗号分隔 ID，例如 `?operationIds=561,562` |

---

## 系统监控

> 对应 Keystone `/monitor` 模块，同时支持 `/monitor/*` 与 `/api/monitor/*`。

### GET `/api/monitor/cacheInfo`

返回 Keystone Redis 监控页兼容结构。Agora 当前没有 Redis 登录态，该接口用运行时与数据库会话统计填充 Redis 风格字段，保证前端缓存监控页可渲染。

**需要认证，仅管理员**

**响应** `200 OK` → `RedisCacheInfoDTO`

关键字段：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `info` | object | Redis 风格键值表，例如 `redis_version`、`used_memory_human` |
| `dbSize` | number | 当前有效 refresh token 会话数 |
| `commandStats` | array | ECharts 饼图数据，包含 `name` / `value` |

---

### GET `/api/monitor/serverInfo`

返回 Keystone 服务器监控页兼容结构，包含 `cpuInfo`、`memoryInfo`、`jvmInfo`、`systemInfo`、`diskInfos`。Rust 服务没有 JVM，`jvmInfo` 字段按前端契约映射为当前 Agora 进程运行时信息。

**需要认证，仅管理员**

---

### GET `/api/monitor/onlineUsers`

分页查询在线用户。Agora 使用未撤销且未过期的 `refresh_tokens` 作为在线会话来源；新登录会记录 IP、登录地点、浏览器和操作系统，历史会话会回退到最近一次成功登录日志。

**需要认证，仅管理员**

**Query 参数**

| 参数 | 类型 | 说明 | 默认值 |
| --- | --- | --- | --- |
| `page` / `pageNum` | number | 页码，从 1 开始 | 1 |
| `pageSize` | number | 每页条数，最大 500 | 10 |
| `ipAddress` | string | 登录 IP，模糊匹配 | — |
| `username` | string | 用户名，模糊匹配 | — |

**响应** `200 OK` → `PageData<OnlineUserDTO>`

---

### DELETE `/api/monitor/onlineUser/{tokenId}`

强制登出在线用户。`tokenId` 为在线用户列表返回的会话编号，Agora 会撤销匹配前缀的 refresh token。

**需要认证，仅管理员**

---

## 番剧信息

### GET `/api/anis`

分页查询番剧列表。

**Query 过滤字段**

| 参数         | 类型     | 说明              |
|------------|--------|-----------------|
| `title`    | string | 番剧标题（模糊匹配）     |
| `platform` | string | 平台名称（精确匹配）     |

**响应** `200 OK` → `PageData<AniInfoDto>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 1,
        "title": "进击的巨人",
        "updateCount": "第25话",
        "updateInfo": "已完结",
        "imageUrl": "https://...",
        "detailUrl": "https://...",
        "updateTime": "2024-01-01T00:00:00Z",
        "platform": "bilibili"
      }
    ],
    "totalCount": 100,
    "page": 1,
    "pageSize": 20,
    "totalPages": 5
  }
}
```

---

### GET `/api/anis/{id}`

查询单条番剧详情。

**路径参数**

| 参数   | 类型   | 说明    |
|------|------|-------|
| `id` | i64  | 番剧 ID |

**响应** `200 OK` → `AniInfoDto`

```json
{
  "status": "ok",
  "data": {
    "id": 1,
    "title": "进击的巨人",
    "updateCount": "第25话",
    "updateInfo": "已完结",
    "imageUrl": "https://...",
    "detailUrl": "https://...",
    "updateTime": "2024-01-01T00:00:00Z",
    "platform": "bilibili"
  }
}
```

**错误**

| 状态码 | 说明      |
|-----|---------|
| 404 | 番剧不存在   |

---

## 番剧收藏

> 以下接口为用户私有数据，所有操作均隔离到当前登录用户。

---

### GET `/api/anis/collect`

分页查询当前用户的番剧收藏列表。

**Query 过滤字段**

| 参数         | 类型      | 说明              |
|------------|---------|-----------------|
| `aniTitle` | string  | 番剧标题（模糊匹配）    |
| `isWatched`| boolean | 是否已观看过滤        |

**响应** `200 OK` → `PageData<AniCollectDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 10,
        "aniItemId": 1,
        "aniTitle": "进击的巨人",
        "collectTime": "2024-03-01T12:00:00Z",
        "isWatched": false
      }
    ],
    "totalCount": 5,
    "page": 1,
    "pageSize": 20,
    "totalPages": 1
  }
}
```

---

### POST `/api/anis/collect`

添加番剧到收藏。

**请求体** `application/json`

```json
{
  "aniItemId": 1,
  "aniTitle": "进击的巨人"
}
```

| 字段          | 类型     | 必填 | 说明       |
|-------------|--------|----|----------|
| `aniItemId` | i64    | ✓  | 番剧 ID    |
| `aniTitle`  | string | ✓  | 番剧标题（冗余） |

**响应** `201 Created` → `AniCollectDTO`

```json
{
  "status": "ok",
  "data": {
    "id": 10,
    "aniItemId": 1,
    "aniTitle": "进击的巨人",
    "collectTime": "2024-03-01T12:00:00Z",
    "isWatched": false
  }
}
```

**错误**

| 状态码 | 说明                          |
|-----|-----------------------------|
| 400 | 该番剧已收藏（UNIQUE 约束冲突）        |

---

### DELETE `/api/anis/collect/{id}`

取消收藏（仅可操作自己的收藏记录）。

**路径参数**

| 参数   | 类型  | 说明     |
|------|-----|--------|
| `id` | i64 | 收藏记录 ID |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": null
}
```

**错误**

| 状态码 | 说明                 |
|-----|--------------------|
| 404 | 收藏记录不存在或不属于当前用户   |

---

### PATCH `/api/anis/collect/{id}/watched`

标记或取消观看状态。

**路径参数**

| 参数   | 类型  | 说明     |
|------|-----|--------|
| `id` | i64 | 收藏记录 ID |

**请求体** `application/json`

```json
{
  "isWatched": true
}
```

| 字段          | 类型      | 必填 | 说明             |
|-------------|---------|-----|----------------|
| `isWatched` | boolean | ✓   | `true`=已看 / `false`=未看 |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": null
}
```

**错误**

| 状态码 | 说明                 |
|-----|--------------------|
| 404 | 收藏记录不存在或不属于当前用户   |

---

## 新闻信息

### GET `/api/news`

分页查询新闻信息源列表（原始抓取数据）。

**Query 过滤字段**

| 参数          | 类型      | 说明                    |
|-------------|---------|------------------------|
| `newsFrom`  | string  | 新闻来源（模糊匹配）           |
| `newsDate`  | string  | 新闻日期，格式 `YYYY-MM-DD` |
| `extracted` | boolean | 是否已提取                 |

**响应** `200 OK` → `PageData<NewsInfoDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 1,
        "newsFrom": "36kr",
        "newsDate": "2024-03-01",
        "data": {},
        "createdAt": "2024-03-01T08:00:00Z",
        "updatedAt": "2024-03-01T09:00:00Z",
        "name": "36kr",
        "extracted": true,
        "extractedAt": "2024-03-01T09:05:00Z"
      }
    ],
    "totalCount": 200,
    "page": 1,
    "pageSize": 20,
    "totalPages": 10
  }
}
```

---

### GET `/api/news/items`

分页查询已提取的新闻条目列表。

**Query 过滤字段**

| 参数            | 类型      | 说明                      |
|-------------|---------|-------------------------|
| `source`    | string  | 新闻来源名称（模糊匹配）           |
| `publishedAt` | string | 发布日期，格式 `YYYY-MM-DD`    |
| `clusterId` | i64     | 聚类 ID（精确匹配）             |
| `extracted` | boolean | 是否已二次提取（关键词/事件）        |

**响应** `200 OK` → `PageData<NewsItemResponseDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 100,
        "itemId": "abc123",
        "title": "某科技公司发布新产品",
        "url": "https://example.com/news/1",
        "source": "36kr",
        "publishedAt": "2024-03-01",
        "clusterId": 5,
        "extracted": true,
        "createdAt": "2024-03-01T08:00:00Z"
      }
    ],
    "totalCount": 500,
    "page": 1,
    "pageSize": 20,
    "totalPages": 25
  }
}
```

---

### GET `/api/news/events`

分页查询热点事件列表。

**Query 过滤字段**

| 参数          | 类型    | 说明                                                          |
|-------------|-------|-------------------------------------------------------------|
| `eventDate` | string | 事件日期，格式 `YYYY-MM-DD`                                       |
| `status`    | i16   | 事件状态：`0`=自动生成 / `1`=已确认 / `2`=已归档 / `3`=已合并 |

**响应** `200 OK` → `PageData<NewsEventDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 1,
        "eventDate": "2024-03-01",
        "clusterId": 5,
        "title": "AI 大模型竞争加剧",
        "summary": "本周多家科技公司相继发布新一代 AI 模型...",
        "newsCount": 12,
        "score": 0.92,
        "status": 1,
        "parentEventId": null,
        "createdAt": "2024-03-01T10:00:00Z"
      }
    ],
    "totalCount": 30,
    "page": 1,
    "pageSize": 20,
    "totalPages": 2
  }
}
```

**`status` 枚举说明**

| 值 | 含义     |
|---|--------|
| 0 | 自动生成   |
| 1 | 已人工确认  |
| 2 | 已归档    |
| 3 | 已合并到其他事件 |

---

### GET `/api/news/events/{id}/items`

查询指定热点事件下关联的所有新闻条目（不分页）。

**路径参数**

| 参数   | 类型  | 说明     |
|------|-----|--------|
| `id` | i64 | 热点事件 ID |

**响应** `200 OK` → `Vec<NewsItemResponseDTO>`

```json
{
  "status": "ok",
  "data": [
    {
      "id": 100,
      "itemId": "abc123",
      "title": "某科技公司发布新产品",
      "url": "https://example.com/news/1",
      "source": "36kr",
      "publishedAt": "2024-03-01",
      "clusterId": 5,
      "extracted": true,
      "createdAt": "2024-03-01T08:00:00Z"
    }
  ]
}
```

---

## 定时任务

### GET `/api/scheduledTasks`

分页查询定时任务列表。

**Query 过滤字段**

| 参数          | 类型      | 说明              |
|-------------|---------|-----------------|
| `name`      | string  | 任务名称（模糊匹配）     |
| `isEnabled` | boolean | 是否启用            |

**响应** `200 OK` → `PageData<ScheduledTasksDTO>`

```json
{
  "status": "ok",
  "data": {
    "items": [
      {
        "id": 1,
        "name": "抓取36kr新闻",
        "cron": "0 */6 * * *",
        "params": {},
        "isEnabled": true,
        "retryTimes": 3,
        "lastRun": "2024-03-01T06:00:00Z",
        "nextRun": "2024-03-01T12:00:00Z",
        "lastStatus": "success"
      }
    ],
    "totalCount": 10,
    "page": 1,
    "pageSize": 20,
    "totalPages": 1
  }
}
```

---

### POST `/api/scheduledTasks`

创建新定时任务。

**请求体** `application/json`

```json
{
  "name": "抓取36kr新闻",
  "cron": "0 */6 * * *",
  "params": { "source": "36kr" },
  "isEnabled": true,
  "retryTimes": 3
}
```

| 字段           | 类型      | 必填 | 默认值   | 说明           |
|--------------|---------|-----|-------|--------------|
| `name`       | string  | ✓   | —     | 任务名称（唯一）    |
| `cron`       | string  | ✓   | —     | Cron 表达式    |
| `params`     | object  | ✓   | —     | 任务参数（任意 JSON）|
| `isEnabled`  | boolean |     | false | 是否立即启用      |
| `retryTimes` | number  |     | 3     | 失败重试次数      |

**响应** `201 Created` → `ScheduledTasksDTO`

---

### PUT `/api/scheduledTasks/{id}`

更新定时任务（所有字段均可选）。

**路径参数**

| 参数   | 类型  | 说明     |
|------|-----|--------|
| `id` | i64 | 任务 ID  |

**请求体** `application/json`

```json
{
  "name": "新名称",
  "cron": "0 8 * * *",
  "params": { "source": "36kr" },
  "retryTimes": 5
}
```

| 字段           | 类型     | 说明           |
|--------------|--------|--------------|
| `name`       | string | 任务名称         |
| `cron`       | string | Cron 表达式    |
| `params`     | object | 任务参数         |
| `retryTimes` | number | 失败重试次数      |

**响应** `200 OK` → `ScheduledTasksDTO`

**错误**

| 状态码 | 说明        |
|-----|-----------|
| 404 | 任务不存在     |

---

### PATCH `/api/scheduledTasks/{id}/status`

切换定时任务的启停状态。

**路径参数**

| 参数   | 类型  | 说明    |
|------|-----|-------|
| `id` | i64 | 任务 ID |

**请求体** `application/json`

```json
{
  "isEnabled": false
}
```

**响应** `200 OK` → `ScheduledTasksDTO`

---

### DELETE `/api/scheduledTasks/{id}`

删除定时任务。

**路径参数**

| 参数   | 类型  | 说明    |
|------|-----|-------|
| `id` | i64 | 任务 ID |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": null
}
```

**错误**

| 状态码 | 说明      |
|-----|---------|
| 404 | 任务不存在   |

---

### POST `/api/sync/task_source`

同步（upsert）任务数据源配置。

**请求体** `application/json`

```json
{
  "name": "抓取36kr新闻",
  "cron": "0 */6 * * *",
  "params": { "source": "36kr" },
  "retryTimes": 3
}
```

| 字段           | 类型     | 必填 | 说明            |
|--------------|--------|----|---------------|
| `name`       | string | ✓  | 任务名称（ON CONFLICT 键）|
| `cron`       | string | ✓  | Cron 表达式     |
| `params`     | object | ✓  | 任意 JSON 参数   |
| `retryTimes` | number | ✓  | 重试次数          |

**响应** `200 OK`

```json
{
  "status": "ok",
  "data": {
    "message": "同步成功"
  }
}
```

---

## 管理接口

> 需要认证，仅限管理员角色调用。

### POST `/admin/task/reload`

热重载任务调度器配置（无需重启服务）。

**请求体** 无

**响应** `200 OK`

---

## 图片代理

### GET `/api/proxy/image`

代理外部图片请求，防止跨域问题。仅支持白名单域名的 HTTP/HTTPS 图片。

**无需认证**

**Query 参数**

| 参数    | 类型     | 必填 | 说明                     |
|-------|--------|----|------------------------|
| `url` | string | ✓  | 要代理的图片 URL（需 URL 编码）  |

**响应** `200 OK`，`Content-Type: image/*`，返回图片二进制流。

**错误**

| 状态码 | 说明                    |
|-----|-----------------------|
| 400 | 缺少 url / url 格式非法 / 域名不在白名单 |

---

## 数据模型汇总

### `AniInfoDto`

| 字段           | 类型     | 说明       |
|--------------|--------|----------|
| `id`         | i64    | 番剧 ID    |
| `title`      | string | 番剧标题     |
| `updateCount`| string | 更新集数     |
| `updateInfo` | string | 更新说明     |
| `imageUrl`   | string | 封面图 URL  |
| `detailUrl`  | string | 详情页 URL  |
| `updateTime` | string | 最新更新时间   |
| `platform`   | string | 来源平台     |

### `AniCollectDTO`

| 字段            | 类型       | 说明          |
|---------------|----------|-------------|
| `id`          | i64      | 收藏记录 ID    |
| `aniItemId`   | i64      | 番剧 ID       |
| `aniTitle`    | string   | 番剧标题        |
| `collectTime` | datetime | 收藏时间（ISO 8601）|
| `isWatched`   | boolean  | 是否已观看      |

### `NewsInfoDTO`

| 字段            | 类型       | 说明            |
|---------------|----------|---------------|
| `id`          | i64      | 新闻源 ID       |
| `newsFrom`    | string   | 数据来源名称       |
| `newsDate`    | date     | 新闻日期         |
| `data`        | object   | 原始抓取 JSON 数据  |
| `name`        | string   | 来源名称         |
| `extracted`   | boolean  | 是否已提取条目      |
| `extractedAt` | datetime | 提取时间         |
| `createdAt`   | datetime | 创建时间         |
| `updatedAt`   | datetime | 更新时间         |

### `NewsItemResponseDTO`

| 字段           | 类型       | 说明             |
|--------------|----------|----------------|
| `id`         | i64      | 条目 ID          |
| `itemId`     | string   | 原始条目唯一标识      |
| `title`      | string   | 新闻标题           |
| `url`        | string   | 原文链接           |
| `source`     | string?  | 来源名称           |
| `publishedAt`| date     | 发布日期（`YYYY-MM-DD`）|
| `clusterId`  | i64?     | 聚类 ID          |
| `extracted`  | boolean  | 是否已做关键词/事件提取   |
| `createdAt`  | datetime? | 入库时间          |

### `NewsEventDTO`

| 字段              | 类型       | 说明                          |
|-----------------|----------|-----------------------------|
| `id`            | i64      | 事件 ID                       |
| `eventDate`     | date     | 事件日期                        |
| `clusterId`     | i64      | 聚类 ID                       |
| `title`         | string?  | 事件标题                        |
| `summary`       | string?  | 事件摘要                        |
| `newsCount`     | i32      | 关联新闻条数                      |
| `score`         | f32?     | 热度评分                        |
| `status`        | i16      | 状态（0=自动/1=确认/2=归档/3=合并）   |
| `parentEventId` | i64?     | 合并到的父事件 ID                  |
| `createdAt`     | datetime | 创建时间                        |

### `ScheduledTasksDTO`

| 字段           | 类型       | 说明             |
|--------------|----------|----------------|
| `id`         | i64      | 任务 ID          |
| `name`       | string   | 任务名称           |
| `cron`       | string   | Cron 表达式       |
| `params`     | object   | 任务参数 JSON      |
| `isEnabled`  | boolean  | 是否启用           |
| `retryTimes` | number   | 最大重试次数         |
| `lastRun`    | datetime? | 上次运行时间        |
| `nextRun`    | datetime? | 下次运行时间        |
| `lastStatus` | string   | 上次运行结果        |
