# NetEase Toolkit

一款基于 Tauri v2 构建的桌面工具，主要用于**将歌词和封面嵌入音频文件**，让歌曲在 Apple Music 等播放器中完整展示封面和歌词。

Apple Music多端同步方法：将音频文件拖动到Mac/Win Apple Music的资料库-歌曲，上传完毕后后自动多端同步


## 功能

### 音频封面 / 歌词嵌入

将下载或已有的音频文件、封面图片、LRC 歌词文件放到同一目录下，确保文件名一致后，通过「格式转换」功能将封面和歌词嵌入到音频文件中：

- 支持的音频格式：MP3、M4A（AAC）、FLAC
- 封面支持：`.jpg` / `.png`
- 歌词支持：`.lrc` 文件（Apple Music 目前仅展示静态歌词，时间戳不会解析）
- 自动匹配：同名文件自动配对，无需手动指定

<img width="1167" height="812" alt="image" src="https://github.com/user-attachments/assets/5199bbaa-555c-4f46-95b9-9cb59d797e7e" />


### 网易云音乐下载

- 通过**二维码登录**网易云账号
- 搜索歌曲、歌手、歌单（链接）、专辑（链接）
- 可选音质：标准、极高、无损、Hi-Res、环绕声、高清环绕、超清母带
- VIP 歌曲需登录后下载
- **降音质重试**：高音质不可用时自动回退到较低音质
- 下载时同步保存封面和 LRC 歌词到本地

<img width="1684" height="1243" alt="image" src="https://github.com/user-attachments/assets/13cc8a3c-1653-4e1b-95f8-d4731ddf8504" />


### 格式转换

在本地音频文件之间转换格式（如 FLAC → MP3、FLAC → M4A），同时保留封面的歌词嵌入能力。

**注意**：格式转换涉及重新编码，需要一定的处理时间。同一格式的转换（例如 MP3 → MP3）仅复制文件并嵌入元数据，速度更快。

## 使用方式

### 1. 准备工作

按照以下结构将文件整理到同一个目录：

```
Music/
├── song.mp3            # 音频文件
├── song.jpg            # 封面图片（与音频同名）
└── song.lrc            # 歌词文件（与音频同名）
```

### 2. 格式转换

1. 打开应用，进入「本地」页面
2. 选中需要处理的歌曲
3. 点击「格式转换」按钮
4. 选择输出格式（MP3 / M4A），勾选「嵌入封面」和「嵌入歌词」
5. 点击「开始转换」

转换完成后，输出目录中的音频文件已包含封面和歌词，将其导入 Apple Music 即可显示。

<img width="1684" height="1243" alt="image" src="https://github.com/user-attachments/assets/0f4a5cc9-70f7-41d6-9a61-0a82fa3d41aa" />


## macOS 使用说明

由于应用未进行 Apple 开发者签名，首次打开时会触发 macOS 的安全拦截。请执行以下命令移除隔离属性：

```bash
xattr -cr /Applications/NetEase\ Toolkit.app
```

如果提示应用已损坏，请运行：

```bash
sudo xattr -d com.apple.quarantine /Applications/NetEase\ Toolkit.app
```

## 技术栈

| 层 | 技术 |
| --- | --- |
| 前端框架 | Vue 3 + TypeScript |
| 状态管理 | Pinia |
| 路由 | Vue Router |
| UI 样式 | Tailwind CSS |
| 图标 | Lucide |
| 桌面框架 | Tauri v2 (Rust) |
| 音频编码 | LAME (MP3), FDK-AAC (M4A) |
| 标签写入 | ID3 (MP3), mp4ameta (M4A) |
| 音频解码 | Symphonia |
| MP4 容器 | mp4 crate |

## 项目结构

```
netease-toolkit/
├── src/                    # Vue 前端
│   ├── components/         # 通用组件（AppHeader, Toast, ConvertModal 等）
│   ├── stores/             # Pinia 状态（auth, local, download, convert, settings）
│   ├── views/              # 页面（Local, Download, Settings）
│   ├── router.ts           # 路由配置
│   ├── main.ts             # 入口
│   └── App.vue             # 根组件
├── src-tauri/              # Rust 后端
│   └── src/
│       ├── lib.rs          # Tauri 命令注册、下载、登录、扫描等
│       ├── converter.rs    # 音频转码 & 元数据嵌入
│       ├── main.rs         # 应用入口
│       ├── commands/
│       │   └── api.rs      # 网易云 API 封装
│       └── utils/
│           └── crypto.rs   # AES 加解密工具
├── scripts/                # 辅助脚本
└── package.json
```

## 构建

```bash
# 安装前端依赖
npm install

# 开发模式
npm run tauri dev

# 构建发布版本
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/` 目录下。
