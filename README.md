# Battery SipJuice

Battery SipJuice 是一个面向 Linux 笔记本和平板的本机电池监控与电源状态工具。它使用 Tauri v2 + Rust + 原生 Web 前端构建，主要从 Linux `sysfs` / `procfs` 读取电池、电源输入和进程 CPU 时间数据。

> **声明**
>
> 这个项目首先是我给自己设备使用的小工具。它是开源的，也欢迎参考、试用和改进；但它不是一个已经覆盖大量硬件的通用商业软件。不同 Linux 发行版、内核、固件和硬件暴露的电源接口差异很大，请把它当作一个仍在演进中的个人开源项目使用。

## Features

- **电池概览**：电量、充放电状态、剩余使用时间、实时功率、温度。
- **多电池支持**：自动枚举所有在位电池，可切换查看每块电池的概览、健康与独立历史。
- **电池健康**：实际满电容量、设计容量、容量损耗、循环次数、SOH、内阻、电池技术。
- **历史曲线**：支持电池侧和输入侧切换，查看电量、功率、电压、电流、温度等趋势。
- **输入电源监测**：显示系统暴露的 USB-C、AC、无线充等输入源状态、功率、电压、电流。
- **输入电量估算**：按输入功率随时间积分，估算当前窗口内的输入电能。
- **应用耗电估算**：按进程 CPU 时间占比分配电池功率，估算应用当前功率和本次运行累计耗电。
- **电池养护提醒**：低电量提醒、高电量拔电提醒，纯软件通知，不改写充电策略。
- **桌面集成**：系统托盘、开机自启动、静默启动、关闭按钮行为设置、浅色/深色主题。

## Platform Support

当前目标平台是 Linux 桌面环境：

- 架构：`aarch64/arm64`、`x86_64/amd64`
- 包格式：`.deb`、`.rpm`、AppImage
- 主要依赖：WebKitGTK 4.1、Tauri v2、Rust、Node.js

这个项目依赖 Linux 内核暴露的标准或半标准接口。支持情况不是由 CPU 架构单独决定，而是由设备是否提供对应的 `sysfs` 节点决定。

## Data Sources

Battery SipJuice 当前主要读取这些本机接口：

- `/sys/class/power_supply/*`
  - 电池容量、状态、电压、电流、功率、温度、健康信息
  - USB-C、AC、无线充等输入源状态
- `/proc`
  - 进程 CPU 时间，用于应用耗电估算

## Important Limits

- **硬件兼容性不保证**：很多字段是否存在取决于内核、固件和驱动。
- **输入侧只支持 `power_supply` 暴露的设备**：普通 USB 外设，例如 2.4G 接收器，只有在系统把它暴露为电源输入源时才会出现在输入侧曲线中。
- **应用耗电不是硬件精确测量**：它是基于电池总功率和 CPU 时间占比的估算，不代表真实逐应用功耗计量。
- **无电池主机可运行但功能会降级**：没有电池时，电池健康、剩余使用时间、应用耗电估算等功能可能为空；输入侧数据取决于主机是否暴露电源传感器。

## Install From Source

### System Dependencies

Debian / Ubuntu 系发行版：

```bash
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev patchelf rpm
```

还需要安装：

- Rust toolchain: <https://rustup.rs/>
- Node.js

安装前端工具依赖：

```bash
npm install
```

### Development

```bash
npm run dev
```

### Build

构建 Tauri 默认目标：

```bash
npm run build
```

只构建当前机器架构的 Debian 包：

```bash
npm run build:deb
```

只构建当前机器架构的 RPM 包：

```bash
npm run build:rpm
```

同时构建当前机器架构的 `.deb` 和 `.rpm`：

```bash
npm run build:linux:packages
```

在 x86_64 Linux 构建机上生成 x64 `.deb` / `.rpm`：

```bash
npm run build:linux:x64
```

Tauri 的 Linux 包是原生构建产物。ARM64 机器适合构建 ARM64 包；x64 包建议在 x86_64 Linux 构建机或 CI 上构建。仓库包含 GitHub Actions 工作流，可用于生成 x64 deb/rpm artifact。

## Versioning

项目内有多个生态的版本字段。推荐使用脚本同步更新：

```bash
npm run version:bump -- 0.4.1
```

这个命令会同步 `package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 和 `src-tauri/tauri.conf.json`。

## Development Notes

- 前端是无框架的 HTML/CSS/JavaScript，Tauri 直接加载 `src/`。
- 后端命令、采样和系统接口读取在 `src-tauri/src/`。
- 历史数据按电池和输入源分别使用固定大小的 RRD 风格归档，避免单份历史无限增长；旧单电池历史会自动迁移。
- 电池历史和输入侧历史分开保存，已有电池历史文件保持兼容。

## Contributing

这个项目仍以个人需求为主，但欢迎提交 issue、建议和补丁。比较有价值的方向包括：

- 补充不同 Linux 设备上的 `power_supply` 兼容性信息。
- 改进无电池主机、扩展坞、USB-C 供电和多电池设备的展示。
- 改进应用耗电估算的分组和命名。
- 补充截图、文档和发行包说明。

提交改动前建议运行：

```bash
node --check src/main.js
cd src-tauri
cargo test
cargo clippy -- -D warnings
```

## License

Battery SipJuice is licensed under the [MIT License](LICENSE).
