# 电源助手 (Battery Assistant)

一个 Linux 电池监控与健康管理小工具。基于 Tauri v2 + Rust + 原生 Web，直读 Linux `power_supply` sysfs 接口，无任何网络上报。

> ⚠️ **说明**：这是一个仅供我个人使用的小工具，功能并不完善，且只在我自己的一台设备上做过测试。其它设备上的表现未经验证，请自行斟酌使用。

## 功能

| 模块 | 内容 |
|------|------|
| **概览** | 环形电量、状态、剩余时间、功率、温度 |
| **健康** | 实际/设计容量、损耗、循环次数、SoH、内阻 |
| **电源** | 实时电压/电流/OCV、USB/无线充电输入状态 |
| **充电控制** | 充电阈值封顶（实验性，见下） |

### ⚠️ 充电控制为实验性功能

部分设备通过 `charge_control_{start,end}_threshold` sysfs 接口暴露充电阈值，但：

1. 写入是否被固件真正执行因设备而异，可能写进去却不生效；
2. 该 sysfs 文件通常属 root 所有，普通用户无写权限，应用时需提权（pkexec/sudo）或预置 udev/polkit 规则。

应用如实呈现这些限制，写入失败不会谎称成功。

## 构建

### 安装系统依赖

```bash
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
```

还需要 [Rust 工具链](https://rustup.rs/) 和 Node.js。安装依赖：

```bash
npm install
```

### 开发运行

```bash
npm run dev
```

### 打包（生成 .deb / AppImage）

```bash
npm run build
```

## 许可证

[MIT](LICENSE)
