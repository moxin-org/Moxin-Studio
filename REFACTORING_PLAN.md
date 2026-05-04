# Moxin-Studio Refactoring Plan

## Goal

Refactor Moxin-Studio to reuse widgets from `makepad-component` and adopt
architectural patterns from `mofa-studio`, eliminating ~800 lines of duplicated
DSL code and establishing a shared component layer.

---

## Phase 1: Eliminate Duplication in app.rs (High Impact, Low Risk)

### 1.1 Replace 12 static chat tiles with PortalList

**Problem**: `app.rs` lines 565-904 contain 12 identical `chat_tile_N` blocks
(340 lines).

**Files to change**:
- `moly-shell/src/app.rs` — DSL + Rust binding

**Approach**: Define a single `ChatTile` template and use PortalList with
enum-based dispatch (same pattern as `moly-kit/src/widgets/messages.rs:210`).

```
// Before: 12 copies x 25 lines = 300 lines
chat_tile_0 = <RoundedView> { ... }
chat_tile_1 = <RoundedView> { ... }
...

// After: 1 template + PortalList = ~40 lines
chat_tiles_list = <PortalList> {
    ChatTile = <RoundedView> {
        width: Fill, height: 144
        show_bg: true
        draw_bg: { color: #ffffff, border_radius: 12.0 }
        flow: Down
        padding: {top: 16, left: 16, right: 16, bottom: 16}
        cursor: Hand

        header = <View> {
            width: Fill, height: Fit
            flow: Right
            align: {y: 0.5}
            chat_title = <Label> { ... }
            delete_btn = <View> { ... }
        }
        <View> { width: Fill, height: Fill }
        date_label = <Label> { ... }
    }
}
```

Rust binding:
```rust
let chats = self.store.get_visible_chats();
list.set_item_range(cx, 0, chats.len());
while let Some(index) = list.next_visible_item(cx) {
    let item = list.item(cx, index, live_id!(ChatTile));
    item.label(ids!(header.chat_title)).set_text(cx, &chats[index].title);
    item.label(ids!(date_label)).set_text(cx, &chats[index].date);
    item.draw_all(cx, &mut Scope::empty());
}
```

**Removes**: ~300 lines of DSL, `update_chat_tiles()` manual field-by-field code,
`handle_chat_tile_clicks()` manual per-tile detection. Replace with
`list.items_with_actions()` pattern from `chat_history.rs:151`.

### 1.2 Replace 6 static chat sidebar items with PortalList

**Problem**: `app.rs` lines 234-431 contain 6 identical `chat_item_N` blocks
(156 lines) with identical hover/selected shaders.

**Files to change**:
- `moly-shell/src/app.rs` — DSL + Rust binding

**Approach**: Same PortalList pattern. Single `ChatListItem` template.

```
chat_history_list = <PortalList> {
    ChatListItem = <View> {
        width: Fill, height: 32
        padding: {left: 8, right: 8}
        align: {y: 0.5}
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance hover: 0.0
            instance selected: 0.0
            fn pixel(self) -> vec4 {
                let base = #ffffff;
                let hover_color = #f1f5f9;
                let selected_color = #dbeafe;
                return mix(mix(base, hover_color, self.hover),
                           selected_color, self.selected);
            }
        }
        title = <Label> {
            width: Fill
            draw_text: {
                color: #374151
                text_style: { font_size: 11.0 }
                wrap: Ellipsis
            }
        }
    }
}
```

**Removes**: ~130 lines of DSL, eliminates `chat_history_visible` /
`chat_history_more` split, removes `show_more_btn` (PortalList scrolls natively).

---

## Phase 2: Replace Hand-Rolled Widgets with makepad-component

### 2.1 Replace SidebarButton with MpButton

**Problem**: `app.rs` lines 51-115 define a custom `SidebarButton` with 65 lines
of shader code for hover/pressed/selected states.

**Files to change**:
- `moly-shell/src/app.rs` — replace DSL definition

**Replace with**: `MpButton` ghost variant from makepad-component. It already has
hover/press animations, icon+text layout, and color variants.

```
SidebarButton = <MpButton> {
    width: Fill, height: Fit
    // Use ghost or outline variant for navigation styling
    // Override draw_text + draw_icon colors to match current design
}
```

**Removes**: 65 lines of custom shader code.

### 2.2 Replace chat tiles with MpCard

**Problem**: Chat tile styling is hand-rolled with basic RoundedView.

**Replace with**: `MpCard` with hover variant.

```
ChatTile = <MpCardHover> {
    // MpCard provides: border_radius, shadow, hover effect
    // Sub-components: MpCardHeader, MpCardContent, MpCardFooter
    header = <MpCardHeader> {
        title = <MpCardTitle> { /* chat title */ }
    }
    content = <MpCardContent> { /* spacer */ }
    footer = <MpCardFooter> {
        date_label = <Label> { /* date */ }
    }
}
```

### 2.3 Replace SearchInput with MpInput

**Problem**: `moly-models` and `moly-settings` each define their own SearchInput
with custom shaders.

**Files to change**:
- `apps/moly-models/src/screen/design.rs`
- `apps/moly-settings/src/screen/design.rs`

**Replace with**: `MpInput` search variant (capsule shape built-in).

### 2.4 Replace StatusDot with MpBadge

**Problem**: Two separate StatusDot implementations — `moly-settings` (4 states)
and `moly-local-models` (6 states).

**Files to change**:
- `apps/moly-settings/src/screen/design.rs`
- `apps/moly-local-models/src/screen/design.rs`

**Replace with**: `MpBadge` in dot mode with color variants
(success/warning/danger/info).

### 2.5 Replace SettingsTextInput with MpInput

**Files to change**:
- `apps/moly-settings/src/screen/design.rs`

**Replace with**: `MpInput` standard variant.

### 2.6 Add MpSpinner for loading states

Currently there are no loading indicators in the shell. Use `MpSpinner` for:
- Model list loading
- Chat history loading
- Settings save feedback

### 2.7 Add MpTooltip for icon buttons

Sidebar icon buttons and action buttons lack hover hints.

---

## Phase 3: Extract Shared Components to moly-widgets

### 3.1 Create components module in moly-widgets

**Files to create**:
- `moly-widgets/src/components/mod.rs`
- `moly-widgets/src/components/chat_list_item.rs`
- `moly-widgets/src/components/chat_tile.rs`
- `moly-widgets/src/components/category_badge.rs`
- `moly-widgets/src/components/model_card.rs`
- `moly-widgets/src/components/download_button.rs`

**What to extract**:

| Widget | Source | Reason |
|--------|--------|--------|
| ChatListItem | app.rs chat_item_* | Used by sidebar + chat history |
| ChatTile | app.rs chat_tile_* | Used by chat history grid |
| CategoryBadge | moly-local-models design.rs | Reusable colored badge |
| ModelCard | moly-models design.rs | Reusable info card |
| DownloadButton | moly-models design.rs | Reusable primary action button |

### 3.2 Consolidate label/text styles

Create shared text styles in `moly-widgets/src/theme.rs`:

```
SectionTitle = <Label> { draw_text: { text_style: <FONT_SEMIBOLD>{ font_size: 16.0 } } }
HintText = <Label> { draw_text: { color: #6b7280, text_style: { font_size: 10.0 } } }
BodyText = <Label> { draw_text: { color: #374151, text_style: { font_size: 11.0 } } }
```

Replaces: `SettingsLabel`, `SettingsHint`, `LocalModelsLabel`, `SectionTitle`
duplicated across app crates.

### 3.3 Add makepad-component as dependency to moly-widgets

```toml
# moly-widgets/Cargo.toml
[dependencies]
makepad-component.workspace = true
```

Re-export commonly used makepad-component widgets:
```rust
// moly-widgets/src/components/mod.rs
pub use makepad_component::widgets::{
    button::*, card::*, input::*, list::*, modal::*,
    badge::*, spinner::*, tooltip::*, switch::*,
};
```

---

## Phase 4: Adopt Shell Pattern from mofa-studio

### 4.1 Extract MolyShell widget

**Problem**: `app.rs` has ~600 lines of inline layout DSL for header, sidebar,
and content areas.

**Create**: `moly-widgets/src/shell.rs`

Follow mofa-studio's `MofaShell` pattern
(`mofa-ui/src/shell/layout.rs:24-124`):

```rust
MolyShell = {{MolyShell}} <View> {
    width: Fill, height: Fill
    flow: Down

    header_slot = <View> {
        width: Fill, height: 72
        flow: Right
        align: {y: 0.5}
        // Logo, title, theme toggle
    }

    main_area = <View> {
        width: Fill, height: Fill
        flow: Right

        sidebar_slot = <View> {
            width: 250, height: Fill
            flow: Down
        }

        content_slot = <View> {
            width: Fill, height: Fill
            flow: Overlay
        }
    }
}
```

**Rust struct methods** (from mofa-studio pattern):
- `set_sidebar_width(cx, width)` — collapse/expand
- `set_sidebar_visible(cx, visible)`
- `header_slot() -> ViewRef`
- `sidebar_slot() -> ViewRef`
- `content_slot() -> ViewRef`

### 4.2 Simplify app.rs

After extracting MolyShell, app.rs DSL shrinks from ~1000 lines to:

```
App = {{App}} {
    ui: <Window> {
        window: { title: "Moxin Studio", inner_size: vec2(1400, 900) }
        body = <MolyShell> {
            header_slot = {
                logo = <Image> { source: (IMG_LOGO) }
                title = <Label> { text: "Moxin Studio" }
                theme_toggle = <MpButton> { /* sun/moon */ }
            }
            sidebar_slot = {
                new_chat_btn = <MpButton> { text: "New Chat" }
                chat_list = <PortalList> { ChatListItem = <ChatListItem> {} }
                nav_buttons = <View> {
                    chat_btn = <MpButton> { text: "Chat" }
                    models_btn = <MpButton> { text: "Models" }
                    settings_btn = <MpButton> { text: "Settings" }
                }
            }
            content_slot = {
                chat_with_canvas = <View> { ... }
                models_app = <ModelsApp> { visible: false }
                settings_app = <SettingsApp> { visible: false }
                // ...
            }
        }
    }
}
```

### 4.3 Improve PageRouter usage

Current Moxin-Studio already has PageRouter in moly-widgets but doesn't use it.
`app.rs` manually tracks `current_view: NavigationTarget` and calls
`apply_view_state()` with manual visibility toggles.

**Change**: Use the existing `PageRouter` to manage navigation state. Replace
`NavigationTarget` enum with `PageRouter` calls. Replace `apply_view_state()`
with `router.navigate_to()` + visibility loop.

---

## Phase 5: Sync A2UI Module

### 5.1 Push moly-kit a2ui fixes to makepad-component

**Problem**: 10 files duplicated between
`makepad-component/crates/ui/src/a2ui/` and `moly-kit/src/a2ui/`.
The moly-kit copy has all recent fixes (JSON repair, char-boundary, brace
balancing). The makepad-component copy is stale.

**Files to sync** (moly-kit → makepad-component):
- `processor.rs` — JSON repair pipeline
- `message.rs` — `#[serde(default)]` on TextComponent.text
- `surface.rs` — reduced font sizes, Manrope fonts

### 5.2 Consider importing a2ui from makepad-component

Once synced, moly-kit could import the a2ui module from makepad-component
instead of maintaining its own copy:

```rust
// moly-kit/src/a2ui/mod.rs
pub use makepad_component::a2ui::*;
```

This requires makepad-component to be the canonical source. Evaluate whether
this is practical given that moly-kit may need a2ui customizations.

---

## Phase 6: Dark Mode Support

### 6.1 Consolidate dark mode shader pattern

**Problem**: Every app crate independently implements:
```glsl
instance dark_mode: 0.0
fn get_color(self) -> vec4 {
    return mix(#light, #dark, self.dark_mode);
}
```

**Solution**: Define base dark mode mixins in `moly-widgets/src/theme.rs`:

```
DarkModeView = <View> {
    show_bg: true
    draw_bg: {
        instance dark_mode: 0.0
        fn get_bg_color(self) -> vec4 {
            return mix(#ffffff, #1a1a2e, self.dark_mode);
        }
        fn pixel(self) -> vec4 {
            return self.get_bg_color();
        }
    }
}
```

### 6.2 Add Themeable trait

From mofa-studio's pattern (`mofa-ui/src/shell/layout.rs`):

```rust
pub trait Themeable {
    fn apply_dark_mode(&mut self, cx: &mut Cx, dark_mode: f64);
}
```

Implement for all extracted components in moly-widgets.

---

## Execution Priority

| Phase | Impact | Risk | Effort | Priority |
|-------|--------|------|--------|----------|
| 1.1 Chat tiles → PortalList | High (340 lines) | Low | Medium | **P0** |
| 1.2 Chat items → PortalList | High (156 lines) | Low | Medium | **P0** |
| 2.1 SidebarButton → MpButton | Medium (65 lines) | Low | Low | **P1** |
| 2.2 Chat tiles → MpCard | Medium | Low | Low | **P1** |
| 2.3-2.5 Inputs/StatusDot | Medium | Low | Low | **P1** |
| 3.1-3.3 Shared components | High (architecture) | Medium | Medium | **P1** |
| 4.1-4.3 MolyShell extraction | High (600 lines) | Medium | High | **P2** |
| 5.1-5.2 A2UI sync | Medium | Medium | Medium | **P2** |
| 6.1-6.2 Dark mode | Low | Low | Medium | **P3** |

---

## Expected Outcome

| Metric | Before | After |
|--------|--------|-------|
| app.rs DSL lines | ~1,000 | ~200 |
| Total duplicated lines | ~500 | 0 |
| Custom shader widgets | 20+ | 5-8 |
| Shared component library | None | 12+ widgets |
| makepad-component usage | 1 widget (MpSwitch) | 8+ widgets |
| A2UI source copies | 2 | 1 |
