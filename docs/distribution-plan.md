# Tessera Distribution Plan

日期：2026-05-16

本文定义 Tessera 从源码运行走向多渠道安装的分发路线。它是 v0.2 的计划文档，不实现发布流水线，不改变 runtime 范围。

## 1. Goals

- 保持 Rust-first：`tessera` CLI/TUI 是主要运行入口，runtime 不依赖 Node。
- 保持单一 headless runtime：Cargo、GitHub Releases、Homebrew、npm wrapper 和 Docker 都分发同一个 CLI/TUI 能力面。
- 保持可审计：release asset、checksum、版本号、commit SHA 和 `doctor --json` 输出可互相核对。
- 保持 secret-safe：任何安装、构建、镜像或 smoke test 都不得写入 provider token、cookie、authorization header 或 `.env` 内容。
- 保持 GUI 后置：Tauri GUI asset 可在 GUI 进入产品化后加入同一 release，但 v0.2 分发计划只要求 CLI/TUI。

## 2. Channel Ownership

| Channel | Audience | Source of truth | v0.3+ output |
| --- | --- | --- | --- |
| GitHub Releases | 直接下载安装、Homebrew、npm wrapper、Docker build 输入 | tag + release assets | signed or checksummed archives |
| Cargo | Rust 用户和源码构建用户 | crates.io packages | `cargo install tessera-cli` |
| Homebrew | macOS/Linux 日常安装 | GitHub Release archive | formula in tap |
| npm wrapper | Node 生态用户和国内镜像用户 | GitHub Release archive or mirror | package that installs a binary |
| Docker | CI、服务器和隔离运行 | GitHub Container Registry | image with `tessera` entrypoint |

GitHub Releases 是二进制 asset 的事实来源。其他二进制渠道只引用 release version、target triple 和 checksum，不重新定义 runtime 行为。

## 3. Versioning

- Release tag 使用 `vMAJOR.MINOR.PATCH`，预发布使用 `vMAJOR.MINOR.PATCH-alpha.N`。
- Rust crate version、CLI `--version`、GitHub tag、Homebrew formula version、npm wrapper version 和 Docker tag 必须一致。
- 预发布可以只发 GitHub Releases 和 Docker preview tag，不要求 Cargo/Homebrew/npm 全量发布。
- 每次 release 必须有 changelog section，并指明 live provider smoke 是否执行。

## 4. GitHub Releases

GitHub Releases 负责承载所有可下载二进制和校验文件。

首批 CLI/TUI asset 目标：

- `tessera-$VERSION-aarch64-apple-darwin.tar.gz`
- `tessera-$VERSION-x86_64-apple-darwin.tar.gz`
- `tessera-$VERSION-x86_64-unknown-linux-gnu.tar.gz`
- `tessera-$VERSION-x86_64-unknown-linux-musl.tar.gz`
- `tessera-$VERSION-x86_64-pc-windows-msvc.zip`
- `checksums.txt`

每个 archive 至少包含：

- `bin/tessera`
- `README.md`
- `LICENSE`
- `CHANGELOG.md` 或当前 release note

Release job gate：

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo fmt --all -- --check
PATH="$HOME/.cargo/bin:$PATH" cargo clippy --workspace --all-targets -- -D warnings
PATH="$HOME/.cargo/bin:$PATH" cargo test --workspace
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-cli -- doctor --json
PATH="$HOME/.cargo/bin:$PATH" cargo run -p tessera-cli -- chat --provider mock --prompt "hello"
```

GUI asset 进入 release 的前置条件：

- Tauri shell 不再只是 mock/replay spike。
- GUI 有真实 runtime read-only 或 live event API 边界。
- GUI 有可重复的 browser screenshot / interaction gate。
- Tauri bundle capability allowlist 已复核。

## 5. Cargo

Cargo 渠道目标是支持：

```bash
cargo install tessera-cli
```

发布顺序：

1. `tessera-protocol`
2. `tessera-client`
3. `tessera-config`
4. `tessera-storage`
5. `tessera-providers`
6. `tessera-core`
7. `tessera-tui`
8. `tessera-cli`

Cargo 发布前置：

- 每个要发布的 crate 都必须有 `repository.workspace = true` 或等价 metadata。
- 内部依赖必须同时包含 `path` 和 crates.io `version`，避免 `cargo publish` 失败。
- `tessera-gui-bridge` 和 `tessera-gui-bindings` 先保留 workspace internal，直到 GUI API 稳定。
- `tessera-cli` 是用户安装入口，库 crate 不承诺 public API stability，除非单独写入 crate README。

## 6. Homebrew

Homebrew 渠道目标是支持：

```bash
brew install aicode-nexus/tap/tessera
```

Formula 应从 GitHub Release archive 拉取二进制，使用 `checksums.txt` 中的 SHA-256 校验。Formula test 只运行不需要 provider secret 的命令：

```bash
tessera --version
tessera doctor --json
```

Homebrew 不应编译 Tauri GUI。GUI app 后续通过 Tauri bundle 或 cask 单独进入。

## 7. npm Wrapper

npm wrapper 是二进制安装器，不是 runtime 依赖。安装后应暴露：

```bash
npx tessera --version
```

设计约束：

- npm package 不包含 provider SDK，不执行 Tessera runtime 逻辑。
- wrapper 只解析平台、选择 release asset、下载或定位本地 binary。
- 支持 `TESSERA_INSTALL_BASE_URL`，便于国内镜像或私有镜像。
- 支持 `TESSERA_BINARY_PATH`，便于 CI 使用预下载二进制。
- 支持 `TESSERA_SKIP_DOWNLOAD=1`，便于离线或打包环境。
- 不在 postinstall 输出 secret、env dump 或 provider config。

实现时优先评估两种形态：

- 单包 postinstall downloader：简单，但安装依赖网络。
- wrapper + platform optionalDependencies：更适合 npm 镜像缓存，但维护包更多。

v0.3 首选可以先做单包 downloader，等下载失败率和镜像需求明确后再拆平台包。

## 8. Docker

Docker 渠道目标是支持：

```bash
docker run --rm ghcr.io/AICode-Nexus/tessera:$VERSION --version
docker run --rm -v "$PWD/.tessera:/data" ghcr.io/AICode-Nexus/tessera:$VERSION doctor --json
```

镜像约束：

- 默认 entrypoint 是 `tessera`。
- 使用非 root 用户运行。
- 不内置 provider secret 或 `.env`。
- 默认 data dir 应可通过环境变量或挂载目录指向 `/data`。
- 镜像只包含 CLI/TUI runtime，不包含 GUI WebView runtime。
- image tag 至少包含 `$VERSION` 和 `latest`；预发布使用 `alpha` 或完整预发布 tag，不覆盖 stable `latest`。

Docker smoke：

```bash
docker run --rm ghcr.io/AICode-Nexus/tessera:$VERSION --version
docker run --rm ghcr.io/AICode-Nexus/tessera:$VERSION doctor --json
```

## 9. Release Workflow Shape

后续 GitHub Actions 可以拆成四个 job：

1. `quality`: fmt、clippy、test、mock doctor/chat smoke。
2. `build-assets`: 按 target 构建 CLI/TUI archive，生成 checksums。
3. `publish-release`: 创建 GitHub Release 并上传 assets。
4. `publish-channels`: 在 release asset 可用后发布 Cargo、Homebrew、npm wrapper、Docker。

`publish-channels` 必须可重跑，并且不得重新构建与 GitHub Release 不一致的二进制。

## 10. Acceptance Checklist

分发实现进入 v0.3+ 前，需要逐项完成：

- [x] `tessera --version` 输出 crate version 和 git SHA。
- [ ] Cargo publish dry-run 通过所有 public crate。
- [ ] GitHub Release archive 可在 macOS/Linux/Windows 解压并运行 `doctor --json`。
- [ ] `checksums.txt` 可验证所有 release assets。
- [ ] Homebrew formula install 和 formula test 通过。
- [ ] npm wrapper 可在不安装 Rust toolchain 的环境执行 `tessera --version`。
- [ ] Docker image 以非 root 用户运行 `doctor --json`。
- [ ] Release notes 明确 live provider smoke 状态。
- [ ] 安装文档说明如何使用镜像 base URL，但不记录任何 secret。

## 11. Explicit Non-Goals

- v0.2 不新增 GitHub Release workflow。
- v0.2 不发布 crates.io package。
- v0.2 不发布 Homebrew tap。
- v0.2 不发布 npm package。
- v0.2 不发布 Docker image。
- v0.2 不把 Tauri GUI 当作稳定分发物。
- 分发渠道不得引入第二套 runtime 或绕过 `tessera-core`。
