use makepad_widgets::*;

use moly_data::{ChatId, Store, StoreAction, ModelRegistry, RegistryCategory, ModelRuntimeClient, ensure_server_running, UpdateInfo, check_for_update};
use std::sync::mpsc;
use std::path::Path;
use moly_kit::a2ui::{A2uiSurface, A2uiSurfaceAction};
use moly_kit::widgets::chat::ChatAction;
use moly_kit::widgets::prompt_input::PromptInputAction;
use moly_kit::widgets::take_pending_a2ui_json;
use moly_widgets::{MolyApp, MolyAppData};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use moly_widgets::theme::*;
    use moly_widgets::components::*;
    use moly_kit::a2ui::surface::*;

    // Import app widgets from external app crates
    use moly_chat::screen::design::*;
    use moly_settings::screen::design::*;
    use moly_mcp::screen::design::*;
    use moly_hub::screen::design::*;

    // Icon dependencies
    ICON_HAMBURGER = dep("crate://self/resources/icons/hamburger.png")
    ICON_MOON = dep("crate://self/resources/icons/moon.svg")
    ICON_CHAT = dep("crate://self/resources/icons/chat.png")
    ICON_SETTINGS = dep("crate://self/resources/icons/settings.png")
    ICON_HUB = dep("crate://self/resources/icons/hub.svg")
    ICON_LLM = dep("crate://self/resources/icons/llm.png")
    ICON_VLM = dep("crate://self/resources/icons/vlm.png")
    ICON_ASR = dep("crate://self/resources/icons/asr.png")
    ICON_TTS = dep("crate://self/resources/icons/tts.png")
    ICON_IMAGE = dep("crate://self/resources/icons/image.png")
    ICON_VIDEO = dep("crate://self/resources/icons/video.png")
    ICON_NEW_CHAT = dep("crate://self/resources/icons/new-chat.svg")
    ICON_TRASH = dep("crate://self/resources/icons/trash.svg")

    // Logo (light and dark variants)
    IMG_LOGO = dep("crate://self/resources/moxin-studio-logo.png")

    // Provider icons - registered globally so they can be loaded by moly-kit
    ICON_PROVIDER_OPENAI = dep("crate://self/resources/providers/openai.png")
    ICON_PROVIDER_ANTHROPIC = dep("crate://self/resources/providers/anthropic.png")
    ICON_PROVIDER_GEMINI = dep("crate://self/resources/providers/gemini.png")
    ICON_PROVIDER_OLLAMA = dep("crate://self/resources/providers/ollama.png")
    ICON_PROVIDER_DEEPSEEK = dep("crate://self/resources/providers/deepseek.png")
    ICON_PROVIDER_OPENROUTER = dep("crate://self/resources/providers/openrouter.png")
    ICON_PROVIDER_SILICONFLOW = dep("crate://self/resources/providers/siliconflow.png")

    // Reusable chat tile for chat history grid
    ChatTile = <RoundedView> {
        width: Fill, height: 144
        show_bg: true
        draw_bg: {
            border_radius: 12.0
            color: (PANEL_BG)
        }
        flow: Down
        padding: {top: 16, left: 16, right: 16, bottom: 16}
        cursor: Hand
        visible: false
        header = <View> {
            width: Fill, height: Fit
            flow: Right
            align: {y: 0.0}
            title = <Label> {
                width: Fill
                draw_text: { color: (TEXT_PRIMARY), text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }, wrap: Ellipsis }
            }
            delete_btn = <View> {
                width: 28, height: 28
                align: {x: 0.5, y: 0.5}
                cursor: Hand
                <Icon> { draw_icon: { svg_file: (ICON_TRASH), color: (TEXT_MUTED) }, icon_walk: {width: 18, height: 18} }
            }
        }
        <View> { width: Fill, height: Fill }
        footer = <View> {
            width: Fill, height: Fit
            flow: Right
            align: {y: 1.0}
            spacing: 8
            category_tag = <RoundedView> {
                width: Fit, height: Fit
                padding: {left: 6, right: 6, top: 2, bottom: 2}
                show_bg: true
                draw_bg: { border_radius: 4.0, color: #6366f1 }
                visible: false
                tag_label = <Label> {
                    draw_text: { color: #ffffff, text_style: <FONT_MEDIUM>{ font_size: 8.5 } }
                    text: "LLM"
                }
            }
            date_label = <Label> { draw_text: { color: (TEXT_MUTED), text_style: { font_size: 10.0 } } }
        }
    }

    // Row of 4 chat tiles for grid layout
    TileRow = <View> {
        width: Fill, height: Fit
        flow: Right
        spacing: 20
        visible: false
        tile_0 = <ChatTile> {}
        tile_1 = <ChatTile> {}
        tile_2 = <ChatTile> {}
        tile_3 = <ChatTile> {}
    }

    // Sidebar button using Button directly (like mofa-studio SidebarMenuButton)
    // Button natively supports icon + text with draw_icon and draw_text
    // Note: Button's draw_bg/draw_text/draw_icon don't support custom instance variables,
    // so we use fixed colors for light mode. Theme switching can be done by swapping button styles.
    SidebarButton = <View> {
        width: Fill, height: 38
        padding: {top: 7, bottom: 7, left: 12, right: 12}
        margin: {bottom: 2}
        flow: Right
        align: {x: 0.0, y: 0.5}
        cursor: Hand
        show_bg: true

        draw_bg: {
            instance selected: 0.0
            instance hover: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let normal = #00000000;
                let hover = #f3f4f6;
                let selected_bg = #eef2f7;
                let color = mix(mix(normal, hover, self.hover), selected_bg, self.selected);
                sdf.box(2.0, 2.0, self.rect_size.x - 4.0, self.rect_size.y - 4.0, 7.0);
                sdf.fill(color);
                sdf.box(2.0, 8.0, 3.0, self.rect_size.y - 16.0, 1.5);
                sdf.fill(vec4(0.04, 0.63, 0.59, self.selected));
                return sdf.result;
            }
        }

        animator: {
            hover = {
                default: off
                off = { from: {all: Forward{duration: 0.12}}, apply: {draw_bg: {hover: 0.0}} }
                on  = { from: {all: Forward{duration: 0.12}}, apply: {draw_bg: {hover: 1.0}} }
            }
        }

        sidebar_icon = <Image> {
            width: 22, height: 22
            margin: {right: 12}
            fit: Smallest
        }
        sidebar_label = <Label> {
            draw_text: {
                text_style: <FONT_MEDIUM>{ font_size: 12.5 }
                color: (TEXT_PRIMARY)
            }
        }
    }

    // Accent CTA button for "New Chat" — blue background, white text/icon
    NewChatButton = <Button> {
        width: Fill, height: 40
        padding: {top: 9, bottom: 9, left: 14, right: 14}
        margin: {bottom: 18}
        align: {x: 0.5, y: 0.5}
        icon_walk: {width: 18, height: 18, margin: {right: 8}}

        animator: {
            hover = {
                default: off,
                off = {
                    from: {all: Forward {duration: 0.15}}
                    apply: { draw_bg: {hover: 0.0} }
                }
                on = {
                    from: {all: Forward {duration: 0.15}}
                    apply: { draw_bg: {hover: 1.0} }
                }
            }
            pressed = {
                default: off,
                off = {
                    from: {all: Forward {duration: 0.1}}
                    apply: { draw_bg: {pressed: 0.0} }
                }
                on = {
                    from: {all: Forward {duration: 0.1}}
                    apply: { draw_bg: {pressed: 1.0} }
                }
            }
        }

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let base = #16a39c;
                let hover_color = #128c86;
                let pressed_color = #0f7a75;
                let color = mix(
                    mix(base, hover_color, self.hover),
                    pressed_color,
                    self.pressed
                );
                sdf.box(2.0, 2.0, self.rect_size.x - 4.0, self.rect_size.y - 4.0, 8.0);
                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            text_style: <FONT_SEMIBOLD>{ font_size: 12.5 }
            color: #ffffff
        }

        draw_icon: {
            fn get_color(self) -> vec4 {
                return #ffffff;
            }
        }
    }

    // Small uppercase section header label for sidebar groups
    SidebarSectionLabel = <Label> {
        width: Fill, height: Fit
        margin: {top: 8, bottom: 2, left: 12, right: 8}
        draw_text: {
            color: (TEXT_MUTED)
            text_style: <FONT_MEDIUM>{ font_size: 10.0 }
        }
    }

    // Slot item in the model-selector dropdown
    ModelDropdownSlot = <View> {
        width: Fill, height: 48
        cursor: Hand
        visible: false
        flow: Right
        align: {y: 0.5}
        padding: {left: 16, right: 12}
        spacing: 8
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                return #ffffff;
            }
        }

        // Category type tag — fixed width so all names align
        slot_type_tag = <RoundedView> {
            width: 52, height: 22
            padding: {left: 4, right: 4}
            align: {x: 0.5, y: 0.5}
            show_bg: true
            draw_bg: {
                color: #f3f4f6
                border_radius: 4.0
            }
            slot_type_label = <Label> {
                text: "LLM"
                draw_text: {
                    color: #4b5563
                    text_style: <FONT_MEDIUM>{ font_size: 10.0 }
                }
            }
        }

        // Model name — fills available space
        slot_name = <Label> {
            width: Fill
            draw_text: {
                color: #1f2937
                text_style: <FONT_MEDIUM>{ font_size: 13.0 }
                wrap: Ellipsis
            }
        }

        // "Loaded" tag — green pill, hidden by default
        slot_loaded_tag = <RoundedView> {
            width: Fit, height: 20
            visible: false
            padding: {left: 6, right: 6}
            align: {x: 0.5, y: 0.5}
            show_bg: true
            draw_bg: {
                color: #dcfce7
                border_radius: 4.0
            }
            <Label> {
                text: "Loaded"
                draw_text: {
                    color: #166534
                    text_style: <FONT_MEDIUM>{ font_size: 10.0 }
                }
            }
        }

        // Size on the right
        slot_meta = <Label> {
            draw_text: {
                color: #9ca3af
                text_style: { font_size: 11.0 }
            }
        }

        // Delete button — simple X
        slot_delete_btn = <View> {
            width: 24, height: 24
            cursor: Hand
            align: {x: 0.5, y: 0.5}
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 5.0);
                    sdf.fill(vec4(0.0, 0.0, 0.0, 0.0));

                    // Draw X shape
                    let cx = self.rect_size.x * 0.5;
                    let cy = self.rect_size.y * 0.5;
                    let arm = 5.0;
                    let thickness = 1.2;
                    sdf.move_to(cx - arm, cy - arm);
                    sdf.line_to(cx + arm, cy + arm);
                    sdf.stroke(#9ca3af, thickness);
                    sdf.move_to(cx + arm, cy - arm);
                    sdf.line_to(cx - arm, cy + arm);
                    sdf.stroke(#9ca3af, thickness);

                    return sdf.result;
                }
            }
        }
    }

    App = {{App}} {
        ui: <Window> {
            window: { title: "Moxin Studio", inner_size: vec2(1400, 900) }
            pass: {
                clear_color: #f5f7fa
            }

            body = <View> {
                width: Fill, height: Fill
                flow: Overlay
                show_bg: true
                draw_bg: {
                    color: #f5f7fa
                }

                // ── Normal app layout (header + sidebar + content) ──────────
                body_layout = <View> {
                    width: Fill, height: Fill
                    flow: Down

                // Update banner (hidden by default)
                update_banner = <View> {
                    width: Fill, height: 36
                    visible: false
                    flow: Right
                    align: {y: 0.5}
                    padding: {left: 16, right: 8}
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 {
                            return #16a39c;
                        }
                    }

                    update_label = <Label> {
                        width: Fill
                        text: ""
                        draw_text: {
                            color: #ffffff
                            text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                        }
                    }

                    update_download_btn = <View> {
                        width: Fit, height: 24
                        padding: {left: 10, right: 10}
                        margin: {right: 4}
                        cursor: Hand
                        align: {x: 0.5, y: 0.5}
                        show_bg: true
                        draw_bg: {
                            instance hover: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 4.0);
                                sdf.fill(mix(#ffffff33, #ffffff55, self.hover));
                                return sdf.result;
                            }
                        }
                        animator: {
                            hover = {
                                default: off
                                off = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 0.0}} }
                                on  = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 1.0}} }
                            }
                        }
                        <Label> {
                            text: "Download"
                            draw_text: {
                                color: #ffffff
                                text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
                            }
                        }
                    }

                    update_dismiss_btn = <View> {
                        width: 24, height: 24
                        cursor: Hand
                        align: {x: 0.5, y: 0.5}
                        show_bg: true
                        draw_bg: {
                            instance hover: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 4.0);
                                sdf.fill(mix(#00000000, #ffffff33, self.hover));
                                return sdf.result;
                            }
                        }
                        animator: {
                            hover = {
                                default: off
                                off = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 0.0}} }
                                on  = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 1.0}} }
                            }
                        }
                        <Label> {
                            text: "✕"
                            draw_text: {
                                color: #ffffffcc
                                text_style: { font_size: 13.0 }
                            }
                        }
                    }
                }

                // Header
                header = <View> {
                    width: Fill, height: 72
                    flow: Right
                    align: {y: 0.5}
                    padding: {left: 20, right: 20, top: 16}
                    show_bg: true
                    draw_bg: {
                        color: #ffffff
                    }

                    // Hamburger menu button
                    hamburger_btn = <View> {
                        width: 40, height: Fit
                        margin: {right: 12}
                        align: {x: 0.5, y: 0.5}
                        cursor: Hand
                        event_order: Down
                        show_bg: false

                        hamburger_icon = <Image> {
                            source: (ICON_HAMBURGER)
                            width: 20, height: 20
                            fit: Smallest
                        }
                    }

                    logo_light = <Image> {
                        source: (IMG_LOGO)
                        width: 180, height: 43
                    }

                    title_label = <Label> {
                        text: ""
                        width: 0
                    }

                    <View> { width: Fill } // Left spacer

                    // ── Model Selector pill (center of header, like LM Studio) ──
                    model_selector_btn = <View> {
                        width: Fit, height: 36
                        cursor: Hand
                        align: {x: 0.5, y: 0.5}
                        padding: {left: 16, right: 12, top: 0, bottom: 0}
                        margin: {right: 4}
                        show_bg: true
                        draw_bg: {
                            instance hover: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, 8.0);
                                sdf.fill(mix(#f3f4f6, #e5e7eb, self.hover));
                                return sdf.result;
                            }
                        }
                        animator: {
                            hover = {
                                default: off
                                off = { from: {all: Forward{duration: 0.15}}, apply: {draw_bg: {hover: 0.0}} }
                                on  = { from: {all: Forward{duration: 0.15}}, apply: {draw_bg: {hover: 1.0}} }
                            }
                        }
                        flow: Right
                        spacing: 8

                        selector_dot = <View> {
                            width: 8, height: 8
                            show_bg: true
                            draw_bg: {
                                instance loaded: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.circle(4.0, 4.0, 3.5);
                                    sdf.fill(mix(#9ca3af, #22c55e, self.loaded));
                                    return sdf.result;
                                }
                            }
                        }

                        // Category type badge — hidden until a model is loaded
                        category_tag = <View> {
                            width: Fit, height: 20
                            visible: false
                            padding: {left: 6, right: 6}
                            align: {x: 0.5, y: 0.5}
                            show_bg: true
                            draw_bg: {
                                instance cat: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 4.0);
                                    // LLM indigo-50, VLM violet-50, ASR green-50, TTS amber-50, Image pink-50, Video sky-50
                                    let c0 = #dbeafe;
                                    let c1 = #ede9fe;
                                    let c2 = #d1fae5;
                                    let c3 = #fef3c7;
                                    let c4 = #fce7f3;
                                    let c5 = #d4f1f9;
                                    let w0 = 1.0 - step(0.5, self.cat);
                                    let w1 = step(0.5, self.cat) * (1.0 - step(1.5, self.cat));
                                    let w2 = step(1.5, self.cat) * (1.0 - step(2.5, self.cat));
                                    let w3 = step(2.5, self.cat) * (1.0 - step(3.5, self.cat));
                                    let w4 = step(3.5, self.cat) * (1.0 - step(4.5, self.cat));
                                    let w5 = step(4.5, self.cat);
                                    sdf.fill(c0 * w0 + c1 * w1 + c2 * w2 + c3 * w3 + c4 * w4 + c5 * w5);
                                    return sdf.result;
                                }
                            }
                            category_tag_label = <Label> {
                                text: "LLM"
                                draw_text: {
                                    instance cat: 0.0
                                    fn get_color(self) -> vec4 {
                                        let c0 = #1a40af;
                                        let c1 = #5b21b6;
                                        let c2 = #047857;
                                        let c3 = #92400f;
                                        let c4 = #9d174d;
                                        let c5 = #0c4a6e;
                                        let w0 = 1.0 - step(0.5, self.cat);
                                        let w1 = step(0.5, self.cat) * (1.0 - step(1.5, self.cat));
                                        let w2 = step(1.5, self.cat) * (1.0 - step(2.5, self.cat));
                                        let w3 = step(2.5, self.cat) * (1.0 - step(3.5, self.cat));
                                        let w4 = step(3.5, self.cat) * (1.0 - step(4.5, self.cat));
                                        let w5 = step(4.5, self.cat);
                                        return c0 * w0 + c1 * w1 + c2 * w2 + c3 * w3 + c4 * w4 + c5 * w5;
                                    }
                                    text_style: <FONT_SEMIBOLD>{ font_size: 9.5 }
                                }
                            }
                        }

                        selector_label = <Label> {
                            text: "No model loaded"
                            draw_text: {
                                color: #374151
                                text_style: <FONT_MEDIUM>{ font_size: 13.0 }
                            }
                        }

                        <Label> {
                            text: "▾"
                            margin: {left: 2}
                            draw_text: {
                                color: #6b7280
                                text_style: { font_size: 11.0 }
                            }
                        }
                    }

                    // Eject / unload button (visible only when a model is loaded)
                    eject_btn = <View> {
                        width: 32, height: 32
                        cursor: Hand
                        visible: false
                        align: {x: 0.5, y: 0.5}
                        margin: {right: 8}
                        show_bg: true
                        draw_bg: {
                            instance hover: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, 6.0);
                                sdf.fill(mix(#f9fafb, #fee2e2, self.hover));
                                return sdf.result;
                            }
                        }
                        animator: {
                            hover = {
                                default: off
                                off = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 0.0}} }
                                on  = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 1.0}} }
                            }
                        }
                        <Label> {
                            text: "⏏"
                            draw_text: {
                                color: #6b7280
                                text_style: { font_size: 15.0 }
                            }
                        }
                    }

                    <View> { width: Fill } // Right spacer

                    // ── RAM usage ring gauge ────────────────────────────
                    ram_gauge = <View> {
                        width: 26, height: 26
                        margin: {right: 4}
                        show_bg: true
                        draw_bg: {
                            instance usage: 0.0
                            fn pixel(self) -> vec4 {
                                let center = self.rect_size * 0.5;
                                let uv = self.pos * self.rect_size - center;
                                let dist = length(uv);
                                let outer_r = center.x - 1.5;
                                let inner_r = outer_r - 3.0;

                                // Ring mask with anti-aliasing
                                let ring = smoothstep(inner_r - 0.5, inner_r + 0.5, dist)
                                         * (1.0 - smoothstep(outer_r - 0.5, outer_r + 0.5, dist));

                                // Angle from top, clockwise, normalized 0..1
                                let angle = atan(uv.x, -uv.y);
                                let a = angle / (2.0 * 3.14159);
                                let norm = a + (1.0 - step(0.0, a));

                                let filled = 1.0 - step(self.usage, norm);

                                // Ring background (unfilled) and fill color
                                let bg = vec3(0.88, 0.89, 0.91);
                                let c_lo  = vec3(0.09, 0.64, 0.29); // green
                                let c_mid = vec3(0.92, 0.70, 0.03); // amber
                                let c_hi  = vec3(0.94, 0.27, 0.27); // red
                                let fill_c = mix(
                                    mix(c_lo, c_mid, smoothstep(0.5, 0.7, self.usage)),
                                    c_hi, smoothstep(0.75, 0.9, self.usage)
                                );

                                let c = mix(bg, fill_c, filled);
                                return vec4(c, ring);
                            }
                        }
                    }
                    ram_label = <Label> {
                        text: ""
                        margin: {right: 12}
                        draw_text: {
                            color: #6b7280
                            text_style: { font_size: 8.5 }
                        }
                    }
                }

                // Content area
                content = <View> {
                    width: Fill, height: Fill
                    flow: Right

                    // Sidebar
                    sidebar = <View> {
                        width: 250, height: Fill
                        show_bg: true
                        draw_bg: {
                            color: #ffffff
                        }
                        flow: Down, padding: {top: 12, bottom: 12, left: 10, right: 8}

                        // Scrollable area for all sidebar content except Settings
                        sidebar_scroll = <ScrollYView> {
                            width: Fill, height: Fill
                            flow: Down

                            // New Chat - primary CTA button (accent blue)
                            new_chat_btn = <NewChatButton> {
                                text: "New Session"
                                draw_icon: { svg_file: (ICON_NEW_CHAT) }
                            }

                            // CHAT section
                            chat_section_label = <View> {
                                width: Fill, height: Fit
                                padding: {top: 4, bottom: 6, left: 12, right: 8}
                                <Label> {
                                    text: "HISTORY"
                                    draw_text: { color: (TEXT_MUTED), text_style: <FONT_MEDIUM>{ font_size: 10.0 } }
                                }
                            }

                            chat_section = <View> {
                                width: Fill, height: Fit
                                flow: Down
                                margin: {bottom: 8}

                                chat_history_btn = <SidebarButton> {
                                    sidebar_label = { text: "Session History" }
                                    sidebar_icon = { source: (ICON_CHAT) }
                                }

                                // Chat history sublist (collapsible, visible when sidebar expanded)
                                chat_history_visible = <View> {
                                    width: Fill, height: Fit
                                    flow: Down
                                    padding: {left: 30, top: 2, bottom: 2}

                                    chat_item_0 = <ChatListItem> {}
                                    chat_item_1 = <ChatListItem> {}
                                    chat_item_2 = <ChatListItem> {}

                                    // Show More button
                                    show_more_btn = <View> {
                                        width: Fill, height: 26
                                        padding: {left: 8, right: 8}
                                        align: {y: 0.5}
                                        flow: Right
                                        cursor: Hand
                                        show_bg: true
                                        draw_bg: {
                                            instance hover: 0.0
                                            fn pixel(self) -> vec4 {
                                                let base = (PANEL_BG);
                                                let hover_color = (HOVER_BG);
                                                return mix(base, hover_color, self.hover);
                                            }
                                        }
                                        show_more_label = <Label> {
                                            width: Fill
                                            text: "Show More"
                                            draw_text: {
                                                color: (TEXT_SECONDARY)
                                                text_style: { font_size: 10.5 }
                                            }
                                        }
                                        show_more_arrow = <Label> {
                                            text: ">"
                                            draw_text: {
                                                color: (TEXT_SECONDARY)
                                                text_style: { font_size: 11.0 }
                                            }
                                        }
                                    }
                                }

                                // Extra chat history items (hidden by default, shown via Show More)
                                chat_history_more = <View> {
                                    width: Fill, height: Fit
                                    flow: Down
                                    padding: {left: 32}
                                    visible: false

                                    chat_item_3 = <ChatListItem> { visible: false }
                                    chat_item_4 = <ChatListItem> { visible: false }
                                    chat_item_5 = <ChatListItem> { visible: false }
                                }
                            }

                            // MODELS section
                            models_section_label = <View> {
                                width: Fill, height: Fit
                                padding: {top: 12, bottom: 6, left: 12, right: 8}
                                <Label> {
                                    text: "MODELS"
                                    draw_text: { color: (TEXT_MUTED), text_style: <FONT_MEDIUM>{ font_size: 10.0 } }
                                }
                            }

                            llm_btn   = <SidebarButton> { sidebar_label = { text: "LLM" }   sidebar_icon = { source: (ICON_LLM) } }
                            vlm_btn   = <SidebarButton> { sidebar_label = { text: "VLM" }   sidebar_icon = { source: (ICON_VLM) } }
                            asr_btn   = <SidebarButton> { sidebar_label = { text: "ASR" }   sidebar_icon = { source: (ICON_ASR) } }
                            tts_btn   = <SidebarButton> { sidebar_label = { text: "TTS" }   sidebar_icon = { source: (ICON_TTS) } }
                            image_btn = <SidebarButton> { sidebar_label = { text: "Image" } sidebar_icon = { source: (ICON_IMAGE) } }
                            video_btn = <SidebarButton> { sidebar_label = { text: "Video" } sidebar_icon = { source: (ICON_VIDEO) } }

                            settings_btn = <SidebarButton> {
                                sidebar_label = { text: "Settings" }
                                sidebar_icon = { source: (ICON_SETTINGS) }
                            }
                        }

                        // Info button pinned at bottom of sidebar
                        <View> {
                            width: Fill, height: 1
                            margin: {left: 0, right: 8, top: 8, bottom: 4}
                            show_bg: true
                            draw_bg: { color: #f1f5f9 }
                        }
                        sidebar_info_btn = <View> {
                            width: Fill, height: 34
                            padding: {left: 12, right: 12}
                            align: {y: 0.5}
                            flow: Right
                            cursor: Hand

                            // Rounded "i" icon
                            <View> {
                                width: 18, height: 18
                                margin: {right: 8}
                                draw_bg: {
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        let c = vec2(9.0, 9.0);
                                        // Circle
                                        sdf.circle(c.x, c.y, 8.5);
                                        sdf.fill(#9ca3af);
                                        sdf.circle(c.x, c.y, 7.0);
                                        sdf.fill(#ffffff);
                                        // Dot
                                        sdf.circle(c.x, 5.0, 1.3);
                                        sdf.fill(#9ca3af);
                                        // Stem
                                        sdf.box(7.5, 7.5, 3.0, 5.5, 0.5);
                                        sdf.fill(#9ca3af);
                                        return sdf.result;
                                    }
                                }
                            }

                            <Label> {
                                text: "About"
                                draw_text: {
                                    fn get_color(self) -> vec4 { return #9ca3af; }
                                    text_style: <FONT_MEDIUM>{ font_size: 10.5 }
                                }
                            }
                        }
                    }

                    // Main content - app container
                    main_content = <View> {
                        width: Fill, height: Fill
                        flow: Overlay

                        // Chat History page (shown when clicking Chat icon)
                        chat_history_page = <View> {
                            width: Fill, height: Fill
                            flow: Down
                            visible: false
                            show_bg: true
                            draw_bg: {
                                color: #f5f7fa
                            }
                            padding: {top: 40, left: 48, right: 48, bottom: 32}

                            // Header with title
                            <View> {
                                width: Fill, height: Fit
                                margin: {bottom: 32}
                                align: {x: 0.5}
                                history_title = <Label> {
                                    text: "Session History"
                                    draw_text: {
                                        color: #1f2937
                                        text_style: <FONT_SEMIBOLD>{ font_size: 28.0 }
                                    }
                                }
                            }

                            // Search bar container
                            <View> {
                                width: Fill, height: Fit
                                align: {x: 0.5}
                                margin: {bottom: 40}

                                search_container = <RoundedView> {
                                    width: 500, height: 48
                                    show_bg: true
                                    draw_bg: {
                                        border_radius: 12.0
                                        color: #e5e7eb
                                    }
                                    padding: {left: 20, right: 20}
                                    align: {y: 0.5}
                                    flow: Right

                                    // Search icon
                                    <Icon> {
                                        draw_icon: {
                                            svg_file: (ICON_CHAT)
                                            color: #6b7280
                                        }
                                        icon_walk: {width: 20, height: 20, margin: {right: 12}}
                                    }

                                    // Search input
                                    search_input = <TextInput> {
                                        width: Fill, height: 32
                                        empty_text: "Search chats..."
                                        draw_text: {
                                            color: #1f2937
                                            color_focus: #1f2937
                                            color_empty: #6b7280
                                            color_empty_focus: #6b7280
                                            text_style: { font_size: 14.0 }
                                        }
                                        draw_selection: {
                                            color: #bfdbfe
                                            color_focus: #bfdbfe
                                        }
                                        draw_cursor: {
                                            color: #1f2937
                                        }
                                        draw_bg: {
                                            fn pixel(self) -> vec4 {
                                                return vec4(0.0, 0.0, 0.0, 0.0);
                                            }
                                        }
                                    }
                                }
                            }

                            // Empty state (shown when no chats)
                            empty_state = <View> {
                                width: Fill, height: Fill
                                align: {x: 0.5, y: 0.3}
                                visible: true
                                empty_label = <Label> {
                                    text: "No session history yet. Click 'New Session' to start."
                                    draw_text: {
                                        color: #6b7280
                                        text_style: { font_size: 16.0 }
                                    }
                                }
                            }

                            // Chat tiles mosaic grid (scrollable)
                            chat_tiles_scroll = <ScrollYView> {
                                width: Fill, height: Fill
                                visible: false

                                chat_tiles_container = <View> {
                                    width: Fill, height: Fit
                                    flow: Down
                                    spacing: 20

                                    tile_row_0 = <TileRow> {}
                                    tile_row_1 = <TileRow> {}
                                    tile_row_2 = <TileRow> {}
                                }
                            }
                        }

                        // Chat with canvas panel (horizontal layout)
                        chat_with_canvas = <View> {
                            width: Fill, height: Fill
                            flow: Right
                            visible: true

                            // Left: Chat app (fills remaining space)
                            chat_app = <ChatApp> {
                                width: Fill, height: Fill
                            }

                            // Middle: Splitter (resizable divider)
                            canvas_splitter = <View> {
                                width: 0, height: Fill  // 0 when collapsed, 16 when expanded
                                cursor: ColResize
                                show_bg: true
                                draw_bg: {
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        // Draw thin line in center
                                        sdf.rect(7.0, 16.0, 2.0, self.rect_size.y - 32.0);
                                        sdf.fill(#d1d5db);
                                        return sdf.result;
                                    }
                                }
                            }

                            // Right: Canvas panel (collapsed by default, opens when A2UI is enabled)
                            canvas_section = <View> {
                                width: 500, height: Fill
                                flow: Right
                                visible: false

                                // Collapse strip — always visible when canvas is expanded
                                canvas_toggle_column = <View> {
                                    visible: true
                                    width: 20, height: Fill
                                    cursor: Hand
                                    show_bg: true
                                    draw_bg: { color: #f8fafc }
                                    align: {x: 0.5, y: 0.5}
                                    <Label> {
                                        text: "›"
                                        draw_text: {
                                            color: #9ca3af
                                            text_style: { font_size: 18.0 }
                                        }
                                    }
                                }

                                // Content column
                                canvas_content = <RoundedView> {
                                    width: Fill, height: Fill
                                    visible: true
                                    draw_bg: {
                                        color: #ffffff
                                        border_radius: 8.0
                                    }
                                    flow: Down

                                    // Header
                                    canvas_header = <View> {
                                        width: Fill, height: Fit
                                        padding: {left: 16, right: 16, top: 12, bottom: 12}
                                        show_bg: true
                                        draw_bg: { color: #f8fafc }

                                        canvas_title = <Label> {
                                            text: "Canvas"
                                            draw_text: {
                                                color: #1f2937
                                                text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                                            }
                                        }
                                    }

                                    // Canvas area with A2UI Surface
                                    canvas_area = <ScrollYView> {
                                        width: Fill, height: Fill
                                        padding: 12

                                        a2ui_surface = <A2uiSurface> {
                                            width: Fill
                                            height: Fit
                                        }
                                    }
                                }
                            }

                            // Reopen strip — visible when canvas is collapsed
                            canvas_reopen_btn = <View> {
                                width: 20, height: Fill
                                visible: true
                                cursor: Hand
                                show_bg: true
                                draw_bg: { color: #f8fafc }
                                align: {x: 0.5, y: 0.5}
                                <Label> {
                                    text: "‹"
                                    draw_text: {
                                        color: #9ca3af
                                        text_style: { font_size: 18.0 }
                                    }
                                }
                            }
                        }

                        // Settings app
                        settings_app = <SettingsApp> {
                            visible: false
                        }

                        // Per-category Model Hub instances
                        llm_hub_app = <ModelHubApp> {
                            hub_category: 1.0
                            visible: false
                        }
                        vlm_hub_app = <ModelHubApp> {
                            hub_category: 2.0
                            visible: false
                        }
                        asr_hub_app = <ModelHubApp> {
                            hub_category: 3.0
                            visible: false
                        }
                        tts_hub_app = <ModelHubApp> {
                            hub_category: 4.0
                            visible: false
                        }
                        image_hub_app = <ModelHubApp> {
                            hub_category: 5.0
                            visible: false
                        }
                        video_hub_app = <ModelHubApp> {
                            hub_category: 6.0
                            visible: false
                        }

                        // MCP app (desktop only)
                        mcp_app = <McpApp> {
                            visible: false
                        }

                        // About / Readme page
                        about_page = <ScrollYView> {
                            width: Fill, height: Fill
                            visible: false
                            flow: Down
                            show_bg: true
                            draw_bg: { color: #f8fafc }
                            padding: {top: 28, left: 32, right: 32, bottom: 28}

                            // Title row
                            <View> {
                                width: Fill, height: Fit
                                flow: Right
                                align: {y: 0.5}
                                margin: {bottom: 2}

                                // Blue info circle
                                <View> {
                                    width: 22, height: 22
                                    margin: {right: 10}
                                    draw_bg: {
                                        fn pixel(self) -> vec4 {
                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                            let c = vec2(11.0, 11.0);
                                            sdf.circle(c.x, c.y, 10.0);
                                            sdf.fill(#3b82f6);
                                            sdf.circle(c.x, 5.5, 1.6);
                                            sdf.fill(#ffffff);
                                            sdf.box(9.2, 8.5, 3.6, 7.5, 0.8);
                                            sdf.fill(#ffffff);
                                            return sdf.result;
                                        }
                                    }
                                }

                                <Label> {
                                    text: "About Moxin Studio"
                                    draw_text: {
                                        color: #1f2937
                                        text_style: <FONT_SEMIBOLD>{ font_size: 18.0 }
                                    }
                                }
                            }

                            <Label> {
                                width: Fill, height: Fit
                                margin: {bottom: 14}
                                text: "Run AI models locally on your Mac. No cloud, no API keys, no data leaves your device."
                                draw_text: {
                                    color: #6b7280
                                    text_style: { font_size: 12.0 }
                                    wrap: Word
                                }
                            }

                            // Category sections — populated dynamically
                            about_llm_header = <Label> { width: Fill, height: Fit, margin: {bottom: 2}
                                draw_text: { color: #5b21b6, text_style: <FONT_SEMIBOLD>{ font_size: 12.0 } } }
                            about_llm_body = <Label> { width: Fill, height: Fit, margin: {bottom: 10}
                                draw_text: { color: #374151, text_style: { font_size: 11.5 }, wrap: Word } }

                            about_vlm_header = <Label> { width: Fill, height: Fit, margin: {bottom: 2}
                                draw_text: { color: #3730a3, text_style: <FONT_SEMIBOLD>{ font_size: 12.0 } } }
                            about_vlm_body = <Label> { width: Fill, height: Fit, margin: {bottom: 10}
                                draw_text: { color: #374151, text_style: { font_size: 11.5 }, wrap: Word } }

                            about_asr_header = <Label> { width: Fill, height: Fit, margin: {bottom: 2}
                                draw_text: { color: #303880, text_style: <FONT_SEMIBOLD>{ font_size: 12.0 } } }
                            about_asr_body = <Label> { width: Fill, height: Fit, margin: {bottom: 10}
                                draw_text: { color: #374151, text_style: { font_size: 11.5 }, wrap: Word } }

                            about_tts_header = <Label> { width: Fill, height: Fit, margin: {bottom: 2}
                                draw_text: { color: #92400f, text_style: <FONT_SEMIBOLD>{ font_size: 12.0 } } }
                            about_tts_body = <Label> { width: Fill, height: Fit, margin: {bottom: 10}
                                draw_text: { color: #374151, text_style: { font_size: 11.5 }, wrap: Word } }

                            about_image_header = <Label> { width: Fill, height: Fit, margin: {bottom: 2}
                                draw_text: { color: #7c3aad, text_style: <FONT_SEMIBOLD>{ font_size: 12.0 } } }
                            about_image_body = <Label> { width: Fill, height: Fit, margin: {bottom: 10}
                                draw_text: { color: #374151, text_style: { font_size: 11.5 }, wrap: Word } }

                            about_video_header = <Label> { width: Fill, height: Fit, margin: {bottom: 2}
                                draw_text: { color: #1d5191, text_style: <FONT_SEMIBOLD>{ font_size: 12.0 } } }
                            about_video_body = <Label> { width: Fill, height: Fit, margin: {bottom: 10}
                                draw_text: { color: #374151, text_style: { font_size: 11.5 }, wrap: Word } }

                            // Footer
                            <Label> {
                                width: Fill, height: Fit
                                margin: {top: 6}
                                text: "Models are downloaded from Hugging Face on first use and cached locally. All inference runs on-device via MLX on Apple Silicon."
                                draw_text: {
                                    color: #9ca3af
                                    text_style: { font_size: 10.5 }
                                    wrap: Word
                                }
                            }
                        }
                    }
                }
                } // closes body_layout

                // ── Model-selector dropdown overlay ─────────────────────────
                model_selector_dropdown = <View> {
                    abs_pos: vec2(0.0, 0.0)
                    width: Fill, height: Fill
                    flow: Overlay
                    visible: false

                    // Full-screen dismiss area (behind the panel)
                    dismiss_area = <View> {
                        width: Fill, height: Fill
                        cursor: Arrow
                    }

                    // Centered dropdown panel row
                    dropdown_wrapper = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        align: {x: 0.5}
                        margin: {top: 72}

                        dropdown_panel = <RoundedView> {
                            width: 540, height: Fit
                            show_bg: true
                            draw_bg: {
                                color: #ffffff
                                border_radius: 12.0
                                border_color: #d1d5db
                                border_size: 1.0
                            }
                            flow: Down

                            // Panel header row
                            dropdown_header = <View> {
                                width: Fill, height: 48
                                flow: Right
                                align: {y: 0.5}
                                padding: {left: 16, right: 16}

                                <Label> {
                                    width: Fill
                                    text: "On-Device Models"
                                    draw_text: {
                                        color: #1f2937
                                        text_style: <FONT_SEMIBOLD>{ font_size: 14.0 }
                                    }
                                }
                                dropdown_status_label = <Label> {
                                    text: ""
                                    draw_text: {
                                        color: #9ca3af
                                        text_style: { font_size: 11.0 }
                                    }
                                }
                                open_finder_btn = <View> {
                                    width: Fit, height: Fit
                                    align: {y: 0.5}
                                    padding: {left: 8}
                                    cursor: Hand
                                    <Label> {
                                        text: "Open in Finder"
                                        draw_text: {
                                            color: #6b7280
                                            text_style: { font_size: 11.0 }
                                        }
                                    }
                                }
                            }

                            <View> { width: Fill, height: 1, show_bg: true, draw_bg: { color: #e5e7eb } }

                            // Empty state
                            empty_state = <View> {
                                width: Fill, height: 80
                                visible: true
                                align: {x: 0.5, y: 0.5}
                                <Label> {
                                    text: "No models downloaded. Visit the Model Hub."
                                    draw_text: { color: #6b7280, text_style: { font_size: 12.0 } }
                                }
                            }

                            // Scrollable model list (hidden when empty)
                            model_scroll = <ScrollYView> {
                                width: Fill, height: Fit
                                flow: Down
                                visible: false

                                slot_0 = <ModelDropdownSlot> {}
                                slot_1 = <ModelDropdownSlot> {}
                                slot_2 = <ModelDropdownSlot> {}
                                slot_3 = <ModelDropdownSlot> {}
                                slot_4 = <ModelDropdownSlot> {}
                                slot_5 = <ModelDropdownSlot> {}
                                slot_6 = <ModelDropdownSlot> {}
                                slot_7 = <ModelDropdownSlot> {}
                                slot_8 = <ModelDropdownSlot> {}
                                slot_9 = <ModelDropdownSlot> {}
                            }

                            // Delete confirmation panel
                            delete_confirm_panel = <View> {
                                width: Fill, height: Fit
                                visible: false
                                flow: Down
                                padding: {top: 24, left: 20, right: 20, bottom: 24}
                                spacing: 12

                                confirm_msg = <Label> {
                                    text: "Delete this model?"
                                    draw_text: {
                                        color: #1f2937
                                        text_style: <FONT_SEMIBOLD>{ font_size: 15.0 }
                                    }
                                }

                                <Label> {
                                    text: "This will permanently remove the model files from disk."
                                    draw_text: {
                                        color: #6b7280
                                        text_style: { font_size: 12.0 }
                                    }
                                }

                                confirm_buttons = <View> {
                                    width: Fill, height: Fit
                                    flow: Right
                                    spacing: 8
                                    align: {x: 1.0, y: 0.5}

                                    cancel_delete_btn = <View> {
                                        width: Fit, height: 36
                                        cursor: Hand
                                        padding: {left: 16, right: 16}
                                        align: {x: 0.5, y: 0.5}
                                        show_bg: true
                                        draw_bg: {
                                            instance hover: 0.0
                                            fn pixel(self) -> vec4 {
                                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, 6.0);
                                                sdf.fill(mix(#f3f4f6, #e5e7eb, self.hover));
                                                return sdf.result;
                                            }
                                        }
                                        animator: {
                                            hover = {
                                                default: off
                                                off = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 0.0}} }
                                                on  = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 1.0}} }
                                            }
                                        }
                                        <Label> {
                                            text: "Cancel"
                                            draw_text: {
                                                color: #374151
                                                text_style: <FONT_MEDIUM>{ font_size: 13.0 }
                                            }
                                        }
                                    }

                                    confirm_delete_btn = <View> {
                                        width: Fit, height: 36
                                        cursor: Hand
                                        padding: {left: 16, right: 16}
                                        align: {x: 0.5, y: 0.5}
                                        show_bg: true
                                        draw_bg: {
                                            instance hover: 0.0
                                            fn pixel(self) -> vec4 {
                                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, 6.0);
                                                sdf.fill(mix(#dc2626, #b91c1c, self.hover));
                                                return sdf.result;
                                            }
                                        }
                                        animator: {
                                            hover = {
                                                default: off
                                                off = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 0.0}} }
                                                on  = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 1.0}} }
                                            }
                                        }
                                        <Label> {
                                            text: "Delete"
                                            draw_text: {
                                                color: #ffffff
                                                text_style: <FONT_MEDIUM>{ font_size: 13.0 }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                }
            }
        }
    }
}

// ── Model selector types ──────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Default, Debug)]
enum ShellModelLoadState {
    #[default]
    Unloaded,
    Loading,
    Loaded,
    Error,
}

#[derive(Clone, Debug)]
struct DownloadedModelEntry {
    registry_id:      String,
    name:             String,
    api_model_id:     String,
    category:         RegistryCategory,
    model_type_str:   &'static str,
    size_display:     String,
    local_path:       String,
    supports_images:  bool,
}

fn category_to_model_type(cat: RegistryCategory) -> &'static str {
    match cat {
        RegistryCategory::Llm      => "llm",
        RegistryCategory::Vlm      => "vlm",
        RegistryCategory::Asr      => "asr",
        RegistryCategory::Tts      => "tts",
        RegistryCategory::ImageGen => "image",
        RegistryCategory::VideoGen => "video",
    }
}

fn registry_category_as_f64(cat: RegistryCategory) -> f64 {
    match cat {
        RegistryCategory::Llm      => 0.0,
        RegistryCategory::Vlm      => 1.0,
        RegistryCategory::Asr      => 2.0,
        RegistryCategory::Tts      => 3.0,
        RegistryCategory::ImageGen => 4.0,
        RegistryCategory::VideoGen => 5.0,
    }
}

/// Check if a registry model's files are present on disk.
///
/// Multiple GGUF quant variants (e.g. q4km vs q8) share the same HuggingFace
/// repo directory.  To avoid false positives we compare each `.gguf` file's
/// actual size against the model's expected `size_bytes`.
fn shell_is_model_downloaded(model: &moly_data::RegistryModel) -> bool {
    let expanded = model.storage.expanded_path();
    let path = Path::new(&expanded);
    if !path.exists() { return false; }
    if model.storage.size_bytes > 100 * 1024 * 1024 {
        return has_weight_files_shell(path, model.storage.size_bytes);
    }
    std::fs::read_dir(path)
        .map(|e| e.filter_map(|x| x.ok())
             .filter(|x| !x.file_name().to_string_lossy().starts_with('.')).count())
        .unwrap_or(0) > 0
}

fn has_weight_files_shell(dir: &Path, expected_size: u64) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let n = e.file_name();
            let name = n.to_string_lossy();
            if name.ends_with(".gguf") {
                // GGUF: single file per quant variant — verify size matches
                // (follows symlinks, which HF cache uses in snapshots/)
                if expected_size > 0 {
                    let actual = std::fs::metadata(e.path()).map(|m| m.len()).unwrap_or(0);
                    let tolerance = expected_size / 20; // 5%
                    if (actual as i64 - expected_size as i64).unsigned_abs() <= tolerance {
                        return true;
                    }
                } else {
                    return true;
                }
            } else if name.ends_with(".safetensors") || name.ends_with(".bin") {
                return true;
            }
            if e.path().is_dir() && has_weight_files_shell(&e.path(), expected_size) { return true; }
        }
    }
    false
}

// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Default)]
enum NavigationTarget {
    /// Chat History page - blank page with "Chat History" text
    #[default]
    ChatHistory,
    /// Active chat - shows the chat interface
    ActiveChat,
    Settings,
    LlmHub,
    VlmHub,
    AsrHub,
    TtsHub,
    ImageHub,
    VideoHub,
    About,
}

#[derive(Live)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    store: Store,
    #[rust]
    app_data: MolyAppData,
    #[rust]
    current_view: NavigationTarget,
    #[rust]
    initialized: bool,
    /// Whether the chat history "Show More" section is expanded
    #[rust]
    chat_history_expanded: bool,
    /// Chat IDs displayed in the tiles (max 12)
    #[rust]
    displayed_chat_ids: Vec<ChatId>,
    /// Current search query for filtering chat history
    #[rust]
    search_query: String,
    /// Whether the canvas panel is collapsed
    #[rust]
    canvas_panel_collapsed: bool,
    /// Width of the canvas panel when expanded
    #[rust]
    canvas_panel_width: f64,
    /// Whether the splitter is being dragged
    #[rust]
    splitter_dragging: bool,
    /// Whether A2UI is enabled for the current chat
    #[rust]
    a2ui_enabled: bool,
    /// Starting X position when drag started
    #[rust]
    splitter_drag_start_x: f64,
    /// Starting width when drag started
    #[rust]
    splitter_drag_start_width: f64,
    /// Current A2UI JSON received from the model
    #[rust]
    pending_a2ui_json: Option<String>,
    /// Chat IDs shown in the sidebar history sublist (up to 6)
    #[rust]
    sidebar_chat_ids: Vec<moly_data::ChatId>,

    // ── Model-selector state ────────────────────────────────────────────────
    /// Whether the model-selector dropdown is currently open
    #[rust]
    selector_open: bool,
    /// Registry ID of the currently loaded local model (empty = none)
    #[rust]
    loaded_model_id: String,
    /// Display name of the loaded model
    #[rust]
    loaded_model_name: String,
    /// Category of the loaded model
    #[rust]
    loaded_model_category: Option<RegistryCategory>,
    /// Whether the loaded model supports input images (image editing)
    #[rust]
    loaded_model_supports_images: bool,
    /// Load state for the shell-level model selector
    #[rust]
    shell_load_state: ShellModelLoadState,
    /// Receiver for the async load thread
    #[rust]
    load_rx: Option<mpsc::Receiver<Result<(), String>>>,
    /// List of downloaded models available for selection
    #[rust]
    downloaded_models: Vec<DownloadedModelEntry>,
    /// Index of model pending delete confirmation (if any)
    #[rust]
    delete_confirm_index: Option<usize>,

    // ── RAM gauge state ─────────────────────────────────────────────────────
    #[rust]
    ram_timer: Timer,
    #[rust]
    ram_usage: f64,
    #[rust]
    ram_used_gb: f64,
    #[rust]
    ram_total_gb: f64,

    // ── Update checker state ────────────────────────────────────────────────
    #[rust]
    update_rx: Option<mpsc::Receiver<Option<UpdateInfo>>>,
    #[rust]
    update_info: Option<UpdateInfo>,
}

impl LiveHook for App {
    fn after_new_from_doc(&mut self, _cx: &mut Cx) {
        if !self.initialized {
            // Load Store from disk (this is called after Makepad creates the struct)
            self.store = Store::load();

            // Set current_view from loaded preferences
            self.current_view = match self.store.current_view() {
                "Settings"  => NavigationTarget::Settings,
                "ActiveChat" => NavigationTarget::ActiveChat,
                "LlmHub"   => NavigationTarget::LlmHub,
                "VlmHub"   => NavigationTarget::VlmHub,
                "AsrHub"   => NavigationTarget::AsrHub,
                "TtsHub"   => NavigationTarget::TtsHub,
                "ImageHub" => NavigationTarget::ImageHub,
                "VideoHub" => NavigationTarget::VideoHub,
                _ => NavigationTarget::ChatHistory,
            };

            // Initialize MolyAppData from Store preferences
            self.app_data = MolyAppData::new();
            self.app_data.sync_from_preferences(
                self.store.is_sidebar_expanded(),
                self.store.current_view(),
                self.store.preferences.get_current_chat_model(),
            );

            self.initialized = true;
            ::log::info!("App initialized via LiveHook, store loaded from disk");
        }
    }
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
        moly_widgets::live_design(cx);
        // Register moly-kit widgets (Chat, Messages, PromptInput, etc.)
        moly_kit::widgets::live_design(cx);
        // Register app widgets from external app crates via MolyApp trait
        <moly_chat::MolyChatApp as MolyApp>::live_design(cx);
        <moly_settings::MolySettingsApp as MolyApp>::live_design(cx);
        <moly_mcp::MolyMcpApp as MolyApp>::live_design(cx);
        <moly_hub::MolyHubApp as MolyApp>::live_design(cx);
    }
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        self.update_sidebar(cx);
        // Force apply view state on startup (bypass same-view check)
        self.apply_view_state(cx, self.current_view);
        // Populate sidebar chat history items
        self.update_sidebar_chats(cx);

        // Initialize canvas panel — collapsed by default, opens when A2UI is enabled
        self.canvas_panel_width = 500.0;
        self.canvas_panel_collapsed = true;

        // Start RAM usage polling (every 1 second)
        self.ram_timer = cx.start_interval(1.0);
        self.poll_ram_usage(cx);

        // Check for updates in background
        self.start_update_check();

        ::log::info!("App initialized with Store and MolyAppData");
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // ── Model selector pill click ───────────────────────────────────────
        if self.ui.view(ids!(body.body_layout.header.model_selector_btn)).finger_down(&actions).is_some() {
            if self.selector_open {
                self.close_selector(cx);
            } else {
                self.open_selector(cx);
            }
        }

        // ── Eject / unload button click ────────────────────────────────────
        if self.ui.view(ids!(body.body_layout.header.eject_btn)).finger_down(&actions).is_some() {
            self.start_unload_model(cx);
        }

        // ── Dropdown: click-outside dismiss area ───────────────────────────
        if self.selector_open {
            if self.ui.view(ids!(body.model_selector_dropdown.dismiss_area)).finger_down(&actions).is_some() {
                self.close_selector(cx);
            }

            // ── Open in Finder button ───────────────────────────────────────
            if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.dropdown_header.open_finder_btn)).finger_down(&actions).is_some() {
                let home = std::env::var("HOME").unwrap_or_default();
                let models_dir = format!("{}/.OminiX/models", home);
                let _ = std::process::Command::new("open").arg(&models_dir).spawn();
            }

            // ── Delete confirmation buttons ─────────────────────────────────
            if self.delete_confirm_index.is_some() {
                if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.delete_confirm_panel.confirm_buttons.cancel_delete_btn)).finger_down(&actions).is_some() {
                    self.hide_delete_confirm(cx);
                }
                if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.delete_confirm_panel.confirm_buttons.confirm_delete_btn)).finger_down(&actions).is_some() {
                    self.perform_delete_model(cx);
                }
            }

            // ── Dropdown slot clicks (with delete button handling) ──────────
            if self.delete_confirm_index.is_none() {
                let n = self.downloaded_models.len();
                if n > 0 {
                    if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_0.slot_delete_btn)).finger_down(&actions).is_some() {
                        self.show_delete_confirm(cx, 0);
                    } else if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_0)).finger_down(&actions).is_some() {
                        let entry = self.downloaded_models[0].clone();
                        self.close_selector(cx);
                        self.start_load_model(cx, entry);
                    }
                }
                if n > 1 {
                    if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_1.slot_delete_btn)).finger_down(&actions).is_some() {
                        self.show_delete_confirm(cx, 1);
                    } else if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_1)).finger_down(&actions).is_some() {
                        let entry = self.downloaded_models[1].clone();
                        self.close_selector(cx);
                        self.start_load_model(cx, entry);
                    }
                }
                if n > 2 {
                    if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_2.slot_delete_btn)).finger_down(&actions).is_some() {
                        self.show_delete_confirm(cx, 2);
                    } else if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_2)).finger_down(&actions).is_some() {
                        let entry = self.downloaded_models[2].clone();
                        self.close_selector(cx);
                        self.start_load_model(cx, entry);
                    }
                }
                if n > 3 {
                    if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_3.slot_delete_btn)).finger_down(&actions).is_some() {
                        self.show_delete_confirm(cx, 3);
                    } else if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_3)).finger_down(&actions).is_some() {
                        let entry = self.downloaded_models[3].clone();
                        self.close_selector(cx);
                        self.start_load_model(cx, entry);
                    }
                }
                if n > 4 {
                    if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_4.slot_delete_btn)).finger_down(&actions).is_some() {
                        self.show_delete_confirm(cx, 4);
                    } else if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_4)).finger_down(&actions).is_some() {
                        let entry = self.downloaded_models[4].clone();
                        self.close_selector(cx);
                        self.start_load_model(cx, entry);
                    }
                }
                if n > 5 {
                    if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_5.slot_delete_btn)).finger_down(&actions).is_some() {
                        self.show_delete_confirm(cx, 5);
                    } else if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_5)).finger_down(&actions).is_some() {
                        let entry = self.downloaded_models[5].clone();
                        self.close_selector(cx);
                        self.start_load_model(cx, entry);
                    }
                }
                if n > 6 {
                    if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_6.slot_delete_btn)).finger_down(&actions).is_some() {
                        self.show_delete_confirm(cx, 6);
                    } else if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_6)).finger_down(&actions).is_some() {
                        let entry = self.downloaded_models[6].clone();
                        self.close_selector(cx);
                        self.start_load_model(cx, entry);
                    }
                }
                if n > 7 {
                    if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_7.slot_delete_btn)).finger_down(&actions).is_some() {
                        self.show_delete_confirm(cx, 7);
                    } else if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_7)).finger_down(&actions).is_some() {
                        let entry = self.downloaded_models[7].clone();
                        self.close_selector(cx);
                        self.start_load_model(cx, entry);
                    }
                }
                if n > 8 {
                    if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_8.slot_delete_btn)).finger_down(&actions).is_some() {
                        self.show_delete_confirm(cx, 8);
                    } else if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_8)).finger_down(&actions).is_some() {
                        let entry = self.downloaded_models[8].clone();
                        self.close_selector(cx);
                        self.start_load_model(cx, entry);
                    }
                }
                if n > 9 {
                    if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_9.slot_delete_btn)).finger_down(&actions).is_some() {
                        self.show_delete_confirm(cx, 9);
                    } else if self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_9)).finger_down(&actions).is_some() {
                        let entry = self.downloaded_models[9].clone();
                        self.close_selector(cx);
                        self.start_load_model(cx, entry);
                    }
                }
            }

        }

        // ── Update banner buttons ──────────────────────────────────────────
        if self.ui.view(ids!(body.body_layout.update_banner.update_download_btn)).finger_down(&actions).is_some() {
            if let Some(ref info) = self.update_info {
                let _ = std::process::Command::new("open").arg(&info.download_url).spawn();
            }
        }
        if self.ui.view(ids!(body.body_layout.update_banner.update_dismiss_btn)).finger_down(&actions).is_some() {
            self.update_info = None;
            self.ui.view(ids!(body.body_layout.update_banner)).set_visible(cx, false);
            self.ui.redraw(cx);
        }

        // Handle hamburger menu click
        if self.ui.view(ids!(body.body_layout.header.hamburger_btn)).finger_down(&actions).is_some() {
            ::log::info!(">>> Hamburger button clicked! <<<");
            self.store.toggle_sidebar();
            self.update_sidebar(cx);
        }

        // Handle New Chat button click (first item in sidebar)
        // Use full path from Window root: body.content.sidebar.new_chat_btn
        let new_chat_clicked = self.ui.button(ids!(body.body_layout.content.sidebar.sidebar_scroll.new_chat_btn)).clicked(&actions);
        let chat_clicked = self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section.chat_history_btn)).finger_down(&actions).is_some();

        if new_chat_clicked {
            ::log::info!(">>> New Chat button clicked! <<<");

            // Request new chat directly on ChatApp (bypasses action dispatch timing issues)
            if let Some(mut chat_app) = self.ui.widget(ids!(body.body_layout.content.main_content.chat_with_canvas.chat_app))
                .borrow_mut::<moly_chat::screen::ChatApp>()
            {
                chat_app.request_new_chat();
            }

            // Clear A2UI canvas for the new chat
            self.pending_a2ui_json = None;
            self.clear_a2ui_canvas(cx);

            // Always show active chat view when creating new chat
            self.current_view = NavigationTarget::ActiveChat;
            self.store.set_current_view("ActiveChat");
            self.apply_view_state(cx, NavigationTarget::ActiveChat);
            self.update_sidebar_chats(cx);
        } else if chat_clicked {
            ::log::info!("Chat button clicked - opening chat history page");
            // Navigate to chat history page (blank page with "Chat History" text)
            self.navigate_to(cx, NavigationTarget::ChatHistory);
        }

        // Handle Show More button click
        if self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section.chat_history_visible.show_more_btn)).finger_down(&actions).is_some() {
            self.chat_history_expanded = !self.chat_history_expanded;
            self.update_chat_history_visibility(cx);
        }
        if self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.llm_btn)).finger_down(&actions).is_some() {
            self.navigate_to(cx, NavigationTarget::LlmHub);
        }
        if self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.vlm_btn)).finger_down(&actions).is_some() {
            self.navigate_to(cx, NavigationTarget::VlmHub);
        }
        if self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.asr_btn)).finger_down(&actions).is_some() {
            self.navigate_to(cx, NavigationTarget::AsrHub);
        }
        if self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.tts_btn)).finger_down(&actions).is_some() {
            self.navigate_to(cx, NavigationTarget::TtsHub);
        }
        if self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.image_btn)).finger_down(&actions).is_some() {
            self.navigate_to(cx, NavigationTarget::ImageHub);
        }
        if self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.video_btn)).finger_down(&actions).is_some() {
            self.navigate_to(cx, NavigationTarget::VideoHub);
        }
        if self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.settings_btn)).finger_down(&actions).is_some() {
            ::log::info!(">>> Settings button clicked! <<<");
            self.navigate_to(cx, NavigationTarget::Settings);
        }
        if self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_info_btn)).finger_down(&actions).is_some() {
            self.populate_about_page(cx);
            self.navigate_to(cx, NavigationTarget::About);
        }

        // Handle sidebar chat history item clicks
        {
            let mut sidebar_clicked: Option<usize> = None;
            macro_rules! check_sidebar {
                ($index:expr, $section:ident, $item:ident) => {
                    if sidebar_clicked.is_none() && $index < self.sidebar_chat_ids.len() {
                        if self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section.$section.$item))
                            .finger_down(&actions).is_some()
                        {
                            sidebar_clicked = Some($index);
                        }
                    }
                };
            }
            check_sidebar!(0, chat_history_visible, chat_item_0);
            check_sidebar!(1, chat_history_visible, chat_item_1);
            check_sidebar!(2, chat_history_visible, chat_item_2);
            check_sidebar!(3, chat_history_more, chat_item_3);
            check_sidebar!(4, chat_history_more, chat_item_4);
            check_sidebar!(5, chat_history_more, chat_item_5);

            if let Some(idx) = sidebar_clicked {
                let chat_id = self.sidebar_chat_ids[idx];
                self.store.chats.set_current_chat(Some(chat_id));
                if let Some(mut chat_app) = self.ui.widget(ids!(body.body_layout.content.main_content.chat_with_canvas.chat_app))
                    .borrow_mut::<moly_chat::screen::ChatApp>()
                {
                    chat_app.load_chat(chat_id);
                }
                self.current_view = NavigationTarget::ActiveChat;
                self.store.set_current_view("ActiveChat");
                self.apply_view_state(cx, NavigationTarget::ActiveChat);
            }
        }

        // Handle chat tile clicks
        self.handle_chat_tile_clicks(cx, actions);

        // Handle search input changes
        let search_input = self.ui.text_input(ids!(body.body_layout.content.main_content.chat_history_page.search_container.search_input));
        if search_input.changed(&actions).is_some() {
            self.search_query = search_input.text();
            self.update_chat_tiles(cx);
        }

        // Handle canvas reopen strip (shown when canvas is collapsed)
        if self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_reopen_btn)).finger_down(&actions).is_some() {
            ::log::info!(">>> Canvas reopen strip clicked! <<<");
            self.toggle_canvas_panel(cx);
        }

        // Handle canvas collapse strip (shown when canvas is expanded)
        if self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_section.canvas_toggle_column)).finger_down(&actions).is_some() {
            ::log::info!(">>> Canvas collapse strip clicked! <<<");
            self.toggle_canvas_panel(cx);
        }

        // Handle canvas splitter drag start
        let splitter = self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_splitter));
        if let Some(fd) = splitter.finger_down(&actions) {
            if !self.canvas_panel_collapsed {
                self.splitter_dragging = true;
                self.splitter_drag_start_x = fd.abs.x;
                self.splitter_drag_start_width = self.canvas_panel_width;
                ::log::info!("Splitter drag started at x={}", fd.abs.x);
            }
        }

        // Handle navigation requests from child widgets
        for action in actions {
            if let StoreAction::Navigate(view) = action.cast() {
                let target = match view.as_str() {
                    "ActiveChat"  => Some(NavigationTarget::ActiveChat),
                    "ChatHistory" => Some(NavigationTarget::ChatHistory),
                    "Settings"    => Some(NavigationTarget::Settings),
                    "LlmHub"   => Some(NavigationTarget::LlmHub),
                    "VlmHub"   => Some(NavigationTarget::VlmHub),
                    "AsrHub"   => Some(NavigationTarget::AsrHub),
                    "TtsHub"   => Some(NavigationTarget::TtsHub),
                    "ImageHub" => Some(NavigationTarget::ImageHub),
                    "VideoHub" => Some(NavigationTarget::VideoHub),
                    _ => None,
                };
                if let Some(t) = target {
                    ::log::info!("StoreAction::Navigate({}) → {:?}", view, t);
                    self.navigate_to(cx, t);
                }
            }
            // Handle hub load/unload notifications — sync the top model selector bar
            if let StoreAction::HubModelLoaded { model_id, model_name, category } = action.cast() {
                ::log::info!("HubModelLoaded: {} ({})", model_name, model_id);
                self.loaded_model_id      = model_id;
                self.loaded_model_name    = model_name;
                self.loaded_model_category = Some(category);
                self.shell_load_state     = ShellModelLoadState::Loaded;
                self.load_rx              = None; // clear any shell-level load
                self.update_selector_bar(cx);
                self.refresh_downloaded_models();
            }
            if let StoreAction::HubModelUnloaded { model_id } = action.cast() {
                // Only clear if the unloaded model matches what the shell shows
                if self.loaded_model_id == model_id {
                    ::log::info!("HubModelUnloaded: {}", model_id);
                    self.loaded_model_id       = String::new();
                    self.loaded_model_name     = String::new();
                    self.loaded_model_category = None;
                    self.shell_load_state      = ShellModelLoadState::Unloaded;
                    self.update_selector_bar(cx);
                    self.refresh_downloaded_models();
                }
            }
            // Handle "Open in Chat" from Model Hub — create new chat with the selected model
            if let StoreAction::OpenChatWithModel { model_id, category } = action.cast() {
                ::log::info!(">>> OpenChatWithModel: {} ({:?}) <<<", model_id, category);
                // Set category BEFORE injecting model (capabilities depend on category)
                self.store.set_active_local_model_category(Some(category));
                self.store.set_active_local_model(Some(model_id.clone()));
                // Request a new chat session
                if let Some(mut chat_app) = self.ui.widget(ids!(body.body_layout.content.main_content.chat_with_canvas.chat_app))
                    .borrow_mut::<moly_chat::screen::ChatApp>()
                {
                    chat_app.request_new_chat();
                }
                self.navigate_to(cx, NavigationTarget::ActiveChat);
                self.update_sidebar_chats(cx);
            }
        }

        // Refresh sidebar when ChatApp creates a new chat (deferred from request_new_chat)
        for action in actions {
            if let moly_chat::screen::ChatHistoryAction::ChatCreated = action.cast() {
                ::log::info!("ChatHistoryAction::ChatCreated — refreshing sidebar");
                self.update_sidebar_chats(cx);
            }
        }

        // Handle A2UI toggle from PromptInput and A2UI tool calls from Chat
        for action in actions {
            if let PromptInputAction::A2uiToggled(enabled) = action.cast() {
                ::log::info!("A2UI toggled: {}", enabled);
                self.a2ui_enabled = enabled;
                if enabled {
                    // Auto-expand canvas panel when A2UI is enabled
                    if self.canvas_panel_collapsed {
                        self.toggle_canvas_panel(cx);
                    }
                } else {
                    // Auto-collapse canvas panel when A2UI is disabled
                    if !self.canvas_panel_collapsed {
                        self.toggle_canvas_panel(cx);
                    }
                    // Clear pending A2UI JSON when disabled
                    self.pending_a2ui_json = None;
                }
            }

            // Handle A2UI JSON from Chat widget
            match action.cast() {
                ChatAction::A2uiJson(json) => {
                    ::log::info!(
                        "Received A2UI JSON ({} bytes)",
                        json.len()
                    );
                    self.pending_a2ui_json = Some(json);
                    self.render_a2ui_canvas(cx);
                }
                ChatAction::A2uiToggled(enabled) => {
                    ::log::info!(
                        "ChatAction::A2uiToggled({})",
                        enabled
                    );
                }
                ChatAction::None => {}
            }

            // Handle A2UI surface data model changes (two-way binding)
            if let A2uiSurfaceAction::DataModelChanged {
                surface_id, path, value
            } = action.cast() {
                ::log::info!(
                    "A2UI DataModelChanged: surface={}, path={}, value={}",
                    surface_id, path, value
                );
                let surface_ref = self.ui.widget(ids!(
                    body.content.main_content.chat_with_canvas
                        .canvas_section.canvas_content
                        .canvas_area.a2ui_surface
                ));
                if let Some(mut surface) =
                    surface_ref.borrow_mut::<A2uiSurface>()
                {
                    if let Some(processor) = surface.processor_mut() {
                        if let Some(data_model) =
                            processor.get_data_model_mut(&surface_id)
                        {
                            data_model.set(&path, value);
                        }
                    }
                }
                self.ui.redraw(cx);
            }
        }

        // Poll global state for pending A2UI JSON
        // (action propagation from nested Chat widget doesn't reach here)
        if let Some(json) = take_pending_a2ui_json() {
            ::log::info!(
                "Picked up pending A2UI JSON from global state ({} bytes)",
                json.len()
            );
            self.pending_a2ui_json = Some(json);
            self.render_a2ui_canvas(cx);
        }
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        // Handle splitter dragging with global mouse events
        if self.splitter_dragging {
            match event {
                Event::MouseMove(mm) => {
                    // Dragging left (negative delta) should increase canvas width
                    // Dragging right (positive delta) should decrease canvas width
                    let delta = mm.abs.x - self.splitter_drag_start_x;
                    let new_width = (self.splitter_drag_start_width - delta).max(200.0).min(1200.0);
                    self.canvas_panel_width = new_width;

                    self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_section))
                        .apply_over(cx, live!{ width: (new_width) });
                    self.ui.redraw(cx);
                }
                Event::MouseUp(_) => {
                    self.splitter_dragging = false;
                    ::log::info!("Splitter drag ended, width={}", self.canvas_panel_width);
                }
                _ => {}
            }
        }

        // Poll RAM usage on timer + refresh sidebar chat titles
        if self.ram_timer.is_event(event).is_some() {
            self.poll_ram_usage(cx);
            self.update_sidebar_chats(cx);
        }

        // Poll model load thread for completion
        self.poll_load_result(cx);

        // Poll update checker for result
        self.poll_update_result(cx);

        // Pass Store to child widgets via Scope
        // TODO: Migrate apps to use MolyAppData instead of Store directly
        // For now, we pass Store for backwards compatibility
        // IMPORTANT: ui.handle_event must be called BEFORE match_event
        // because actions are generated during handle_event and then
        // processed by match_event's handle_actions
        let scope = &mut Scope::with_data(&mut self.store);
        self.ui.handle_event(cx, event, scope);


        // Process actions after they've been generated
        self.match_event(cx, event);
    }
}

impl App {
    // ── Model selector methods ────────────────────────────────────────────────

    /// Scan the registry for downloaded models and cache the list.
    fn refresh_downloaded_models(&mut self) {
        let registry = ModelRegistry::load();
        self.downloaded_models = registry.models.iter()
            .filter(|m| shell_is_model_downloaded(m))
            .map(|m| DownloadedModelEntry {
                registry_id:     m.id.clone(),
                name:            m.name.clone(),
                api_model_id:    m.runtime.api_model_id.clone(),
                category:        m.category,
                model_type_str:  category_to_model_type(m.category),
                size_display:    m.storage.size_display.clone(),
                local_path:      m.storage.expanded_path(),
                supports_images: m.runtime.supports_images,
            })
            .collect();
        ::log::info!("Model selector: {} downloaded models", self.downloaded_models.len());
    }

    /// Open the dropdown and populate slots.
    fn open_selector(&mut self, cx: &mut Cx) {
        if self.shell_load_state == ShellModelLoadState::Loading { return; }
        self.refresh_downloaded_models();
        self.selector_open = true;
        self.delete_confirm_index = None;
        self.update_dropdown_slots(cx);
        self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.delete_confirm_panel)).set_visible(cx, false);
        self.ui.view(ids!(body.model_selector_dropdown)).set_visible(cx, true);
        self.ui.redraw(cx);
    }

    /// Close the dropdown.
    fn close_selector(&mut self, cx: &mut Cx) {
        self.selector_open = false;
        self.delete_confirm_index = None;
        self.ui.view(ids!(body.model_selector_dropdown)).set_visible(cx, false);
        self.ui.redraw(cx);
    }

    /// Populate the About page with model info from the registry.
    fn populate_about_page(&mut self, cx: &mut Cx) {
        let registry = ModelRegistry::load();

        struct CatInfo {
            cat: RegistryCategory,
            header_id: &'static [LiveId],
            body_id: &'static [LiveId],
            title: &'static str,
        }
        let cats = [
            CatInfo { cat: RegistryCategory::Llm,      header_id: &[live_id!(about_llm_header)],   body_id: &[live_id!(about_llm_body)],   title: "LLM \u{2014} Large Language Models" },
            CatInfo { cat: RegistryCategory::Vlm,      header_id: &[live_id!(about_vlm_header)],   body_id: &[live_id!(about_vlm_body)],   title: "VLM \u{2014} Vision Language Models" },
            CatInfo { cat: RegistryCategory::Asr,      header_id: &[live_id!(about_asr_header)],   body_id: &[live_id!(about_asr_body)],   title: "ASR \u{2014} Speech Recognition" },
            CatInfo { cat: RegistryCategory::Tts,      header_id: &[live_id!(about_tts_header)],   body_id: &[live_id!(about_tts_body)],   title: "TTS \u{2014} Text to Speech" },
            CatInfo { cat: RegistryCategory::ImageGen, header_id: &[live_id!(about_image_header)], body_id: &[live_id!(about_image_body)], title: "Image Generation" },
            CatInfo { cat: RegistryCategory::VideoGen, header_id: &[live_id!(about_video_header)], body_id: &[live_id!(about_video_body)], title: "Video Generation" },
        ];

        let page = self.ui.view(ids!(body.body_layout.content.main_content.about_page));
        for ci in &cats {
            // Collect unique base models
            let mut seen = std::collections::HashSet::new();
            let mut lines: Vec<String> = Vec::new();
            for m in &registry.models {
                if m.category != ci.cat { continue; }
                // Strip quant suffix for grouping
                let base = {
                    let n = &m.name;
                    if let Some(i) = n.rfind(" (Q") { n[..i].to_string() }
                    else if let Some(i) = n.rfind(" (FP") { n[..i].to_string() }
                    else if let Some(i) = n.rfind(" (BF") { n[..i].to_string() }
                    else { n.clone() }
                };
                if !seen.insert(base.clone()) { continue; }
                let variants: Vec<&str> = registry.models.iter()
                    .filter(|v| v.category == ci.cat && {
                        let vn = &v.name;
                        let vbase = if let Some(i) = vn.rfind(" (Q") { vn[..i].to_string() }
                            else if let Some(i) = vn.rfind(" (FP") { vn[..i].to_string() }
                            else if let Some(i) = vn.rfind(" (BF") { vn[..i].to_string() }
                            else { vn.clone() };
                        vbase == base
                    })
                    .filter_map(|v| v.runtime.quantization.as_deref())
                    .collect();
                let size = &m.storage.size_display;
                if variants.is_empty() {
                    lines.push(format!("\u{2022} {} ({})", base, size));
                } else {
                    lines.push(format!("\u{2022} {} [{}] (from {})", base, variants.join(", "), size));
                }
            }
            let body_text = if lines.is_empty() {
                "Coming soon.".to_string()
            } else {
                lines.join("\n")
            };

            page.label(ci.header_id).set_text(cx, ci.title);
            page.label(ci.body_id).set_text(cx, &body_text);
        }
    }

    /// Update selector pill label and eject-button visibility.
    fn update_selector_bar(&mut self, cx: &mut Cx) {
        let label_text = match self.shell_load_state {
            ShellModelLoadState::Unloaded => "No model loaded".to_string(),
            ShellModelLoadState::Loading  => format!("Loading {}...", self.loaded_model_name),
            ShellModelLoadState::Loaded   => self.loaded_model_name.clone(),
            ShellModelLoadState::Error    => "Load failed — click to retry".to_string(),
        };
        let loaded = matches!(self.shell_load_state, ShellModelLoadState::Loaded);

        self.ui.label(ids!(body.body_layout.header.model_selector_btn.selector_label))
            .set_text(cx, &label_text);

        let loaded_val = if loaded { 1.0 } else { 0.0 };
        self.ui.view(ids!(body.body_layout.header.model_selector_btn.selector_dot))
            .apply_over(cx, live!{ draw_bg: { loaded: (loaded_val) } });

        self.ui.view(ids!(body.body_layout.header.eject_btn))
            .set_visible(cx, loaded);

        // Category tag — show with correct type/color when a model is loaded
        let tag = self.ui.view(ids!(body.body_layout.header.model_selector_btn.category_tag));
        tag.set_visible(cx, loaded);
        if loaded {
            let cat_val = registry_category_as_f64(
                self.loaded_model_category.unwrap_or(RegistryCategory::Llm)
            );
            let cat_label = match self.loaded_model_category {
                Some(RegistryCategory::Llm)      => "LLM",
                Some(RegistryCategory::Vlm)      => "VLM",
                Some(RegistryCategory::Asr)      => "ASR",
                Some(RegistryCategory::Tts)      => "TTS",
                Some(RegistryCategory::ImageGen) => "Image",
                Some(RegistryCategory::VideoGen) => "Video",
                None => "LLM",
            };
            tag.apply_over(cx, live! { draw_bg: { cat: (cat_val) } });
            tag.label(ids!(category_tag_label)).set_text(cx, cat_label);
            tag.label(ids!(category_tag_label)).apply_over(cx, live! { draw_text: { cat: (cat_val) } });
        }
    }

    /// Write data into a single dropdown slot widget.
    fn update_slot_view(&self, cx: &mut Cx, slot: WidgetRef, entry: &DownloadedModelEntry) {
        slot.set_visible(cx, true);
        slot.label(ids!(slot_name)).set_text(cx, &entry.name);
        slot.label(ids!(slot_meta)).set_text(cx, &entry.size_display);

        // Show/hide "Loaded" tag
        let is_loaded = self.loaded_model_id == entry.registry_id;
        slot.view(ids!(slot_loaded_tag)).set_visible(cx, is_loaded);

        // Set type tag label and color per category
        let type_label = entry.category.label();
        slot.label(ids!(slot_type_tag.slot_type_label)).set_text(cx, type_label);

        let tag = slot.view(ids!(slot_type_tag));
        let label = slot.label(ids!(slot_type_tag.slot_type_label));
        // Color-coded type tags (bg, text) per category
        // Avoid green and red — reserved for loaded/error states
        let (bg, fg) = match entry.category {
            RegistryCategory::Llm      => (vec4(0.867, 0.839, 0.996, 1.0), vec4(0.357, 0.129, 0.714, 1.0)), // purple
            RegistryCategory::Vlm      => (vec4(0.878, 0.906, 1.000, 1.0), vec4(0.216, 0.188, 0.639, 1.0)), // indigo
            RegistryCategory::Asr      => (vec4(0.855, 0.871, 0.996, 1.0), vec4(0.188, 0.220, 0.573, 1.0)), // slate-blue
            RegistryCategory::Tts      => (vec4(0.984, 0.929, 0.835, 1.0), vec4(0.573, 0.357, 0.082, 1.0)), // amber
            RegistryCategory::ImageGen => (vec4(0.953, 0.878, 0.957, 1.0), vec4(0.502, 0.145, 0.502, 1.0)), // magenta
            RegistryCategory::VideoGen => (vec4(0.835, 0.918, 0.996, 1.0), vec4(0.114, 0.318, 0.573, 1.0)), // blue
        };
        tag.apply_over(cx, live! { draw_bg: { color: (bg) } });
        label.apply_over(cx, live! { draw_text: { color: (fg) } });
    }

    /// Populate/hide all 10 dropdown slots from `self.downloaded_models`.
    fn update_dropdown_slots(&mut self, cx: &mut Cx) {
        let models = self.downloaded_models.clone();
        let n = models.len();

        self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.empty_state))
            .set_visible(cx, n == 0);
        self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll))
            .set_visible(cx, n > 0);

        // Explicit per-slot update (ids!() can't be in macro_rules)
        let slot = self.ui.widget(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_0));
        if n > 0 { self.update_slot_view(cx, slot, &models[0]); } else { slot.set_visible(cx, false); }

        let slot = self.ui.widget(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_1));
        if n > 1 { self.update_slot_view(cx, slot, &models[1]); } else { slot.set_visible(cx, false); }

        let slot = self.ui.widget(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_2));
        if n > 2 { self.update_slot_view(cx, slot, &models[2]); } else { slot.set_visible(cx, false); }

        let slot = self.ui.widget(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_3));
        if n > 3 { self.update_slot_view(cx, slot, &models[3]); } else { slot.set_visible(cx, false); }

        let slot = self.ui.widget(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_4));
        if n > 4 { self.update_slot_view(cx, slot, &models[4]); } else { slot.set_visible(cx, false); }

        let slot = self.ui.widget(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_5));
        if n > 5 { self.update_slot_view(cx, slot, &models[5]); } else { slot.set_visible(cx, false); }

        let slot = self.ui.widget(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_6));
        if n > 6 { self.update_slot_view(cx, slot, &models[6]); } else { slot.set_visible(cx, false); }

        let slot = self.ui.widget(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_7));
        if n > 7 { self.update_slot_view(cx, slot, &models[7]); } else { slot.set_visible(cx, false); }

        let slot = self.ui.widget(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_8));
        if n > 8 { self.update_slot_view(cx, slot, &models[8]); } else { slot.set_visible(cx, false); }

        let slot = self.ui.widget(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll.slot_9));
        if n > 9 { self.update_slot_view(cx, slot, &models[9]); } else { slot.set_visible(cx, false); }
    }

    /// Show delete confirmation for the given model index.
    fn show_delete_confirm(&mut self, cx: &mut Cx, index: usize) {
        if index >= self.downloaded_models.len() { return; }
        self.delete_confirm_index = Some(index);
        let name = self.downloaded_models[index].name.clone();
        self.ui.label(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.delete_confirm_panel.confirm_msg))
            .set_text(cx, &format!("Delete {}?", name));
        self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.model_scroll)).set_visible(cx, false);
        self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.empty_state)).set_visible(cx, false);
        self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.delete_confirm_panel)).set_visible(cx, true);
        self.ui.redraw(cx);
    }

    /// Hide delete confirmation and restore the model list.
    fn hide_delete_confirm(&mut self, cx: &mut Cx) {
        self.delete_confirm_index = None;
        self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.delete_confirm_panel)).set_visible(cx, false);
        self.update_dropdown_slots(cx);
        self.ui.redraw(cx);
    }

    /// Perform the model deletion after confirmation.
    fn perform_delete_model(&mut self, cx: &mut Cx) {
        let Some(idx) = self.delete_confirm_index.take() else { return };
        if idx >= self.downloaded_models.len() { return; }

        let entry = self.downloaded_models[idx].clone();

        // If this model is currently loaded, unload it first
        if self.loaded_model_id == entry.registry_id {
            self.start_unload_model(cx);
        }

        // Optimistically remove from the list
        self.downloaded_models.remove(idx);

        // Delete files in background
        let path = entry.local_path.clone();
        std::thread::spawn(move || {
            let p = std::path::Path::new(&path);
            if p.exists() {
                if let Err(e) = std::fs::remove_dir_all(p) {
                    ::log::error!("Failed to delete model at {}: {}", path, e);
                } else {
                    ::log::info!("Deleted model files at {}", path);
                }
            }
        });

        // Update UI
        self.ui.view(ids!(body.model_selector_dropdown.dropdown_wrapper.dropdown_panel.delete_confirm_panel)).set_visible(cx, false);
        self.update_dropdown_slots(cx);
        self.ui.redraw(cx);
    }

    /// Start loading a model in a background thread.
    fn start_load_model(&mut self, cx: &mut Cx, entry: DownloadedModelEntry) {
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        self.load_rx = Some(rx);
        self.shell_load_state    = ShellModelLoadState::Loading;
        self.loaded_model_id     = entry.registry_id.clone();
        self.loaded_model_name   = entry.name.clone();
        self.loaded_model_category = Some(entry.category);
        self.loaded_model_supports_images = entry.supports_images;

        let api_model_id  = entry.api_model_id.clone();
        let model_type    = entry.model_type_str.to_string();

        std::thread::spawn(move || {
            let result = ensure_server_running()
                .and_then(|()| {
                    if model_type == "video" || model_type == "video_generation" {
                        return Ok(());
                    }
                    ModelRuntimeClient::localhost().load_model(&api_model_id, &model_type)
                });
            let _ = tx.send(result);
        });

        self.update_selector_bar(cx);
        self.ui.redraw(cx);
    }

    /// Optimistically unload the current model (fire-and-forget).
    fn start_unload_model(&mut self, cx: &mut Cx) {
        let model_type = match self.loaded_model_category {
            Some(RegistryCategory::Llm)      => "llm",
            Some(RegistryCategory::Vlm)      => "vlm",
            Some(RegistryCategory::Asr)      => "asr",
            Some(RegistryCategory::Tts)      => "tts",
            Some(RegistryCategory::ImageGen) => "image",
            Some(RegistryCategory::VideoGen) => "video",
            None                             => "all",
        }.to_string();

        std::thread::spawn(move || {
            ModelRuntimeClient::localhost().unload_model(&model_type).ok();
        });

        // Optimistic UI reset
        self.shell_load_state    = ShellModelLoadState::Unloaded;
        self.loaded_model_id     = String::new();
        self.loaded_model_name   = String::new();
        self.loaded_model_category = None;
        self.store.set_active_local_model(None);

        self.update_selector_bar(cx);
        self.ui.redraw(cx);
    }

    /// Poll the load thread; navigate on success, report on failure.
    fn poll_load_result(&mut self, cx: &mut Cx) {
        let result = self.load_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        let Some(result) = result else { return };
        self.load_rx = None;

        match result {
            Ok(()) => {
                self.shell_load_state = ShellModelLoadState::Loaded;

                // Set category + capabilities BEFORE injecting model
                self.store.set_active_local_model_category(self.loaded_model_category);
                self.store.set_active_local_model_supports_images(self.loaded_model_supports_images);
                let model_id = self.loaded_model_id.clone();
                self.store.set_active_local_model(Some(model_id));

                // All model types go to Chat after loading
                if let Some(mut chat_app) = self.ui
                    .widget(ids!(body.body_layout.content.main_content.chat_with_canvas.chat_app))
                    .borrow_mut::<moly_chat::screen::ChatApp>()
                {
                    chat_app.request_new_chat();
                }

                self.navigate_to(cx, NavigationTarget::ActiveChat);

                self.update_sidebar_chats(cx);
            }
            Err(e) => {
                self.shell_load_state    = ShellModelLoadState::Error;
                self.loaded_model_id     = String::new();
                self.loaded_model_name   = String::new();
                self.loaded_model_category = None;
                ::log::error!("Model load failed: {}", e);
            }
        }

        self.update_selector_bar(cx);
        self.ui.redraw(cx);
    }

    fn navigate_to(&mut self, cx: &mut Cx, target: NavigationTarget) {
        ::log::info!("navigate_to: current={:?}, target={:?}", self.current_view, target);
        self.current_view = target;

        // Persist to Store
        let view_name = match target {
            NavigationTarget::ChatHistory => "ChatHistory",
            NavigationTarget::ActiveChat  => "ActiveChat",
            NavigationTarget::Settings    => "Settings",
            NavigationTarget::LlmHub      => "LlmHub",
            NavigationTarget::VlmHub      => "VlmHub",
            NavigationTarget::AsrHub      => "AsrHub",
            NavigationTarget::TtsHub      => "TtsHub",
            NavigationTarget::ImageHub    => "ImageHub",
            NavigationTarget::VideoHub    => "VideoHub",
            NavigationTarget::About       => "About",
        };
        self.store.set_current_view(view_name);

        self.apply_view_state(cx, target);
    }

    /// Apply UI state for the given view (visibility and button selection)
    fn apply_view_state(&mut self, cx: &mut Cx, target: NavigationTarget) {
        // Update app visibility
        // Chat history page and active chat are mutually exclusive
        let show_chat_history = target == NavigationTarget::ChatHistory;
        let show_active_chat = target == NavigationTarget::ActiveChat;

        self.ui.widget(ids!(body.body_layout.content.main_content.chat_history_page)).set_visible(cx, show_chat_history);
        self.ui.widget(ids!(body.body_layout.content.main_content.chat_with_canvas)).set_visible(cx, show_active_chat);
        self.ui.widget(ids!(body.body_layout.content.main_content.llm_hub_app)).set_visible(cx, target == NavigationTarget::LlmHub);
        self.ui.widget(ids!(body.body_layout.content.main_content.vlm_hub_app)).set_visible(cx, target == NavigationTarget::VlmHub);
        self.ui.widget(ids!(body.body_layout.content.main_content.asr_hub_app)).set_visible(cx, target == NavigationTarget::AsrHub);
        self.ui.widget(ids!(body.body_layout.content.main_content.tts_hub_app)).set_visible(cx, target == NavigationTarget::TtsHub);
        self.ui.widget(ids!(body.body_layout.content.main_content.image_hub_app)).set_visible(cx, target == NavigationTarget::ImageHub);
        self.ui.widget(ids!(body.body_layout.content.main_content.video_hub_app)).set_visible(cx, target == NavigationTarget::VideoHub);
        self.ui.widget(ids!(body.body_layout.content.main_content.settings_app)).set_visible(cx, target == NavigationTarget::Settings);
        self.ui.widget(ids!(body.body_layout.content.main_content.about_page)).set_visible(cx, target == NavigationTarget::About);

        // Notify ChatApp when it becomes visible (to refresh model list)
        if show_active_chat {
            if let Some(mut chat_app) = self.ui.widget(ids!(body.body_layout.content.main_content.chat_with_canvas.chat_app)).borrow_mut::<moly_chat::screen::ChatApp>() {
                chat_app.on_become_visible();
            }
        }

        // Update chat tiles when showing chat history
        if show_chat_history {
            self.update_chat_tiles(cx);
        }

        // Update button selection state (SidebarButton is a Button with draw_bg.selected)
        // Chat button is selected for both ChatHistory and ActiveChat
        let chat_selected = show_chat_history || show_active_chat;
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section.chat_history_btn)).apply_over(cx, live! {
            draw_bg: { selected: (if chat_selected { 1.0 } else { 0.0 }) }
        });
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.llm_btn)).apply_over(cx, live! {
            draw_bg: { selected: (if target == NavigationTarget::LlmHub { 1.0 } else { 0.0 }) }
        });
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.vlm_btn)).apply_over(cx, live! {
            draw_bg: { selected: (if target == NavigationTarget::VlmHub { 1.0 } else { 0.0 }) }
        });
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.asr_btn)).apply_over(cx, live! {
            draw_bg: { selected: (if target == NavigationTarget::AsrHub { 1.0 } else { 0.0 }) }
        });
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.tts_btn)).apply_over(cx, live! {
            draw_bg: { selected: (if target == NavigationTarget::TtsHub { 1.0 } else { 0.0 }) }
        });
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.image_btn)).apply_over(cx, live! {
            draw_bg: { selected: (if target == NavigationTarget::ImageHub { 1.0 } else { 0.0 }) }
        });
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.video_btn)).apply_over(cx, live! {
            draw_bg: { selected: (if target == NavigationTarget::VideoHub { 1.0 } else { 0.0 }) }
        });
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.settings_btn)).apply_over(cx, live! {
            draw_bg: { selected: (if target == NavigationTarget::Settings { 1.0 } else { 0.0 }) }
        });

        self.ui.redraw(cx);
    }

    fn update_sidebar(&mut self, cx: &mut Cx) {
        let expanded = self.store.is_sidebar_expanded();
        let width = if expanded { 250.0 } else { 60.0 };

        self.ui.view(ids!(body.body_layout.content.sidebar)).apply_over(cx, live! {
            width: (width)
        });

        // Hide section label views and chat history sublist when sidebar is collapsed to icon-only mode
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section_label)).set_visible(cx, expanded);
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.models_section_label)).set_visible(cx, expanded);
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section.chat_history_visible)).set_visible(cx, expanded);

        self.ui.redraw(cx);
    }

    /// Populate sidebar chat history items (items 0-5) from the Store.
    /// Called on startup, after new chat creation, and after chat deletion.
    fn update_sidebar_chats(&mut self, cx: &mut Cx) {
        let chats: Vec<_> = self.store.chats.get_sorted_chats()
            .into_iter()
            .filter(|c| !c.messages.is_empty())
            .take(6)
            .collect();
        let n = chats.len();
        self.sidebar_chat_ids = chats.iter().map(|c| c.id).collect();

        macro_rules! update_item {
            ($index:expr, $section:ident, $item:ident) => {
                let vis = $index < n;
                self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section.$section.$item))
                    .set_visible(cx, vis);
                if vis {
                    // Sanitize: collapse newlines, truncate to single display line
                    let raw = &chats[$index].title;
                    let single: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
                    let display = if single.chars().count() > 28 {
                        let head: String = single.chars().take(26).collect();
                        format!("{}…", head)
                    } else {
                        single
                    };
                    self.ui.label(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section.$section.$item.title))
                        .set_text(cx, &display);
                }
            };
        }

        update_item!(0, chat_history_visible, chat_item_0);
        update_item!(1, chat_history_visible, chat_item_1);
        update_item!(2, chat_history_visible, chat_item_2);
        update_item!(3, chat_history_more, chat_item_3);
        update_item!(4, chat_history_more, chat_item_4);
        update_item!(5, chat_history_more, chat_item_5);

        // Only show "Show More" when there are more than 3 chats
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section.chat_history_visible.show_more_btn))
            .set_visible(cx, n > 3);

        self.ui.redraw(cx);
    }

    /// Update chat history visibility based on expanded state
    fn update_chat_history_visibility(&mut self, cx: &mut Cx) {
        // Update "Show More" section visibility
        self.ui.view(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section.chat_history_more)).set_visible(cx, self.chat_history_expanded);

        // Update "Show More" button text and arrow
        let (text, arrow) = if self.chat_history_expanded {
            ("Show Less", "v")
        } else {
            ("Show More", ">")
        };
        self.ui.label(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section.chat_history_visible.show_more_label)).set_text(cx, text);
        self.ui.label(ids!(body.body_layout.content.sidebar.sidebar_scroll.chat_section.chat_history_visible.show_more_arrow)).set_text(cx, arrow);

        self.ui.redraw(cx);
    }

    /// Toggle the canvas panel visibility (slide in/out)
    fn toggle_canvas_panel(&mut self, cx: &mut Cx) {
        self.canvas_panel_collapsed = !self.canvas_panel_collapsed;

        // Initialize width if not set (default to 500px)
        if self.canvas_panel_width == 0.0 {
            self.canvas_panel_width = 500.0;
        }

        if self.canvas_panel_collapsed {
            // Collapse: hide canvas section, show reopen strip
            self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_section))
                .set_visible(cx, false);
            self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_splitter))
                .apply_over(cx, live!{ width: 0 });
            self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_reopen_btn))
                .set_visible(cx, true);
        } else {
            // Expand: show canvas section at saved width, hide reopen strip
            let width = self.canvas_panel_width;
            self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_reopen_btn))
                .set_visible(cx, false);
            self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_section))
                .set_visible(cx, true);
            self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_section))
                .apply_over(cx, live!{ width: (width) });
            self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_section.canvas_content))
                .set_visible(cx, true);
            self.ui.view(ids!(body.body_layout.content.main_content.chat_with_canvas.canvas_splitter))
                .apply_over(cx, live!{ width: 16 });
        }

        self.ui.redraw(cx);
    }

    /// Render A2UI components in the canvas area from JSON.
    ///
    /// Takes A2UI JSON (generated by the LLM as structured output) and feeds it
    /// directly to the A2uiSurface for rendering.
    fn render_a2ui_canvas(&mut self, cx: &mut Cx) {
        let Some(json_str) = self.pending_a2ui_json.take() else {
            return;
        };

        let preview_end = json_str
            .char_indices()
            .take_while(|(i, _)| *i < 600)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(json_str.len().min(600));
        eprintln!(
            "[A2UI render] JSON ({} bytes), first ~600 chars:\n{}",
            json_str.len(),
            &json_str[..preview_end]
        );
        // Dump full JSON to temp file for debugging
        let _ = std::fs::write("/tmp/a2ui_last_json.txt", &json_str);

        // Test: can serde parse it as generic JSON?
        match serde_json::from_str::<serde_json::Value>(&json_str) {
            Ok(val) => {
                let kind = if val.is_array() {
                    format!("array of {}", val.as_array().unwrap().len())
                } else if val.is_object() {
                    "object".to_string()
                } else {
                    "other".to_string()
                };
                eprintln!("[A2UI render] JSON parses as generic Value: {}", kind);
            }
            Err(e) => {
                eprintln!(
                    "[A2UI render] JSON fails as generic Value: {}",
                    e
                );
                // Dump the problematic area around line 10
                let lines: Vec<&str> = json_str.lines().collect();
                let start = if lines.len() > 7 { 7 } else { 0 };
                let end = if lines.len() > 13 { 13 } else { lines.len() };
                for (i, line) in lines[start..end].iter().enumerate() {
                    eprintln!("  line {}: {}", start + i + 1, line);
                }
            }
        }

        let surface_ref = self.ui.widget(ids!(
            body.content.main_content.chat_with_canvas
                .canvas_section.canvas_content
                .canvas_area.a2ui_surface
        ));
        if let Some(mut surface) = surface_ref.borrow_mut::<A2uiSurface>() {
            surface.clear();
            match surface.process_json(&json_str) {
                Ok(events) => {
                    eprintln!(
                        "[A2UI render] Surface processed {} events",
                        events.len()
                    );
                }
                Err(e) => {
                    eprintln!("[A2UI render] Surface parse error: {}", e);
                }
            }
        } else {
            eprintln!("[A2UI render] Could not borrow A2uiSurface");
        }

        self.ui.redraw(cx);
    }

    /// Clear the A2UI canvas surface.
    fn clear_a2ui_canvas(&mut self, cx: &mut Cx) {
        let surface_ref = self.ui.widget(ids!(
            body.content.main_content.chat_with_canvas
                .canvas_section.canvas_content
                .canvas_area.a2ui_surface
        ));
        if let Some(mut surface) =
            surface_ref.borrow_mut::<A2uiSurface>()
        {
            surface.clear();
        }
        self.ui.redraw(cx);
    }

    /// Update the chat history tiles with data from Store
    fn hex_to_vec4(hex: &str) -> Vec4 {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
        Vec4 { x: r, y: g, z: b, w: 1.0 }
    }

    fn update_chat_tiles(&mut self, cx: &mut Cx) {
        // Only show chats that have messages (filter out empty chats)
        // Also filter by search query if present
        let search_lower = self.search_query.to_lowercase();
        let chats: Vec<_> = self.store.chats.get_sorted_chats()
            .into_iter()
            .filter(|c| !c.messages.is_empty())
            .filter(|c| {
                if search_lower.is_empty() {
                    return true;
                }
                // Check title
                if c.title.to_lowercase().contains(&search_lower) {
                    return true;
                }
                // Check message content
                c.messages.iter().any(|m| m.content.text.to_lowercase().contains(&search_lower))
            })
            .collect();
        let chat_count = chats.len().min(12); // Max 12 tiles

        // Update displayed_chat_ids
        self.displayed_chat_ids = chats.iter().take(12).map(|c| c.id).collect();

        // Show/hide empty state and scroll container
        let has_chats = chat_count > 0;
        self.ui.view(ids!(body.body_layout.content.main_content.chat_history_page.empty_state)).set_visible(cx, !has_chats);
        self.ui.view(ids!(body.body_layout.content.main_content.chat_history_page.chat_tiles_scroll)).set_visible(cx, has_chats);

        // Show/hide row containers based on how many chats we have
        // Row 0 visible if we have any chats (indices 0-3)
        // Row 1 visible if we have more than 4 chats (indices 4-7)
        // Row 2 visible if we have more than 8 chats (indices 8-11)
        self.ui.view(ids!(body.body_layout.content.main_content.chat_history_page.chat_tiles_scroll.chat_tiles_container.tile_row_0))
            .set_visible(cx, chat_count > 0);
        self.ui.view(ids!(body.body_layout.content.main_content.chat_history_page.chat_tiles_scroll.chat_tiles_container.tile_row_1))
            .set_visible(cx, chat_count > 4);
        self.ui.view(ids!(body.body_layout.content.main_content.chat_history_page.chat_tiles_scroll.chat_tiles_container.tile_row_2))
            .set_visible(cx, chat_count > 8);

        macro_rules! update_tile {
            ($index:expr, $row:ident, $tile:ident) => {
                let visible = $index < chat_count;
                self.ui.view(ids!(body.body_layout.content.main_content.chat_history_page.chat_tiles_scroll.chat_tiles_container.$row.$tile))
                    .set_visible(cx, visible);
                if visible {
                    let chat = chats[$index];
                    self.ui.label(ids!(body.body_layout.content.main_content.chat_history_page.chat_tiles_scroll.chat_tiles_container.$row.$tile.header.title))
                        .set_text(cx, &chat.title);
                    let date_str = chat.accessed_at.format("%b %d, %Y").to_string();
                    self.ui.label(ids!(body.body_layout.content.main_content.chat_history_page.chat_tiles_scroll.chat_tiles_container.$row.$tile.footer.date_label))
                        .set_text(cx, &date_str);

                    // Set category tag
                    let tag_view = self.ui.view(ids!(body.body_layout.content.main_content.chat_history_page.chat_tiles_scroll.chat_tiles_container.$row.$tile.footer.category_tag));
                    if let Some(cat) = chat.model_category {
                        tag_view.set_visible(cx, true);
                        self.ui.label(ids!(body.body_layout.content.main_content.chat_history_page.chat_tiles_scroll.chat_tiles_container.$row.$tile.footer.category_tag.tag_label))
                            .set_text(cx, cat.label());
                        let color = Self::hex_to_vec4(cat.color());
                        tag_view.apply_over(cx, live! { draw_bg: { color: (color) } });
                    } else {
                        tag_view.set_visible(cx, false);
                    }
                }
            };
        }

        update_tile!(0, tile_row_0, tile_0);
        update_tile!(1, tile_row_0, tile_1);
        update_tile!(2, tile_row_0, tile_2);
        update_tile!(3, tile_row_0, tile_3);
        update_tile!(4, tile_row_1, tile_0);
        update_tile!(5, tile_row_1, tile_1);
        update_tile!(6, tile_row_1, tile_2);
        update_tile!(7, tile_row_1, tile_3);
        update_tile!(8, tile_row_2, tile_0);
        update_tile!(9, tile_row_2, tile_1);
        update_tile!(10, tile_row_2, tile_2);
        update_tile!(11, tile_row_2, tile_3);

        self.ui.redraw(cx);
    }

    /// Handle chat tile clicks and delete button clicks
    fn handle_chat_tile_clicks(&mut self, cx: &mut Cx, actions: &Actions) {
        let mut tile_clicked: Option<usize> = None;
        let mut delete_clicked: Option<usize> = None;

        macro_rules! check_tile {
            ($index:expr, $row:ident, $tile:ident) => {
                if $index < self.displayed_chat_ids.len() && delete_clicked.is_none() && tile_clicked.is_none() {
                    if self.ui.view(ids!(body.body_layout.content.main_content.chat_history_page.chat_tiles_scroll.chat_tiles_container.$row.$tile.header.delete_btn))
                        .finger_down(actions).is_some() {
                        delete_clicked = Some($index);
                    }
                    else if self.ui.view(ids!(body.body_layout.content.main_content.chat_history_page.chat_tiles_scroll.chat_tiles_container.$row.$tile))
                        .finger_down(actions).is_some() {
                        tile_clicked = Some($index);
                    }
                }
            };
        }

        check_tile!(0, tile_row_0, tile_0);
        check_tile!(1, tile_row_0, tile_1);
        check_tile!(2, tile_row_0, tile_2);
        check_tile!(3, tile_row_0, tile_3);
        check_tile!(4, tile_row_1, tile_0);
        check_tile!(5, tile_row_1, tile_1);
        check_tile!(6, tile_row_1, tile_2);
        check_tile!(7, tile_row_1, tile_3);
        check_tile!(8, tile_row_2, tile_0);
        check_tile!(9, tile_row_2, tile_1);
        check_tile!(10, tile_row_2, tile_2);
        check_tile!(11, tile_row_2, tile_3);

        // Handle delete action
        if let Some(idx) = delete_clicked {
            let chat_id = self.displayed_chat_ids[idx];
            ::log::info!("Delete button clicked for chat at index {}, id={}", idx, chat_id);
            self.store.chats.delete_chat(chat_id);
            self.update_chat_tiles(cx);
            self.update_sidebar_chats(cx);
            return;
        }

        // Handle tile click (open chat)
        if let Some(idx) = tile_clicked {
            let chat_id = self.displayed_chat_ids[idx];
            ::log::info!("Chat tile clicked at index {}, id={}", idx, chat_id);

            // Set current chat in store
            self.store.chats.set_current_chat(Some(chat_id));

            // Load chat in ChatApp
            if let Some(mut chat_app) = self.ui.widget(ids!(body.body_layout.content.main_content.chat_with_canvas.chat_app))
                .borrow_mut::<moly_chat::screen::ChatApp>()
            {
                chat_app.load_chat(chat_id);
            }

            // Navigate to active chat
            self.current_view = NavigationTarget::ActiveChat;
            self.store.set_current_view("ActiveChat");
            self.apply_view_state(cx, NavigationTarget::ActiveChat);
        }
    }

    // ── Update checker ────────────────────────────────────────────────────

    fn start_update_check(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.update_rx = Some(rx);
        std::thread::spawn(move || {
            let current = env!("CARGO_PKG_VERSION");
            let result = check_for_update(current);
            let _ = tx.send(match result {
                Ok(info) => info,
                Err(e) => {
                    ::log::warn!("Update check failed: {}", e);
                    None
                }
            });
        });
    }

    fn poll_update_result(&mut self, cx: &mut Cx) {
        let result = self.update_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        let Some(info) = result else { return };
        self.update_rx = None;

        if let Some(update) = info {
            let msg = format!(
                "Moxin Studio {} is available — you have {}",
                update.version,
                env!("CARGO_PKG_VERSION"),
            );
            self.ui.label(ids!(body.body_layout.update_banner.update_label))
                .set_text(cx, &msg);
            self.ui.view(ids!(body.body_layout.update_banner))
                .set_visible(cx, true);
            self.update_info = Some(update);
            self.ui.redraw(cx);
        }
    }

    // ── RAM gauge ───────────────────────────────────────────────────────────

    fn poll_ram_usage(&mut self, cx: &mut Cx) {
        let (used, total) = get_system_ram();
        self.ram_used_gb = used;
        self.ram_total_gb = total;
        self.ram_usage = if total > 0.0 { (used / total).min(1.0) } else { 0.0 };

        self.ui.view(ids!(body.body_layout.header.ram_gauge))
            .apply_over(cx, live! { draw_bg: { usage: (self.ram_usage) } });
        self.ui.label(ids!(body.body_layout.header.ram_label))
            .set_text(cx, &format!("RAM\n{:.0}/{:.0}GB", self.ram_used_gb, self.ram_total_gb));
        self.ui.redraw(cx);
    }
}

fn get_system_ram() -> (f64, f64) {
    use std::process::Command;

    let total_bytes = Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);

    let vm_output = Command::new("vm_stat")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    let mut page_size = 16384u64;
    let mut active = 0u64;
    let mut wired = 0u64;
    let mut compressed = 0u64;

    for line in vm_output.lines() {
        if line.starts_with("Mach Virtual Memory Statistics") {
            if let Some(start) = line.find("page size of ") {
                let rest = &line[start + 13..];
                if let Some(end) = rest.find(' ') {
                    page_size = rest[..end].parse().unwrap_or(16384);
                }
            }
        } else if line.contains("Pages active") {
            active = parse_vm_stat_val(line);
        } else if line.contains("Pages wired") {
            wired = parse_vm_stat_val(line);
        } else if line.contains("Pages occupied by compressor") {
            compressed = parse_vm_stat_val(line);
        }
    }

    let used_bytes = (active + wired + compressed) * page_size;
    (
        used_bytes as f64 / 1_073_741_824.0,
        total_bytes as f64 / 1_073_741_824.0,
    )
}

fn parse_vm_stat_val(line: &str) -> u64 {
    line.split(':')
        .nth(1)
        .and_then(|v| v.trim().trim_end_matches('.').parse().ok())
        .unwrap_or(0)
}

app_main!(App);
