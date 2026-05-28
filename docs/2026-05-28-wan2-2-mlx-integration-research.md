# Wan2.2 MLX 接入调研报告

**日期：** 2026-05-28  
**项目：** Moxin Studio / OminiX-API / OminiX-MLX  
**范围：** 调研 Wan2.2 视频生成模型接入 Moxin Studio 的可行性，重点关注 Apple Silicon 上的 MLX 本地推理路径。

---

## 结论摘要

Wan2.2 可以接入，但当前项目还没有一条可直接运行的 Wan2.2 链路。

Moxin Studio 侧已经具备一部分视频生成的 UI 和 registry 表达能力。当前 registry 中已经有 `Wan2.2 5B (4-bit MLX)` 条目，Chat 和 Model Hub 也已经会尝试调用 `/v1/videos/generations`。但是，这个 registry 条目目前只是 manual 占位，没有下载地址；OminiX-API 当前没有暴露视频生成路由；OminiX-MLX 当前也没有 `wan2-mlx` 模型 crate。

目前有两个现实方向：

1. **Rust MLX：** 在 OminiX-MLX 中实现原生 `wan2-mlx` crate，再通过 OminiX-API 暴露给 Moxin Studio。
2. **Python MLX：** 使用已有 Python `mlx-video` 实现作为 sidecar 或 subprocess 后端，再由 OminiX-API 包一层 `/v1/videos/generations`。

我的判断是：**Python MLX 是最快的 MVP 路径，Rust MLX 是长期产品化路径。** 如果目标是尽快验证 Moxin Studio 内的视频生成工作流，Python MLX 更务实。如果目标是维持项目“无 Python runtime / 原生本地推理”的架构承诺，并最终做成可发布的桌面产品能力，Rust MLX 才是最终方向，但它需要显著更多模型移植工作。

---

## 当前 Registry 状态

Moxin Studio 内置 registry 已经包含一个 Wan2.2 占位条目：

```json
{
  "id": "wan2.2-5b-4bit-mlx",
  "name": "Wan2.2 5B (4-bit MLX) — Coming Soon",
  "category": "video_gen",
  "source": {
    "kind": "manual",
    "revision": "main"
  },
  "storage": {
    "local_path": "~/.OminiX/models/wan2.2-5b/mlx_model_4bit",
    "size_bytes": 4000000000,
    "size_display": "~4 GB"
  },
  "runtime": {
    "api_type": "video_generation",
    "api_model_id": "wan2.2-5b-4bit-mlx",
    "memory_gb": 8.0,
    "supports_images": false,
    "supports_streaming": false,
    "quantization": "4bit"
  },
  "coming_soon": true
}
```

这个条目目前不能直接使用：

- `source.kind` 是 `manual`，没有 `repo_id`，也没有直接 `url`。
- `local_path` 只是一个约定路径：`~/.OminiX/models/wan2.2-5b/mlx_model_4bit`。
- 条目标记了 `coming_soon`。
- runtime model id `wan2.2-5b-4bit-mlx` 当前没有对应的 OminiX-API 后端实现。
- `~4 GB` 体积和 `8 GB` 内存估算偏乐观，应在真实转换和生成测试后替换为实测值。

作为参考，官方 `Wan-AI/Wan2.2-TI2V-5B-Diffusers` 仓库是公开且非 gated 的，但 Diffusers checkpoint 在转换或量化前约 34 GB。4-bit MLX 转换后可能小很多，但当前 registry 并没有指向任何已验证的预转换 MLX 模型仓库。

---

## 当前项目接入面

### Moxin Studio

Moxin Studio 已经有视频生成类别和部分 UI 流程：

- `RegistryCategory::VideoGen`
- `ApiType::VideoGeneration`
- `PanelType::VideoGeneration`
- Chat 支持 `VideoGen` 模式
- Model Hub 有 video panel
- 会调用 `POST http://localhost:8080/v1/videos/generations`
- 会从 base64 响应中保存 MP4 结果

也就是说，Studio 侧的接入面大体已经存在。缺口主要不在桌面 UI，而在运行时后端。

### OminiX-API

OminiX-API 当前更像编排层：

- HTTP 路由
- OpenAI-compatible 请求/响应 schema
- 模型 load/unload/status
- 单 inference thread 持有 MLX 模型，串行化 GPU 访问
- 分发请求到具体模型 engine

它不只是一个很薄的代理，但绝大多数模型具体逻辑来自 OminiX-MLX 的模型 crate。以图像生成为例，API 代码会 import 并协调 `flux-klein-mlx`、`zimage-mlx`、`qwen-image-mlx`。

当前缺口：

- 没有 `/v1/videos/generations` 路由。
- 没有 `VideoGenerationRequest` / `VideoGenerationResponse` 类型。
- 没有 `InferenceRequest::Video` 变体。
- 没有 `VideoEngine`。
- 没有 video 模型的 load/unload/status slot。
- 没有依赖任何 Wan 模型 crate。

### OminiX-MLX

OminiX-MLX 当前已有 LLM、ASR、TTS、VLM、OCR、图像生成等模型 crate，但没有 Wan 视频模型 crate。

相关已有 crate 包括：

- `flux-klein-mlx`
- `zimage-mlx`
- `qwen-image-mlx`
- `qwen3-mlx`
- `qwen3-vl-mlx`
- `qwen3-asr-mlx`
- `qwen3-tts-mlx`

当前缺口：

- 没有 `wan2-mlx` crate。
- 没有 Wan2.2 DiT、Wan2.2 VAE、视频 scheduler、视频导出 pipeline 的原生 Rust 实现。

---

## 外部生态调研

Wan2.2 官方资源：

- 官方仓库：<https://github.com/Wan-Video/Wan2.2>
- TI2V-5B Diffusers 模型：<https://huggingface.co/Wan-AI/Wan2.2-TI2V-5B-Diffusers>
- T2V-A14B Diffusers 模型：<https://huggingface.co/Wan-AI/Wan2.2-T2V-A14B-Diffusers>
- I2V-A14B Diffusers 模型：<https://huggingface.co/Wan-AI/Wan2.2-I2V-A14B-Diffusers>

官方 Wan2.2 模型族：

- `Wan2.2-T2V-A14B`：text-to-video，双模型 MoE，14B active parameters。
- `Wan2.2-I2V-A14B`：image-to-video，双模型 MoE，14B active parameters。
- `Wan2.2-TI2V-5B`：统一 text/image-to-video，dense 5B 模型，高压缩 Wan2.2 VAE。
- `Wan2.2-S2V-14B`：speech-to-video。
- `Wan2.2-Animate-14B`：角色动画/替换。

首个接入目标建议选择 `Wan2.2-TI2V-5B`，而不是 A14B：

- 更小。
- 单模型 pipeline，不是 high-noise / low-noise 双模型 MoE pipeline。
- 同时支持 text-to-video 和 image-to-video。
- 更接近 Apple Silicon 本地运行的现实目标。

现有 Python MLX 实现：

- 仓库：<https://github.com/Blaizzy/mlx-video>
- Wan2 README：<https://github.com/Blaizzy/mlx-video/blob/main/mlx_video/models/wan_2/README.md>

`mlx-video` 已经支持：

- Wan2.1 T2V
- Wan2.2 T2V-A14B
- Wan2.2 I2V-A14B
- Wan2.2 TI2V-5B
- 从原始 checkpoint 转换到 MLX
- 4-bit / 8-bit 量化
- 生成 MP4
- LoRA 支持

因此，Python MLX 是很强的风险验证路径。

---

## 方向一：原生 Rust MLX

### 含义

新增一个 OminiX-MLX crate，可能命名为 `wan2-mlx`，然后将其作为一等视频后端接入 OminiX-API。

这个 crate 需要承载真实模型实现：

- Wan2.2 model config 解析
- UMT5-XXL text encoder 支持，或复用/改造已有实现
- Wan DiT / transformer 实现
- 3D RoPE
- flow-matching scheduler
- Wan2.2 3D VAE encode/decode
- latent packing/unpacking
- TI2V/I2V 的可选图像条件输入
- sampling loop
- quantized layer loading
- 视频帧后处理
- MP4 导出，或将帧交给 API 层导出

OminiX-API 随后需要新增：

- `/v1/videos/generations`
- video request/response 类型
- `InferenceRequest::Video`
- `VideoEngine`
- video 模型动态 load/unload/status
- 与当前 Studio 调用兼容的 base64 MP4 响应

### 优点

- 保持项目核心承诺：原生、本地、无 Python runtime。
- 与当前 OminiX 架构最一致。
- 可以完整控制内存、进度、取消、模型生命周期。
- 实现完成后更容易随桌面应用签名和分发。
- 避免 Python 环境漂移和依赖安装问题。

### 缺点

- 实现成本高。
- 本质是移植一个完整视频扩散运行时，不只是加 registry。
- Apple Silicon 统一内存下需要细致的内存策略。
- 真实模型文件的转换和量化流程也需要解决。
- A14B 比 TI2V-5B 复杂很多，因为涉及双模型 MoE pipeline。

### 工作量估计

针对聚焦的 `Wan2.2-TI2V-5B` MVP：

- **3-5 人周**：粗糙内部 prototype。前提是以 `mlx-video` 为参考、限制为 text-to-video、固定参数、使用预转换 MLX 权重。
- **6-10 人周**：较稳定实现，包含更好的错误处理、内存处理、转换假设验证和 API 集成。

针对更完整的产品化实现：

- **2-4 个月**：如果包含转换、4-bit 量化、I2V、progress/cancel、VAE tiling、registry polish 和稳健打包。
- 如果再包含 A14B 双模型 MoE、LoRA/Lightning 支持，工作量会继续增加。

### 判断

如果 Moxin/OminiX 希望 Wan2.2 像现有图像、音频、LLM 模型一样成为原生能力，这是正确的长期方向。

但它不应该被当成一个“快速接入”任务。它是一个模型移植项目。第一个 milestone 应该收窄：`Wan2.2-TI2V-5B`、预转换 MLX 权重、固定 704x1280/1280x704 输出、固定帧数、暂不做 A14B。

---

## 方向二：Python MLX Sidecar

### 含义

使用已有 `mlx-video` Python 实现，并将其包装在 OminiX-API 后面。

可能有两种形态：

1. **Subprocess 后端：** OminiX-API 通过 `python -m mlx_video.models.wan_2.generate` 拉起生成进程。
2. **本地 sidecar service：** OminiX-API 启动或连接一个小型 Python HTTP 服务，由它持有模型和生成队列。

API 对 Moxin Studio 仍然暴露同一个 endpoint：

```http
POST /v1/videos/generations
```

Moxin Studio 不需要知道后端是 Rust MLX 还是 Python MLX。

### 优点

- 最快让当前 Studio 视频 UI 生成真实 MP4。
- 使用已经支持 Wan2.2 TI2V/T2V/I2V 的实现。
- 已包含转换和量化工具。
- 适合在投入 Rust 移植前测量真实内存、耗时、模型体积和 UX 约束。
- 可以快速验证模型选择和默认生成参数。

### 缺点

- 违背“无 Python runtime”的产品承诺。
- 增加桌面应用打包复杂度。
- 需要管理 Python 环境、依赖、模型转换脚本。
- 如果使用 subprocess，每次启动可能较慢；除非 sidecar 常驻并保持模型 warm。
- 错误处理、进度、取消、GPU 内存释放会比原生 engine 更脆弱。
- 长期维护依赖外部 Python 项目。

### 工作量估计

proof of concept：

- **3-7 天**：前提是模型文件已转换，且允许本地 Python 依赖。

可用的内部 MVP：

- **1-2 周**：新增 API endpoint、subprocess/sidecar wrapper、输出处理、基础进度日志、registry 调整。

可分发功能：

- **3-6 周**：因为真正麻烦的是 Python 打包、依赖安装、转换流程管理和升级维护。

### 判断

这是最好的短期验证路径。它能快速回答最重要的不确定性：

- 本地 Apple Silicon Mac 能否生成可接受的 Wan2.2 视频？
- 什么参数现实可用？
- 峰值统一内存是多少？
- 短视频生成需要多久？
- registry 应该描述什么模型格式？
- 当前 Moxin 视频 UI 是否需要进度、缩略图、取消和队列？

我建议把 Python MLX 作为受控 prototype 路径，但除非产品愿意放松“pure Rust / no Python runtime”的定位，否则不应把它作为最终默认实现。

---

## 推荐路线

### Phase 0：先用 Python MLX 在本机证明 Wan2.2 能跑

目标：在 Moxin 外部使用 `mlx-video` 生成一个 MP4。

任务：

- 在本地开发环境安装 `mlx-video`。
- 下载 `Wan-AI/Wan2.2-TI2V-5B`。
- 转换到 MLX。
- 量化到 4-bit。
- 使用保守参数生成短视频。
- 记录：
  - 转换后模型体积
  - 峰值统一内存
  - 生成耗时
  - 输出质量
  - 可工作的命令

这一步应该早于任何 Rust 模型移植工作。

### Phase 1：用 Python MLX 做 API Prototype

目标：让 Moxin Studio 现有视频 UI 通过 OminiX-API 生成 MP4。

任务：

- 在 OminiX-API 中新增 `/v1/videos/generations`。
- 新增 video request/response 类型。
- 实现 subprocess 或 sidecar video backend。
- 返回 OpenAI 风格响应：`data[0].b64_json`。
- 更新 registry 条目，指向真实本地模型布局；或继续保持 manual，但移除误导性的体积/内存估算。
- 增加基础 timeout 和错误报告。

这能在不等待原生 Rust port 的情况下形成一个可演示产品流程。

### Phase 2：原生 Rust `wan2-mlx` Spike

目标：确认原生 Rust 路径技术上可行。

任务：

- 创建小型 `wan2-mlx` crate。
- 加载已转换的 TI2V-5B MLX config 和 safetensors。
- 实现足够的 transformer 和 VAE 路径，跑一个最小 smoke test。
- 与 `mlx-video` 对比 tensor shape 和单步输出。
- 决定是否进入完整 port。

### Phase 3：原生 Rust 产品化路径

目标：用一等 OminiX-MLX 后端替换 Python 后端。

任务：

- 完成 `wan2-mlx`。
- 加入 quantized loading。
- 加入 VAE tiling / 内存控制。
- 加入 progress/cancel。
- 加入动态 load/unload。
- 为已验证模型文件加入 registry 下载支持。
- 保留 Python MLX 作为开发/参考路径，而不是最终默认路径。

---

## 最终建议

先用 **Python MLX** 证明 Moxin Studio 工作流并收集真实性能数据。不要一开始就盲目投入 `wan2-mlx`。

当可工作的命令、模型布局、内存边界和 UX 预期都明确后，如果 Wan2.2 是 Moxin/OminiX 的战略功能，再推进 **Rust MLX** 路径。

简而言之：

- **Python MLX：** 最适合 MVP、测量和风险验证。
- **Rust MLX：** 最适合最终产品质量和架构一致性。

当前 registry 条目只能视为占位。除非已经有经过验证的预转换 MLX 仓库，或已经有可工作的转换/下载流程，否则不应把它呈现为可直接下载和运行的模型。
