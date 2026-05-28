# Qwen-Image-Edit-2511 接入计划（OminiX-MLX / OminiX-API / Moxin Studio）

> **For agentic workers:** REQUIRED SUB-SKILL: 用 superpowers:subagent-driven-development 或 superpowers:executing-plans 逐任务执行。步骤用 `- [ ]` 勾选框追踪。
> **注：** 文件名带 `sdcpp` 是历史原因；本文档现覆盖**两条路线**，sd.cpp 只是其中之一（备选）。

**目标：** 在 16GB M4 Mac 上，于 Moxin Studio 里下载 `qwen-image-edit-2511`，上传图片 + 编辑提示词，本地推理返回编辑后图片。

**两条路线 + 决策门：**
- **Route A（MLX 原生）：** 修 OminiX-MLX 两个 primitive 缺陷 + 用 Q4_0 GGUF（设 group_size 32）→ 复用 API 已写好的编排链路。改动小、纯 MLX、速度快；但有几个必须实测的未知点。
- **Route B（sd.cpp 子进程，备选/兜底）：** 在 OminiX-API 加一个调 `stable-diffusion.cpp` 的子进程引擎。零 MLX-crate 改动、原生吃任意 k-quant、有 CPU offload、最可能"真的在 16GB 上出图"；代价是外部二进制 + 两套推理栈。
- **先跑"决策门"（半天，见 §3）** 再决定主走哪条，不要预先押注。

---

## 0. 本版修订说明（相对早期 sd.cpp-only 版）

本版基于对 `OminiX-MLX` / `OminiX-API` / `Moxin Studio` 三仓库的**代码逐行核查** + 对候选权重文件的**在线头部核查**（HTTP Range，不下整模型），修正了早期版本的几处误判：

1. **真正的阻塞不是"找对模型文件"，而是 `QwenVAE` 的 encoder/quant_conv 根本没被加载**（见 §2 Defect 1）。API 的编排其实已端到端写完。
2. **正确的 VAE 源是 `mlx-community/qwen-image-edit-2511-8bit` 的 `vae/0.safetensors`**（key 与 crate 逐字匹配），不是早期以为的 Comfy-Org 单文件（那个是 **sd.cpp** 的正确 VAE，不是 MLX crate 的）。
3. **`group_size` 不是死路。** 它是 `QwenConfig` 的普通字段，你拥有 crate，可改/可自动判定为 32 以吃 Q4_0。早期把它当硬墙是错的。
4. **Q4_0 路线免离线转换。** 实证 `qwen-image-edit-2511-Q4_0.gguf` 无 k-quant、~11.85GB 量化态、可直接下载，不需要大内存机器做转换。
5. **K-quant 一律排除**（Q2_K/Q3_K_M/Q4_K_M…）：MLX `load_gguf` 会把它们反量化成 f16 → ~40GB OOM（§2 实证）。

---

## 1. 架构原则（总纲，先读）

三个项目分工在所有模型上一致，image-edit 不是特例：

```
mlx-rs / mlx-rs-core          ← 张量框架（≈ libtorch）
   ↑
qwen-image-mlx（及 flux-klein-mlx, zimage-mlx, qwen3-mlx ...）
   ↑                            ← 每模型一个 crate：模型结构 + 层 + 权重加载器 + scheduler/rope/pack 等数学 helper
OminiX-API/engines/*.rs        ← 编排层：tokenize、去噪循环、CFG、scheduler 接线、VAE 前后处理、内存调度、HTTP
```

**两条铁律（所有 phase 遵守）：**
> **A. 原语归 MLX crate**（模型结构、层、权重加载器、scheduler/rope/pack）。
> **B. 编排归 API**（循环、CFG、调度接线、VAE 前后处理、内存、HTTP）。

**不要把去噪循环搬进 MLX crate。** crate 里自带循环的 `QwenImagePipeline` 是无人使用的孤儿代码——API 自己重写了循环（`image.rs` 里 `generate_flux`/`generate_qwen_image`/`generate_qwen_image_edit`/`generate_zimage` 各自跑循环）。

---

## 2. 现状与阻塞（代码核查，路径在 `/Users/alan0x/Documents/projects/OminiX-API` 与 `OminiX-MLX`）

### 2.1 API：`generate_qwen_image_edit` 编排已端到端写完（非桩）

`image.rs:1068-1280` 是完整链路：参考图解码→`qwen_vae.encode()`（~564-595）；正/负 prompt 编码（1084-1117）；CFG 批处理（1119-1149）；参考 latent packing（1153-1161）；edit RoPE（1167-1173）；scheduler/noise（1176-1185）；去噪循环（`forward_edit`→CFG→`scheduler.step`，每步清 cache，1192-1235）；unpack→denorm→`qwen_vae.decode()`→PNG（1237-1266）。
且**已接 HTTP**：edit 走 `/v1/images/generations` 带 `image` 字段 → `generate()`（517-519）检测到 `request.image` 即走 img2img → 分发 `generate_qwen_image_edit`（619）。**无需新增 `/v1/images/edits` 才能跑。**

### 2.2 Defect 1（致命，真正阻塞）：`QwenVAE` encoder/quant_conv 未加载

`qwen-image-mlx` 当初为文生图写，只需 decode 路径。`load_vae_weights`（`vae/weights.rs:14-25`）**只加载 `post_quant_conv` 和 `decoder`**，没有 encoder / quant_conv 的加载代码。但 `QwenVAE` struct 有 `encoder`/`quant_conv` 字段和 `encode()` 方法（`vae/vae.rs:215,221,286`）。于是 API 调 `qwen_vae.encode()` 时 **encoder 跑在随机初始化权重上 → 产出垃圾参考 latent**。这是 edit 此刻就坏的实锤 bug，与任何模型文件无关。

| VAE 组件 | 方向 | 生图需要 | 编辑需要 | 加载状态 |
|---|---|---|---|---|
| encoder | 图→latent | ✗ | ✓ | **缺** |
| quant_conv | encoder 后 1×1 投影 | ✗ | ✓ | **缺** |
| post_quant_conv | decoder 前 | ✓ | ✓ | 已加载 |
| decoder | latent→图 | ✓ | ✓ | 已加载 |

**已核查**：`mlx-community/qwen-image-edit-2511-8bit` 的 `vae/0.safetensors` 的 key 与 `vae/weights.rs` 期望**逐字一致**（含独特后缀 `.conv3d.weight`，及 `encoder.conv_in`/`decoder.conv_in`/`post_quant_conv`/`quant_conv`）。所以 decoder 本就对得上，Phase A1 的实质工作收敛为**补上 encoder + quant_conv 的加载调用**（key 会完美匹配），不是重写 mapper。

### 2.3 Defect 2：权重加载静默

`load_transformer_weights`（`qwen_quantized.rs:1002-1058`）末尾 `model.update_flattened(...)` + `Ok(())`。`update_flattened` 静默尽力匹配：文件多余 key 悄悄丢、模型缺的参数悄悄保留随机初值，都不报错。"加载 Ok" ≠ "权重真对上"。`load_text_encoder`、`load_vae_from_dir` 同理。→ Phase A2 加 strict 报告。

### 2.4 量化档与 MLX 支持度（决定选哪个文件）

`Array::load_gguf`（`OminiX-MLX/mlx-rs/ops/io.rs:86` → `utils/io.rs:61` 调 MLX C++ `mlx_load_gguf`），被 `image.rs:864` 调用：
- **MLX 原样保留量化（uint32+scales+biases）的只有 `Q4_0`/`Q4_1`/`Q8_0`**。`qwen_quantized.rs:1007` 专门检测 `dtype==Uint32` 的 `.weight` remap 成 `.inner.weight`——反证代码就预期这些类型返回量化权重。
- **K-quant（`Q2_K`/`Q3_K`/`Q4_K_M`/`Q5_K`/`Q6_K`）不被 MLX 原生量化，只能反量化成 f16** → ~20B transformer 膨胀 ~40GB → 16GB 必爆。
- **实测铁证**：注册表里同一文生图模型 `qwen-image-2512-4bit`（MLX 原生 4bit）标 **16GB**，`qwen-image-2512-q4km`（GGUF Q4_K_M）描述 **"~24 GB peak"**——多出的 ~8GB 正是 K-quant 被反量化。
- **拍板检查（5 分钟硬门槛）**：加载任一档 GGUF 后打印一个 `.weight` 的 `dtype`——`Uint32`=量化保留可用，`Float16`=已反量化将 OOM。

**Q4_0 文件实证（在线头部解析 `unsloth/.../qwen-image-edit-2511-Q4_0.gguf`）：** 1934 张量，`general.architecture=qwen_image`，张量名为 diffusers 约定（`transformer_blocks.N.attn.to_q/add_k_proj/...`）；量化构成 **Q4_0×720 + Q4_1×120 + F32×1088 + BF16×6，无任何 k-quant** → MLX 保留量化、不 OOM。**但 Q4_0/Q4_1 都是 32 元素块（GGML `QK4_0=32`），而 `QwenConfig.quantization_group_size` 写死 64**（见下，可改）。

### 2.5 `QwenConfig` 与 group_size（可改，非死路）

`QwenConfig`（`qwen_quantized.rs:19-42`）：`num_layers:60`、`quantization_group_size:64`、`quantization_bits:4`(default)/`8`(`with_8bit()`)。`quantized_linear()`（:63-66）用这两个字段建所有 `QuantizedLinear`。API 按文件名选 bits（`image.rs:832` `if contains("8bit")`）。
- **group_size 是普通字段**，MLX `QuantizedLinear` 支持 32/64/128。要吃 Q4_0（块 32），把该模型的 `group_size` 设/自动判定为 32 即可（一行级改动）。**这不是"和 8bit 冲突"的难题**：16GB 上不会同时加载 group64 的 8bit；要兼容也只需在已有的按文件名分流处加一支，或直接从 GGUF 量化类型自动推 group_size。

### 2.6 Moxin Studio：注册表地址已失效
注册表 `qwen-image-edit-2511-q4km`（**Q4_K_M GGUF**，`models_registry.json:609-624`）的 calcuis URL **401 失效**；即便复活，Q4_K_M 是 k-quant，会反量化 OOM。客户端走 `/images/generations`（`ominix_image_client.rs:219`），与 API img2img 入口一致。`image_edit` panel 已存在。→ 待模型层验证通过再改（Phase 5 / §6）。

---

## 3. 决策门（先做，半天，决定主走哪条）

目的：用最低成本判定 Route A（MLX）是否走得通；走不通就走 Route B（sd.cpp）。

### Task G1：Q4_0 dtype + group_size 兼容性
**Files:** 临时改 `OminiX-MLX/qwen-image-mlx/src/qwen_quantized.rs`（group_size）+ 加一行 dtype 打印

- [ ] **Step 1:** 下载 `unsloth/Qwen-Image-Edit-2511-GGUF` → `qwen-image-edit-2511-Q4_0.gguf` 到 `~/.OminiX/models/qwen-image-edit-2511/`。
- [ ] **Step 2:** 在 `load_transformer_weights` 里临时打印第一个 `.weight` 的 `dtype` 和（若量化）其 scales 的形状。期望：`Uint32`（量化保留）。若 `Float16` → MLX 反量化了，停，走 Route B。
- [ ] **Step 3:** 把该模型的 `QwenConfig.quantization_group_size` 设为 32（或从 gguf 量化类型自动判定），加载 transformer。期望：无 shape mismatch、`QuantizedLinear(group32,bits4)` 吃下 Q4_0/Q4_1。**这是唯一只能实测、不能在线证的点。** 通过→Route A 可行；报形状错且改 group_size 仍不行→Route B。

### Task G2：VL 文本编码器 strict-load
- [ ] **Step 1:** 下 `unsloth/Qwen2.5-VL-7B-Instruct-GGUF` 的一个 GGUF，过 `load_text_encoder`，开 strict（见 Phase A2）看 missing/shape-mismatch 是否为空。空→可用；否则需换源或改 mapper（VL 是最大未验证项）。

**门控结论：** G1+G2 都过 → 走 **Route A**（更省、纯 MLX）。任一卡死 → 走 **Route B**（已验证能跑）。无论走哪条，Defect 1（VAE encoder 加载）在 Route A 下都必须修。

### 决策门执行结果（2026-05-28，已实测）

用 Python `mlx` 0.31.1 对真实下载的文件核查，无需编译 ominix-api：

- **G1 = ✅ 通过。** `mx.load(qwen-image-edit-2511-Q4_0.gguf)`：`transformer_blocks.0.attn.add_k_proj.weight` dtype=**`uint32`（量化保留、未反量化、不 OOM）**；全模型 840 uint32 + 241 f32 + 6 f16。标准三件套（`.weight` uint32 `[3072,384]` + `.scales`/`.biases` f16 `[3072,96]`）。**group_size = 3072/96 = 32**（全模型一致）→ 需把 `QwenConfig.quantization_group_size` 设/自动判定为 32（小改）。transformer 加载可行。
- **G2 = ⚠️ 需改 crate（非死路，已有现成量化 VL）。**
  - GGUF 版 VL（unsloth）❌:张量名 llama.cpp 风格（`token_embd`/`blk.N.attn_q`/`blk.N.ffn_gate`/`output_norm`），与 crate 期望 `encoder.*` 零重叠、无重映射、静默加载 → 跑随机权重；且 crate 文本编码器是全精度 plain Linear（mapper `*layer..q_proj.weight = w`，不碰 `.scales/.inner`），吃不了量化 GGUF。
  - 但核查发现两个现成 MLX 文件:**(a)** `mlx-community/qwen-image-edit-2511-8bit` 的 `text_encoder/` 命名与 crate 期望（`encoder.embed_tokens.weight`/`encoder.layers.N.self_attn.q_proj.weight`/`encoder.norm.weight`）**逐字一致**，但全精度 ~15.6GB（太大）;**(b)** `mlx-community/Qwen2.5-VL-7B-Instruct-4bit` 已是 **MLX 4bit ~4.5GB**，实测 shape 与 `TextEncoderConfig::default()` **完全吻合**（q_proj `weight[3584,448]`+`scales[3584,56]`、k_proj `[512,448]`、gate_proj `[18944,448]`、hidden 3584/28 层/GQA 4kv/intermediate 18944、**group_size=64**），但命名是 `language_model.model.*`。
  - **所以 G2 解法（已验证可行）= ① 前缀改名 `language_model.model.` → `encoder.`（丢 `visual.*`/`lm_head`）+ ② crate 文本编码器改用 `QuantizedLinear`（group_size 64、bits 4）+ loader 处理 `.scales/.biases`。量化本身无需自己做。** 配 staging（编码 prompt 后释放编码器），4.5GB 编码器 + 11GB transformer 在 16GB 宽裕。

**最终判定:** G1 ✅;G2 不是死路。**Route A 总计 3 处 crate 改动**:① VAE encoder/quant_conv 加载（Defect 1，致命，§2.2/Phase A1）② transformer `group_size`→32（Phase A3）③ 文本编码器 `QuantizedLinear` + 前缀改名（Phase A3b）。**两条路对 16GB 都可行,是真实权衡** —— Route A = 上述 3 处工程量、纯 MLX 架构、装得下、快;Route B（sd.cpp）= 零 crate 改动、把握最高,代价是外部二进制 + 两套栈。按"愿不愿意改 crate / 要不要保持纯 MLX"来选。

---

## 4. Route A — MLX 原生路线（修两洞 + Q4_0/group32 + 验证）

### Phase A1（致命修复）：补全 `QwenVAE` encoder/quant_conv 加载
**Files:** Modify `OminiX-MLX/qwen-image-mlx/src/vae/weights.rs`（`load_vae_weights`，~14-25）

- [ ] **Step 1:** 用 `mlx-community/qwen-image-edit-2511-8bit` 的 `vae/0.safetensors`（key 与 crate 一致）。`load_vae_from_dir`（`vae/weights.rs:180-186`）已探测 `qwen_image_vae.safetensors` / `vae/0.safetensors` 两个名，用后者即命中。
- [ ] **Step 2:** 在 `load_vae_weights` 中**镜像现有 decoder/post_quant_conv 的加载写法**，新增：
  - `encoder` 各层（对照 `QwenImageEncoder3D` 结构 + decoder 的 `load_resblock_weights`/`load_conv3d_weights`/`load_rms_norm_weights`/`load_attention_weights` helper；文件里的 `encoder.*` key 直接匹配）。
  - `quant_conv`（对照已有 `post_quant_conv` 的 `load_conv3d_weights`；文件里是 `quant_conv.conv3d.weight/bias`）。
- [ ] **Step 3（验证：encode→decode 回环）：** 输入 512×512 图：`image → normalize → VAE.encode → latent → VAE.decode → png`。断言：latent shape `[1,16,64,64]`；encode 输出非 NaN/Inf 且 **mean/std 在合理范围**（随机 encoder 也可能输出看似正常却语义全错的张量，必须比统计量）；重建图非全黑/白/噪声。
- [ ] **Step 4:** Commit `feat(vae): load QwenVAE encoder + quant_conv (enables edit)`。

### Phase A2：三个加载器加 strict 报告
**Files:** Modify `qwen_quantized.rs`(`load_transformer_weights`)、`text_encoder.rs`(`load_text_encoder`)、`vae/weights.rs`(`load_vae_*`)

- [ ] **Step 1:** 把"静默 update_flattened"改为可选 strict：遍历对比 model 参数 key 集合 vs 文件 key 集合，分类打印/报错 **loaded / missing / unused / shape-mismatch**（环境变量开关，验证开、生产关）。
- [ ] **Step 2:** Commit `feat(loaders): optional strict weight-load report`。

### Phase A3：group_size 处理 Q4_0
**Files:** Modify `qwen_quantized.rs`(`QwenConfig`) + `OminiX-API/src/engines/image.rs:832`

- [ ] **Step 1:** 让 edit-2511 的 `QwenConfig` 用 `group_size=32, bits=4`。最干净：加载 GGUF 时按量化类型自动判定（Q4_0/Q4_1→32，原生 4bit safetensors→64），否则按文件名/模型 id 分流（已有 `:832` 模式）。
- [ ] **Step 2:** Commit `feat(qwen-image): group_size 32 path for Q4_0 gguf`。

### Phase A3b：文本编码器量化加载（VL-4bit + 改名 + QuantizedLinear）
**Files:** Modify `qwen-image-mlx/src/text_encoder.rs`（`QwenTextEncoder` 结构 + `load_text_encoder_weights`）

已验证 `mlx-community/Qwen2.5-VL-7B-Instruct-4bit` 的 shape 与 `TextEncoderConfig::default()` 完全吻合（hidden 3584 / 28 层 / GQA 4kv / intermediate 18944），**group_size=64、bits=4**，~4.5GB。但它命名为 `language_model.model.*` 且是量化态,而 crate 文本编码器目前是全精度 plain Linear。

- [ ] **Step 1:** 下载 `mlx-community/Qwen2.5-VL-7B-Instruct-4bit` 的 `language_model.*` 分片 + index 到模型目录的 `text_encoder/`（或保留原 repo，加载时过滤）。仅需 `language_model.model.*`,可丢弃 `visual.*` 与 `lm_head`。
- [ ] **Step 2:** 把 `QwenTextEncoder` 的各 Linear（`q/k/v/o_proj`、`mlp.{gate,up,down}_proj`、`embed_tokens`）改为 `QuantizedLinear`（`group_size=64, bits=4`），forward 用量化 matmul（照搬 transformer `qwen_quantized.rs` 里 `QuantizedLinear` 的用法）。`input_layernorm`/`post_attention_layernorm`/`norm` 保持全精度。
- [ ] **Step 3:** 在 `load_text_encoder_weights` 加前缀重映射 `language_model.model.` → `encoder.`（及 `language_model.model.norm`→`encoder.norm`、`language_model.model.embed_tokens`→`encoder.embed_tokens`），并按量化三件套加载 `.weight`(uint32)/`.scales`/`.biases`（照搬 `load_transformer_weights` 的 uint32 检测逻辑）。开 strict（Phase A2）确认 missing 清零。
- [ ] **Step 4（验证）:** 加载后 forward 一段 prompt,输出 embedding shape 末维=3584、非 NaN;统计量与全精度版（在大内存机器上跑 mlx-community 8bit 全精度 text_encoder）对比接近。
- [ ] **Step 5:** Commit `feat(text-encoder): QuantizedLinear + language_model.*→encoder.* remap (VL-4bit)`。

### Phase A4：API 路由识别 edit-2511
**Files:** Modify `OminiX-API/src/engines/image.rs:58`（`from_model_id`）

- [ ] **Step 1:** `ImageModelType::QwenImage` 的 id 列表加入 `"qwen-image-edit-2511-4bit"`/`"qwen-image-edit-2511-q40"`（与注册表 `api_model_id` 对齐）。确认带 `image` 字段时走 `generate_qwen_image_edit`（`:619` 已接，无需改编排）。
- [ ] **Step 2:** Commit `feat(image): map qwen-image-edit-2511 ids to QwenImage`。

### Phase A5：API 逐级验证（复用现有链路）
**Files:** none（启动 API + curl；首验前把 `image.rs:1078-1082` 的 `num_steps=20`/`cfg=2.5` 临时改小、强制 512×512）

- [ ] **Step 1（strict + VAE 回环）：** 发带 `image` 的 `/v1/images/generations`，看三份 strict 报告无关键 missing/shape-mismatch；`qwen_vae.encode()` latent shape/统计正确、非 NaN。
- [ ] **Step 2（text encoder + 单步 forward）：** embedding 与 transformer `joint_attention_dim=3584` 对齐、非 NaN；首步 `forward_edit` 输出 shape == main latent packed shape、非 NaN/Inf、单步不炸内存。
- [ ] **Step 3（最小闭环）：** `512×512, steps 1→2→4→8, 固定 seed, 单参考图`。断言 PNG 可开、非全黑/白/噪、prompt 有可见影响；记录内存峰值与耗时。

### Phase A6：16GB 低内存优化（闭环可用后）
- [ ] staged loading（编码 prompt 后释放 text encoder、编码 reference 后释放缓存）；每 step 清 MLX cache；限 512×512；禁 batch/n>1；可选 Lightning 4-step LoRA。
- [ ] 记录 Q4_0/group32 在 16GB 的峰值与耗时，定推荐档。

**Route A 真实剩余风险（这些是"得动手验"，不是原理不行）：**
1. **MLX 对 Q4_0/Q4_1 的导入是否真喂得进 `QuantizedLinear(group32,bits4)`**（Task G1 Step 3，唯一只能实测项；大概率成，是 MLX 预期路径）。
2. **VAE encoder 的 forward 从没被跑过**——补加载后 encode 本身可能有 bug（Phase A1 回环验证捕捉）。
3. **VL 文本编码器 GGUF 映射**（Task G2 / Phase A5 Step 2，最大未验证项）。
4. **16GB 无 CPU offload**：Q4_0 transformer ~11GB + VL + VAE + 激活，靠 staging 硬塞，可能仍紧/swap。

---

## 5. Route B — stable-diffusion.cpp 子进程（备选/兜底）

何时选：决策门 G1/G2 卡死，或想要最高把握"真在 16GB 出图"。优点：零 MLX-crate 改动、原生吃任意 k-quant（可选 Q2_K/Q3_K_M 留余量）、有 `--offload-to-cpu`。缺点：外部二进制依赖、两套推理栈、偏离纯 MLX 架构、慢。模板是 `OminiX-API/src/engines/ascend.rs:879`（`ominix-diffusion-cli` 就是 sd.cpp 风格 CLI）。

文件（sd.cpp 兼容，与 Route A 不同！）：transformer = `unsloth/.../qwen-image-edit-2511-Q3_K_M.gguf`(9.9GB) 或 `Q2_K`(7.5GB)；VAE = **`Comfy-Org/Qwen-Image_ComfyUI`** 的 `qwen_image_vae.safetensors`（**这是 sd.cpp 的正确 VAE**，与 MLX crate 那条无关）；text encoder = `unsloth/Qwen2.5-VL-7B-Instruct-GGUF`。

### Task B0：建 sd 二进制 + 手动跑通（先做，无代码）
- [ ] `git clone --recursive https://github.com/leejet/stable-diffusion.cpp && cmake -DSD_METAL=ON -DCMAKE_BUILD_TYPE=Release && cmake --build`，得到 `sd`。
- [ ] 下三文件，手动跑一次（**reference 图用 `-r`，不是 `-i`**）：
```bash
sd --diffusion-model <Q3_K_M.gguf> --vae <qwen_image_vae.safetensors> --llm <qwen2.5-vl.gguf> \
   --cfg-scale 2.5 --sampling-method euler --diffusion-fa --flow-shift 3 --offload-to-cpu \
   -r /tmp/ref.png -p "make the sky a sunset" -W 768 -H 768 --steps 20 -o /tmp/out.png -v
```
出图即证可行；OOM 则换 Q2_K + 512²。**失败就停，先解决 sd.cpp 再写 Rust。**

### Task B1：`SdCppImageEngine`（镜像 `AscendImageEngine`）
**Files:** Create `OminiX-API/src/engines/sdcpp_image.rs`；Modify `engines/mod.rs`
- [ ] 加 `SdCppConfig { sd_bin, diffusion_model, vae, llm, steps, cfg_scale, flow_shift, offload_to_cpu }` + `is_available()`。
- [ ] **纯函数 `build_sd_args(...)`（TDD 单测）：** 断言 reference 用 `-r`（不是 `-i`）、含 `--offload-to-cpu`/`--diffusion-model`/`--llm`。
```rust
pub fn build_sd_args(cfg:&SdCppConfig, prompt:&str, w:u32, h:u32, out:&str, r:Option<&str>) -> Vec<String> {
    let mut a = vec![
        "--diffusion-model".into(), cfg.diffusion_model.to_string_lossy().into(),
        "--vae".into(), cfg.vae.to_string_lossy().into(),
        "--llm".into(), cfg.llm.to_string_lossy().into(),
        "--cfg-scale".into(), cfg.cfg_scale.to_string(),
        "--sampling-method".into(),"euler".into(),"--diffusion-fa".into(),
        "--flow-shift".into(), cfg.flow_shift.to_string(),
        "--steps".into(), cfg.steps.to_string(),
        "-W".into(), w.to_string(), "-H".into(), h.to_string(),
        "-p".into(), prompt.into(), "-o".into(), out.into(),
    ];
    if cfg.offload_to_cpu { a.push("--offload-to-cpu".into()); }
    if let Some(r)=r { a.push("-r".into()); a.push(r.into()); }
    a
}
```
- [ ] `generate(&self, req:&ImageGenerationRequest) -> Result<ImageGenerationResponse>`：把 `req.image`(b64) 写临时 png 作 `-r`，spawn `sd`，读输出 png → b64 → `ImageGenerationResponse{ data: vec![ImageData{ b64_json:Some(b64), .. }] }`（完全照 `ascend.rs:879-969` 结构，去掉 `GGML_CANN_*` env）。

### Task B2：接线
**Files:** Modify `src/config.rs`、`src/state.rs:33`、`src/handlers/image.rs:17`、`src/handlers/models.rs:126`
- [ ] `SdCppConfig` 从 `SDCPP_*` 环境变量读入 → `AppState.sdcpp_config: Option<Arc<..>>`。
- [ ] `images_generations` 顶部分支：`request.model == Some("qwen-image-edit-2511-sdcpp")` → 用 `SdCppImageEngine`（state-access + JSON render 照 `images_generations_ascend`(:48-90) 抄）。
- [ ] `/v1/models/load` 的 `"image"` 分支：该 id 设为 verify-only no-op（sd.cpp 每次调用即载，无常驻）。

### Task B3：curl 端到端验证
- [ ] `SDCPP_BIN=<sd> cargo run -p ominix-api`，`curl .../v1/images/generations` 带 b64 `image` → 解出 png 打开。

---

## 6. Moxin Studio 接入（两条路共用，仅注册表）

前端 `image_edit` panel + `OminiXImageClient`（POST `/images/generations`）已存在，只需注册表条目。**仅在 Route A Phase A5 或 Route B B3 通过后做。**
**Files:** Modify `Moxin-Studio/moly-data/src/models_registry.json`
- [ ] 追加条目；按选定路线填 `source`/`extra_sources`（Route A：Q4_0 GGUF + mlx-community `vae/0.safetensors` + VL GGUF，`inference_engine:"mlx"`；Route B：Q3_K_M/Q2_K GGUF + Comfy-Org VAE + VL GGUF，`inference_engine:"sdcpp"`），`storage.size_bytes` 用实测值（Moxin 对 `.gguf` 有 5% 大小校验，见 `moly-shell/src/app.rs`）。
- [ ] `ui.panel_type:"image_edit"`，标 experimental，默认 512×512。
- [ ] `python3 -c "import json;json.load(open('moly-data/src/models_registry.json'))"` 校验 JSON。
- [ ] 全栈验证：Hub 下载→Load→编辑面板传图+提示词→返回 b64→显示。确认非 sd.cpp 的 MLX 模型未被回归。

---

## 7. 路线对比与推荐

| | Route A（MLX 原生） | Route B（sd.cpp 子进程） |
|---|---|---|
| 16GB 能装下 | Q4_0 ~11GB + staging，可能（无 offload，紧）| 是（小 k-quant + `--offload-to-cpu`）|
| 是否需改 MLX crate | 是（VAE encoder + group_size + strict）| 否 |
| 是否需大内存机器 | 否（Q4_0 直接下载）| 否 |
| 用现成公开文件 | 是（unsloth Q4_0 + mlx-community VAE + VL）| 是（unsloth k-quant + Comfy-Org VAE + VL）|
| 保持纯 MLX 架构 | 是 | 否（外部二进制）|
| 速度 | 快 | 慢 |
| 真"能出图"把握 | 中（3 个未验证点）| 高（成熟运行时）|
| 额外依赖/维护 | 无新二进制 | sd 二进制 + 两套栈 |

**推荐（决策门已于 2026-05-28 跑完，见 §3「决策门执行结果」）：G1 ✅;G2 非死路。两条路对 16GB 都可行,按取舍选:**
- **Route A（纯 MLX）** 需 **3 处 crate 改动**:① VAE encoder/quant_conv 加载（Defect 1，致命）② transformer `group_size`→32 ③ 文本编码器 `QuantizedLinear` + `language_model.*`→`encoder.*` 改名（已验证 `mlx-community/Qwen2.5-VL-7B-Instruct-4bit` shape 吻合、group_size 64、~4.5GB,量化无需自己做）。优点:纯架构、快、配 staging 在 16GB 宽裕。
- **Route B（sd.cpp）** 零 crate 改动、把握最高,代价:外部 `sd` 二进制 + 两套栈。

选 A 还是 B,取决于"愿不愿意动 crate 代码 / 是否要保持纯 MLX 架构"。两条都不再有"装不下 16GB"的硬伤。

---

## 8. 验收标准

**模型层：** 三件套可读且 VAE 含 encoder；三份 strict 报告无关键 missing/shape-mismatch；VAE encode/decode 回环统计合理；text encoder forward 对齐 3584；单步 `forward_edit` 通过；512×512 出 PNG 且 prompt 有可见影响；16GB 上完成一次编辑并记录峰值/耗时。
**产品层：** Moxin 可下载（地址有效）；传图+prompt 触发编辑；API 返回 `b64_json` 并显示；错误场景（缺模型/OOM/未传图/分辨率不支持/strict 失败）有明确提示。

---

## 附录 A：在线权重核查技术（不下整模型）
- **safetensors**：`curl -r 0-7` 取头 8 字节（u64 LE = header 长度）→ 再 range 取该长度 → JSON 列全部张量名/dtype/shape。（已用此法证实 mlx-community VAE key 匹配、Comfy-Org VAE 不匹配 MLX crate。）
- **GGUF**：range 取前 ~8MB → 解析 magic/version/计数 → 跳过 metadata KV → 读 tensor-info 得每张量名 + ggml 量化类型。（已用此法证实 Q4_0 文件构成 Q4_0/Q4_1/F32/BF16、无 k-quant。）

## 附录 B：自量化 MLX 4bit（group64）——Q4_0/group32 出问题时的退路
若 Task G1 Step 3 证明 MLX 吃不下 Q4_0（group32 仍 shape 错且不想改 crate），可离线把全精度/8-bit 权重用 `mlx.nn.quantize(model, bits=4, group_size=64)` 压成 MLX 原生 4bit safetensors（transformer ~11GB，结构同 mlx-community：`transformer/*.safetensors`+index、`vae/0.safetensors`），直接被现有 Rust 加载路径消费、与 crate 的 group64 一致。代价：一次性离线量化需 ≥32GB 机器加载源权重 + 质量需与 Q4_0 实测对比。
> 旁证：文生图 `qwen-image-2512` 有现成 `mlx-community/Qwen-Image-2512-4bit`（4bit、16GB 可跑），但 **edit 社区只有 8bit**——这正是 edit 需自量化或用 Q4_0 的根源。
