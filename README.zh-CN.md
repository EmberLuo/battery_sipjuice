<p align="center">
  <img src="src/app-icon.svg" width="112" alt="Battery SipJuice 图标">
</p>

<h1 align="center">Battery SipJuice</h1>

<p align="center">
  一款面向 Linux 笔记本和平板的本地优先电池监控与养护工具。
</p>

<p align="center">
  <a href="README.md">English</a> · <strong>简体中文</strong>
</p>

<p align="center">
  <a href="https://github.com/EmberLuo/battery_sipjuice/actions/workflows/linux-x64-packages.yml"><img src="https://github.com/EmberLuo/battery_sipjuice/actions/workflows/linux-x64-packages.yml/badge.svg" alt="Linux 安装包构建"></a>
  <img src="https://img.shields.io/badge/platform-Linux-FCC624?logo=linux&logoColor=black" alt="Linux">
  <img src="https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri&logoColor=white" alt="Tauri v2">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-2ea44f" alt="Apache License 2.0"></a>
</p>

## 截图

<p align="center"><sub>使用当前源码和隔离的演示数据渲染。</sub></p>

<p align="center">
  <img src="docs/screenshots/overview.png" width="900" alt="Battery SipJuice 概览">
</p>

<table>
  <tr>
    <td><img src="docs/screenshots/monitor.png" alt="电池监测历史"></td>
    <td><img src="docs/screenshots/sessions.png" alt="充电会话历史"></td>
  </tr>
  <tr>
    <td align="center">监测历史</td>
    <td align="center">充电会话</td>
  </tr>
</table>

<p align="center">
  <img src="docs/screenshots/health.png" width="900" alt="长期电池健康趋势">
</p>

## 项目简介

Battery SipJuice 是一款仅面向 Linux 的桌面工具，用于了解电池状态、充电行为和长期用电情况。它读取 Linux `power_supply` 和 `procfs` 提供的标准或厂商扩展数据，并通过轻量的 Tauri 界面进行展示。

所有监测数据都保存在本机。Battery SipJuice 不需要账号、不上传遥测数据，也不会修改固件的充电策略。

> 可用指标取决于设备的 Linux 内核、固件和驱动。硬件没有提供的读数会显示为不可用，而不会通过猜测生成。

## 主要功能

- **实时电池监测**：查看电量、状态、剩余时间、功率、电压、电流和温度。
- **多电池支持**：分别查看每块在位电池及其独立历史和健康记录。
- **七天历史曲线**：使用固定大小的多分辨率归档记录电池和输入电源数据，存储空间不会无限增长。
- **充电会话记录**：自动记录充电区间、持续时间、Wh/mAh 估算、功率、温度和输入来源。
- **长期健康趋势**：每天及充电结束后保存健康快照，展示平滑后的容量健康曲线和每周期磨损估算。
- **输入电源监测**：显示 Linux `power_supply` 暴露的 USB-C、交流电源和无线充电输入。
- **按应用耗电估算**：根据进程 CPU 时间占比分配电池放电功率，适合相对比较，不等同于硬件精确计量。
- **电池养护提醒**：支持低/高电量、高/低温度和持续异常耗电通知。
- **Linux 桌面集成**：系统托盘、开机自启动、静默启动、单实例运行和可配置的关闭行为。
- **个性化界面**：中英文、浅色/深色/跟随系统主题，以及自选或跟随系统的强调色。

## 从源码构建

### 环境要求

- 带有 WebKitGTK 4.1 的 Linux
- [Rust](https://rustup.rs/)
- Node.js 和 npm

Debian 或 Ubuntu 可以使用以下命令安装本机构建依赖：

```bash
sudo apt update
sudo apt install -y \
  build-essential curl file libayatana-appindicator3-dev \
  libjavascriptcoregtk-4.1-dev librsvg2-dev libsoup-3.0-dev \
  libssl-dev libwebkit2gtk-4.1-dev libxdo-dev patchelf rpm wget
```

### 开发运行

```bash
git clone https://github.com/EmberLuo/battery_sipjuice.git
cd battery_sipjuice
npm install
npm run dev
```

### 构建安装包

```bash
npm run build
```

默认命令会构建 Tauri 配置中的目标（`.deb` 和 AppImage）。也可以使用以下命令：

| 命令 | 产物 |
| --- | --- |
| `npm run build:deb` | Debian 安装包 |
| `npm run build:rpm` | RPM 安装包 |
| `npm run build:linux:packages` | Debian 和 RPM 安装包 |
| `npm run build:linux:x64` | x86_64 Debian 和 RPM 安装包 |

构建产物通常位于 `src-tauri/target/release/bundle/`；显式指定目标架构时位于 `src-tauri/target/<target-triple>/release/bundle/`。所有打包命令都会先根据 `src/app-icon.svg` 重新生成应用所需的 PNG 图标。

## 许可证

Battery SipJuice 使用 [Apache License 2.0](LICENSE) 发布。再分发时必须按照许可证要求保留许可证文本、适用的版权与归属声明，以及项目的 [NOTICE](NOTICE) 文件。
