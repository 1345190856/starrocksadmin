# StarRocks Admin 项目概况

## 项目简介

**StarRocks Admin** 是一个现代化的 StarRocks 数据库集群管理平台，提供直观的 Web 界面来管理和监控多个 StarRocks 集群。相比 StarRocks 原生的管理界面，本平台提供了更丰富的功能和更好的用户体验。

## 1. 项目组织结构

```
starrocks-admin/
├── backend/                    # 后端服务 (Rust)
│   ├── src/                   # 源码目录
│   │   ├── handlers/          # API 处理器 (路由处理)
│   │   ├── services/          # 业务服务层 (核心业务逻辑)
│   │   ├── models/            # 数据模型 (结构体定义)
│   │   ├── middleware/        # 中间件 (认证、权限等)
│   │   ├── db/               # 数据库层 (连接池、迁移)
│   │   ├── utils/            # 工具类 (JWT、调度器等)
│   │   ├── config.rs         # 配置管理
│   │   ├── embedded.rs       # 静态资源嵌入
│   │   └── main.rs           # 程序入口
│   ├── migrations/           # 数据库迁移文件
│   ├── tests/               # 单元测试和集成测试
│   ├── Cargo.toml           # Rust 依赖配置
│   ├── build.rs             # 构建脚本 (嵌入前端资源)
│   └── rust-toolchain.toml  # Rust 工具链版本
├── frontend/                  # 前端应用 (Angular)
│   ├── src/app/              # Angular 应用源码
│   │   ├── @core/            # 核心模块 (服务、拦截器)
│   │   ├── @theme/           # 主题样式 (Nebular 主题)
│   │   ├── auth/             # 认证模块 (登录、注册)
│   │   └── pages/            # 页面组件 (业务页面)
│   ├── package.json          # Node.js 依赖配置
│   ├── angular.json          # Angular 构建配置
│   └── tsconfig.json         # TypeScript 配置
├── build/                     # 构建脚本目录
│   ├── build-frontend.sh     # 前端构建脚本
│   ├── build-backend.sh      # 后端构建脚本
│   └── verify-base-href.sh   # 基础路径验证
├── deploy/                    # 部署配置目录
│   ├── docker/               # Docker 部署
│   │   ├── Dockerfile        # 多阶段构建镜像
│   │   └── docker-compose.yml # Docker Compose 配置
│   ├── k8s/                  # Kubernetes 部署
│   │   └── deploy-all.yaml   # K8s 资源定义
│   └── chart/                # Helm Chart
│       ├── Chart.yaml        # Chart 元数据
│       ├── values.yaml       # 默认配置值
│       └── templates/        # K8s 模板文件
├── docs/                      # 项目文档
│   ├── deploy/               # 部署指南
│   ├── images/               # 界面截图
│   └── profile/              # 性能分析文档
├── scripts/                   # 脚本工具
│   ├── permissions/          # 权限配置脚本
│   ├── dev/                  # 开发工具脚本
│   └── test/                 # 测试脚本
├── Makefile                   # 构建任务定义
├── README.md                  # 项目说明文档
└── CHANGELOG.md               # 版本更新日志
```

## 2. 技术栈和框架

### 后端技术栈 (Rust)

| 组件 | 技术选型 | 版本要求 | 用途说明 |
|------|---------|---------|---------|
| **Web 框架** | Axum | 0.7+ | 高性能异步 Web 框架，基于 Tokio |
| **数据库** | SQLite + MySQL | - | SQLite 本地存储，MySQL 连接 StarRocks |
| **ORM** | SQLx | 0.7+ | 类型安全的 SQL 工具包，支持编译时检查 |
| **认证** | jsonwebtoken | 9.2+ | JWT Token 生成和验证 |
| **权限控制** | Casbin | 2.7+ | 基于 RBAC 的权限管理框架 |
| **API 文档** | utoipa + Swagger UI | 4.2+ | 自动生成 OpenAPI 文档 |
| **日志** | tracing | 0.1+ | 结构化日志记录和追踪 |
| **配置** | toml + clap | - | TOML 配置文件 + 命令行参数解析 |
| **静态文件** | rust-embed | 8.0+ | 将前端资源嵌入到二进制文件 |
| **HTTP 客户端** | reqwest | 0.11+ | 异步 HTTP 客户端，连接 StarRocks API |
| **序列化** | serde | 1.0+ | JSON/TOML 序列化和反序列化 |

### 前端技术栈 (Angular)

| 组件 | 技术选型 | 版本要求 | 用途说明 |
|------|---------|---------|---------|
| **框架** | Angular | 15.2+ | 现代化前端框架 |
| **UI 库** | Nebular | 11.0+ | 基于 Eva Design System 的 UI 组件库 |
| **图表** | ECharts + ngx-charts | 4.9+ / 23.0+ | 数据可视化图表库 |
| **代码编辑器** | CodeMirror | 6.x | SQL 代码编辑器，支持语法高亮 |
| **样式** | SCSS + Bootstrap | 4.3+ | 样式预处理器 + CSS 框架 |
| **图标** | Eva Icons + FontAwesome | - | 图标库 |
| **状态管理** | RxJS | 6.6+ | 响应式编程库 |
| **HTTP 客户端** | Angular HttpClient | - | 与后端 API 通信 |

## 3. 程序入口和启动流程

### 后端入口 (`backend/src/main.rs`)

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 加载配置 (config.toml + 环境变量 + CLI 参数)
    let config = Config::load()?;
    
    // 2. 初始化日志系统 (文件日志 + 控制台日志)
    init_logging(&config.logging)?;
    
    // 3. 创建数据库连接池 (SQLite)
    let pool = db::create_pool(&config.database.url).await?;
    
    // 4. 初始化各种服务
    let auth_service = Arc::new(AuthService::new(pool.clone(), jwt_util));
    let cluster_service = Arc::new(ClusterService::new(pool.clone(), mysql_pool_manager));
    // ... 其他服务初始化
    
    // 5. 启动指标收集器 (可选，定时采集集群指标)
    if config.metrics.enabled {
        start_metrics_collector(metrics_collector_service).await;
    }
    
    // 6. 构建 Axum 路由 (公开路由 + 受保护路由)
    let app = build_router(app_state).await;
    
    // 7. 启动 HTTP 服务器
    axum::serve(listener, app).await?;
}
```

### 前端入口 (`frontend/src/main.ts`)

```typescript
// 1. 启动 Angular 应用
platformBrowserDynamic()
  .bootstrapModule(AppModule)
  .catch(err => console.error(err));
```

### 应用架构图

```
┌─────────────────┐    HTTP/WebSocket    ┌─────────────────┐
│   Angular SPA   │ ◄─────────────────► │   Axum Server   │
│                 │                      │                 │
│ ┌─────────────┐ │                      │ ┌─────────────┐ │
│ │   Pages     │ │                      │ │  Handlers   │ │
│ │             │ │                      │ │             │ │
│ │ ┌─────────┐ │ │                      │ │ ┌─────────┐ │ │
│ │ │ Auth    │ │ │                      │ │ │ Auth    │ │ │
│ │ │ Cluster │ │ │                      │ │ │ Cluster │ │ │
│ │ │ Query   │ │ │                      │ │ │ Query   │ │ │
│ │ └─────────┘ │ │                      │ │ └─────────┘ │ │
│ └─────────────┘ │                      │ └─────────────┘ │
│                 │                      │                 │
│ ┌─────────────┐ │                      │ ┌─────────────┐ │
│ │  Services   │ │                      │ │  Services   │ │
│ └─────────────┘ │                      │ └─────────────┘ │
└─────────────────┘                      └─────────────────┘
                                                   │
                                                   ▼
                                         ┌─────────────────┐
                                         │     SQLite      │
                                         │   (本地存储)     │
                                         └─────────────────┘
                                                   │
                                                   ▼
                                         ┌─────────────────┐
                                         │   StarRocks     │
                                         │   (目标集群)     │
                                         └─────────────────┘
```

## 4. 环境要求

### 开发环境

| 工具 | 最低版本 | 推荐版本 | 安装方式 |
|------|---------|---------|---------|
| **Rust** | 1.75+ | 1.85+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Node.js** | 18+ | 20+ | `nvm install 20 && nvm use 20` |
| **npm** | 8+ | 10+ | 随 Node.js 安装 |

### 生产环境

| 资源 | 最低要求 | 推荐配置 | 说明 |
|------|---------|---------|------|
| **CPU** | 1 核 | 2 核+ | 支持并发请求处理 |
| **内存** | 256MB | 1GB+ | 包含 SQLite 缓存和连接池 |
| **磁盘** | 50MB | 1GB+ | 日志文件和数据库存储 |
| **操作系统** | Linux x86_64 | Linux x86_64 | 支持 Docker 部署 |

### 软件依赖

| 部署方式 | 依赖要求 |
|---------|---------|
| **本地部署** | Rust 1.75+, Node.js 18+ |
| **Docker 部署** | Docker 20.10+, Docker Compose 2.0+ |
| **Kubernetes 部署** | Kubernetes 1.16+, kubectl |
| **Helm 部署** | Helm 3.0+, Kubernetes 1.16+ |

## 5. 编译部署流程

### 5.1 本地编译部署

```bash
# 1. 环境准备
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
nvm install 18 && nvm use 18

# 2. 克隆项目
git clone https://github.com/annoymous/starrocksadmin.git
cd starrocks-admin

# 3. 一键构建
make build
# 构建过程:
# - 前端构建: npm install + ng build --prod --base-href ./
# - 代码检查: cargo clippy --release --all-targets
# - 后端构建: cargo build --release
# - 打包分发: 创建 tar.gz 包

# 4. 启动服务
cd build/dist
./bin/starrocks-admin.sh start

# 5. 访问应用
open http://localhost:8080
```

### 5.2 Docker 部署

```bash
# 方式1: 使用预构建镜像 (推荐)
docker pull ghcr.io/annoymous/starrocksadmin:latest
docker run -d -p 8080:8080 --name starrocks-admin \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/logs:/app/logs \
  --restart unless-stopped \
  ghcr.io/annoymous/starrocksadmin:latest

# 方式2: 从源码构建
make docker-build  # 构建镜像
make docker-up     # 启动容器

# 方式3: Docker Compose
cd deploy/docker
docker compose up -d
```

### 5.3 Kubernetes 部署

```bash
# YAML 部署 (简单环境)
kubectl apply -f deploy/k8s/deploy-all.yaml
kubectl wait --for=condition=ready pod -l app=starrocks-admin -n starrocks-admin

# Helm 部署 (生产环境推荐)
helm install starrocks-admin deploy/chart -n starrocks-admin --create-namespace

# 自定义配置部署
helm install starrocks-admin deploy/chart \
  --set image.tag=1.0.0 \
  --set service.type=LoadBalancer \
  --set persistence.size=20Gi \
  --set jwtSecret="your-secure-jwt-secret"
```

## 6. 核心功能模块

### 6.1 集群管理
- **多集群支持**: 统一管理多个 StarRocks 集群
- **连接测试**: 验证集群连接状态和权限
- **集群切换**: 快速切换当前操作的集群
- **健康检查**: 实时监控集群健康状态

### 6.2 实时监控
- **性能指标**: CPU、内存、磁盘、网络使用率
- **查询统计**: QPS、平均响应时间、错误率
- **资源趋势**: 历史性能数据和趋势分析
- **告警通知**: 异常状态自动告警

### 6.3 查询管理
- **实时查询**: 查看正在执行的查询列表
- **查询终止**: 强制终止长时间运行的查询
- **SQL 执行**: 在线 SQL 编辑器，支持语法高亮
- **审计日志**: 完整的查询历史记录和审计

### 6.4 节点管理
- **FE 节点**: Frontend 节点状态监控和管理
- **BE 节点**: Backend 节点性能指标和资源使用
- **节点操作**: 节点上下线、状态切换

### 6.5 物化视图
- **视图列表**: 查看所有物化视图及其状态
- **创建视图**: 通过 SQL 创建新的物化视图
- **刷新管理**: 手动或自动刷新物化视图
- **性能分析**: 物化视图使用统计和优化建议

### 6.6 权限管理 (RBAC)
- **用户管理**: 创建、编辑、删除用户账户
- **角色管理**: 定义角色和权限集合
- **权限分配**: 为用户分配角色和权限
- **组织管理**: 多租户组织隔离

## 7. 配置管理

### 7.1 配置优先级 (从高到低)
1. **命令行参数**: `--server-host`, `--server-port` 等
2. **环境变量**: `APP_SERVER_HOST`, `APP_SERVER_PORT` 等
3. **配置文件**: `conf/config.toml`
4. **默认值**: 代码中定义的默认配置

### 7.2 主要配置项

```toml
[server]
host = "0.0.0.0"              # 服务监听地址
port = 8080                   # 服务监听端口

[database]
url = "sqlite://data/starrocks-admin.db"  # SQLite 数据库路径

[auth]
jwt_secret = "your-secret-key"            # JWT 签名密钥 (生产环境必须修改)
jwt_expires_in = "24h"                    # Token 过期时间

[logging]
level = "info,starrocks_admin=debug"      # 日志级别
file = "logs/starrocks-admin.log"         # 日志文件路径

[metrics]
interval_secs = "30s"         # 指标采集间隔 (支持 30s, 5m, 1h 格式)
retention_days = "7d"         # 历史数据保留时长 (支持 7d, 2w 格式)
enabled = true                # 是否启用指标采集

[audit]
database = "starrocks_audit_db__"         # StarRocks 审计日志数据库
table = "starrocks_audit_tbl__"           # StarRocks 审计日志表
```

### 7.3 环境变量覆盖示例

```bash
# 生产环境启动示例
APP_SERVER_PORT=8080 \
APP_JWT_SECRET="your-secure-jwt-secret-32-chars" \
APP_LOG_LEVEL="info" \
APP_METRICS_ENABLED=true \
APP_METRICS_INTERVAL_SECS=1m \
APP_METRICS_RETENTION_DAYS=14d \
./bin/starrocks-admin
```

## 8. 架构特点

### 8.1 技术架构
- **前后端分离**: Angular SPA + Rust API 服务
- **嵌入式部署**: 前端资源嵌入到后端二进制文件
- **异步处理**: 基于 Tokio 的高性能异步运行时
- **类型安全**: Rust 类型系统保证内存安全和并发安全

### 8.2 部署架构
- **单体应用**: 前后端打包为单个可执行文件
- **云原生**: 支持 Docker、Kubernetes、Helm 部署
- **多环境**: 开发、测试、生产环境配置分离
- **可扩展**: 支持水平扩展和负载均衡

### 8.3 安全特性
- **JWT 认证**: 无状态的用户身份验证
- **RBAC 权限**: 基于角色的访问控制
- **多租户**: 组织级别的数据隔离
- **审计日志**: 完整的操作审计追踪

## 9. 开发和维护

### 9.1 代码质量
- **Rust Clippy**: 静态代码分析和 lint 检查
- **TypeScript**: 前端类型安全检查
- **单元测试**: 核心业务逻辑测试覆盖
- **集成测试**: 端到端功能测试

### 9.2 CI/CD 流程
- **GitHub Actions**: 自动化构建、测试、发布
- **多架构构建**: 支持 amd64 和 arm64 架构
- **版本管理**: 语义化版本控制和自动发布
- **镜像发布**: 自动推送到 GitHub Container Registry

### 9.3 监控和运维
- **健康检查**: `/health` 和 `/ready` 端点
- **指标采集**: 自动采集集群性能指标
- **日志管理**: 结构化日志和文件轮转
- **配置热更新**: 支持运行时配置更新

## 10. 扩展和定制

### 10.1 插件机制
- **服务扩展**: 通过依赖注入添加新服务
- **中间件**: 自定义认证和权限中间件
- **API 扩展**: 添加新的 REST API 端点

### 10.2 主题定制
- **Nebular 主题**: 支持自定义 UI 主题
- **品牌定制**: Logo、颜色、字体等视觉元素
- **国际化**: 支持多语言界面 (预留)

### 10.3 集成能力
- **外部认证**: 支持 LDAP、OAuth2 等认证方式
- **监控集成**: 与 Prometheus、Grafana 集成
- **告警通知**: 支持邮件、钉钉、企业微信等通知方式

---

## 总结

StarRocks Admin 是一个现代化的企业级数据库管理平台，采用 Rust + Angular 技术栈，提供了完整的 StarRocks 集群管理解决方案。项目具有高性能、高安全性、易部署的特点，支持多种部署方式，适合从开发测试到生产环境的各种场景使用。

**项目地址**: https://github.com/annoymous/starrocksadmin  