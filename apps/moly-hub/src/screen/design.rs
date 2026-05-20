use makepad_widgets::*;
use super::ModelHubApp;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use moly_widgets::theme::*;

    // ── Category badge (5 categories: LLM=0, VLM=1, ASR=2, TTS=3, Image=4) ──

    HubCategoryBadge = <View> {
        width: Fit, height: Fit
        padding: {left: 6, right: 6, top: 2, bottom: 2}
        margin: {left: 8}
        draw_bg: {
            instance cat: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 3.0);
                // LLM indigo, VLM violet, ASR green, TTS amber, Image pink
                let c0 = #dbeafe; // LLM bg
                let c1 = #ede9fe; // VLM bg
                let c2 = #d1fae5; // ASR bg
                let c3 = #fef3c7; // TTS bg
                let c4 = #fce7f3; // Image bg
                // Select by integer step
                let w0 = 1.0 - step(0.5, self.cat);
                let w1 = step(0.5, self.cat) * (1.0 - step(1.5, self.cat));
                let w2 = step(1.5, self.cat) * (1.0 - step(2.5, self.cat));
                let w3 = step(2.5, self.cat) * (1.0 - step(3.5, self.cat));
                let w4 = step(3.5, self.cat);
                let color = c0 * w0 + c1 * w1 + c2 * w2 + c3 * w3 + c4 * w4;
                sdf.fill(color);
                return sdf.result;
            }
        }
        cat_label = <Label> {
            draw_text: {
                instance cat: 0.0
                fn get_color(self) -> vec4 {
                    let c0 = #1a40af; // LLM
                    let c1 = #5b21b6; // VLM
                    let c2 = #047857; // ASR
                    let c3 = #92400f; // TTS
                    let c4 = #9d174d; // Image
                    let w0 = 1.0 - step(0.5, self.cat);
                    let w1 = step(0.5, self.cat) * (1.0 - step(1.5, self.cat));
                    let w2 = step(1.5, self.cat) * (1.0 - step(2.5, self.cat));
                    let w3 = step(2.5, self.cat) * (1.0 - step(3.5, self.cat));
                    let w4 = step(3.5, self.cat);
                    return c0 * w0 + c1 * w1 + c2 * w2 + c3 * w3 + c4 * w4;
                }
                text_style: <FONT_MEDIUM>{ font_size: 9.0 }
            }
        }
    }

    // ── Status dot (0=gray/not-downloaded, 1=yellow/downloading, 2=green/ready, 4=red/error) ──

    HubStatusDot = <View> {
        width: 8, height: 8
        margin: {right: 8}
        draw_bg: {
            instance status: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.circle(4.0, 4.0, 4.0);
                let gray   = #d1d5db;
                let yellow = #f59b0b;
                let green  = #22c55a;
                let blue   = #3b82f6;
                let red    = #b91c1c;
                let color = mix(gray,   yellow, clamp(self.status - 0.0, 0.0, 1.0));
                let color = mix(color,  green,  clamp(self.status - 1.0, 0.0, 1.0));
                let color = mix(color,  blue,   clamp(self.status - 2.0, 0.0, 1.0));
                let color = mix(color,  red,    clamp(self.status - 4.0, 0.0, 1.0));
                sdf.fill(color);
                return sdf.result;
            }
        }
    }

    // ── Inline mini progress bar ──

    HubInlineProgress = <View> {
        width: 118, height: 3
        margin: {top: 5, left: 22}
        show_bg: true
        draw_bg: {
            instance progress: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 1.5);
                let bg   = #d1d5db;
                let fill = #3b82f6;
                sdf.fill(mix(bg, fill, step(self.pos.x, self.progress)));
                return sdf.result;
            }
        }
    }

    // ── Model list item ──

    HubModelListItem = <View> {
        width: Fill, height: Fit
        padding: {left: 0, right: 0, top: 0, bottom: 0}
        margin: {left: 8, right: 8, bottom: 3}
        flow: Down
        cursor: Hand
        event_order: Down
        show_bg: true
        draw_bg: {
            instance selected: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 7.0);
                let base = #00000000;
                let selected = #e4ebf5;
                sdf.fill(mix(base, selected, self.selected));
                sdf.box(0.0, 9.0, 3.0, self.rect_size.y - 18.0, 1.5);
                sdf.fill(vec4(0.04, 0.63, 0.59, self.selected));
                return sdf.result;
            }
        }
        item_row = <View> {
            width: Fill, height: 44
            flow: Right
            align: {y: 0.5}
            padding: {left: 14, right: 10}
            model_status = <HubStatusDot> {}
            model_name = <Label> {
                width: Fill
                flow: Right
                align: {y: 0.5}
                margin: {right: 8}
                draw_text: {
                    fn get_color(self) -> vec4 {
                        return #111827;
                    }
                    text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                    wrap: Ellipsis
                }
            }
            downloaded_badge = <View> {
                width: 52, height: 26
                padding: {left: 6, right: 6, top: 0, bottom: 0}
                margin: {left: 0}
                align: {x: 0.5, y: 0.5}
                visible: false
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 5.0);
                        sdf.fill(#c9f7dc);
                        return sdf.result;
                    }
                }
                <Label> {
                    text: "Ready"
                    draw_text: {
                        fn get_color(self) -> vec4 { return #047857; }
                        text_style: <FONT_MEDIUM>{ font_size: 9.0 }
                    }
                }
            }
        }
        inline_progress = <HubInlineProgress> { visible: false }
    }

    // ── Subfolder group header (e.g. "Qwen3", "GLM") ──

    HubSubfolderGroupHeader = <View> {
        width: Fill, height: Fit
        padding: {left: 24, right: 14, top: 12, bottom: 5}
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                return #f7f9fc;
            }
        }
        subfolder_header_label = <Label> {
            draw_text: {
                fn get_color(self) -> vec4 {
                    return #6b7280;
                }
                text_style: <FONT_MEDIUM>{ font_size: 9.5 }
            }
        }
    }

    // ── Category group header ──

    HubCategoryGroupHeader = <View> {
        width: Fill, height: Fit
        padding: {left: 18, right: 14, top: 12, bottom: 6}
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                return #f7f9fc;
            }
        }
        category_header_label = <Label> {
            draw_text: {
                fn get_color(self) -> vec4 {
                    return #9ca3af;
                }
                text_style: <FONT_SEMIBOLD>{ font_size: 10.0 }
            }
        }
    }

    // ── Action button ──

    HubActionButton = <Button> {
        width: Fit, height: 30
        padding: {left: 13, right: 13}
        margin: {right: 8}
        animator: {
            hover = {
                default: off,
                off = { from: {all: Forward {duration: 0.1}} apply: { draw_bg: {hover: 0.0} } }
                on  = { from: {all: Forward {duration: 0.1}} apply: { draw_bg: {hover: 1.0} } }
            }
            pressed = {
                default: off,
                off = { from: {all: Forward {duration: 0.07}} apply: { draw_bg: {pressed: 0.0} } }
                on  = { from: {all: Forward {duration: 0.07}} apply: { draw_bg: {pressed: 1.0} } }
            }
        }
        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance danger: 0.0   // 0=primary blue, 1=danger red
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 6.0);
                let primary = mix(#3b82f6, #2563fa, self.hover);
                let danger  = mix(#b91c1c, #991b1b, self.hover); // no e after digit
                let color = mix(primary, danger, self.danger);
                sdf.fill(mix(color, color * 0.9, self.pressed));
                return sdf.result;
            }
        }
        draw_text: {
            fn get_color(self) -> vec4 { return #ffffff; }
            text_style: <FONT_MEDIUM>{ font_size: 11.2 }
        }
    }

    // ── Info row (label + value) ──

    HubInfoRow = <View> {
        width: Fill, height: Fit
        flow: Right
        padding: {top: 5, bottom: 5}
        align: {y: 0.0}
        info_label = <Label> {
            width: 100
            draw_text: {
                fn get_color(self) -> vec4 { return #9ca3af; }
                text_style: <FONT_MEDIUM>{ font_size: 11.0 }
            }
        }
        info_value = <Label> {
            width: Fill
            draw_text: {
                fn get_color(self) -> vec4 { return #374151; }
                text_style: <FONT_REGULAR>{ font_size: 11.0 }
                wrap: Word
            }
        }
    }

    // ── Progress bar ──

    HubProgressFill = <View> {
        width: Fill, height: Fill
        show_bg: true
        draw_bg: {
            instance progress: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x * self.progress, self.rect_size.y, 4.0);
                sdf.fill(#3b82f6);
                return sdf.result;
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Panel helper widgets
    // ─────────────────────────────────────────────────────────────────────────

    HubInputLabel = <Label> {
        width: Fill, height: Fit
        margin: {bottom: 3, top: 9}
        draw_text: {
            fn get_color(self) -> vec4 {
                return #6b7280;
            }
            text_style: <FONT_SEMIBOLD>{ font_size: 10.0 }
        }
    }

    HubPanelInput = <TextInput> {
        width: Fill, height: 36
        padding: {left: 12, right: 12, top: 9, bottom: 7}
        label_align: {x: 0.0, y: 0.5}
        margin: {bottom: 2}
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 7.0);
                sdf.fill(#ffffff);
                sdf.stroke(#e5e7eb, 1.0);
                return sdf.result;
            }
        }
        draw_text: {
            fn get_color(self) -> vec4 { return #374151; }
            color: #374151
            color_empty: #9ca3af
            text_style: { font_size: 12.0 }
        }
        draw_cursor: {
            uniform border_radius: 0.5
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius);
                sdf.fill(mix(#00000000, #1f2937, (1.0 - self.blink) * self.focus));
                return sdf.result;
            }
        }
    }

    HubPanelOutput = <View> {
        width: Fill, height: Fit
        padding: {left: 12, right: 12, top: 10, bottom: 10}
        margin: {top: 0, bottom: 12}
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 7.0);
                sdf.fill(#ffffff);
                sdf.stroke(#e5e7eb, 1.0);
                return sdf.result;
            }
        }
        output_label = <Label> {
            width: Fill
            draw_text: {
                fn get_color(self) -> vec4 {
                    return #1f2937;
                }
                text_style: { font_size: 12.0 }
                wrap: Word
            }
        }
    }

    HubPanelStatus = <Label> {
        width: Fill, height: Fit
        margin: {top: 6}
        draw_text: {
            fn get_color(self) -> vec4 {
                return #6b7280;
            }
            text_style: { font_size: 11.0 }
            wrap: Word
        }
    }

    // Shared model detail header included in each type panel
    HubPanelHeader = <View> {
        width: Fill, height: Fit
        flow: Down
        padding: {left: 30, right: 30, top: 22, bottom: 14}

        // Model name
        <View> {
            width: Fill, height: Fit
            flow: Right
            align: {y: 0.5}
            margin: {bottom: 7}
            panel_model_name = <Label> {
                draw_text: {
                    fn get_color(self) -> vec4 {
                        return #1f2937;
                    }
                    text_style: <FONT_SEMIBOLD>{ font_size: 20.5 }
                }
            }
        }

        // Description
        panel_model_desc = <Label> {
            width: Fill, height: Fit
            margin: {bottom: 12}
            draw_text: {
                fn get_color(self) -> vec4 {
                    return #6b7280;
                }
                text_style: { font_size: 12.2 }
                wrap: Word
            }
        }

        // Status + size + memory info row
        <View> {
            width: Fill, height: Fit
            flow: Right
            align: {y: 0.5}
            margin: {bottom: 10}
            panel_status_dot = <HubStatusDot> {}
            panel_download_btn = <HubActionButton> {
                text: "Download"
                visible: false
            }
            panel_status_text = <Label> {
                margin: {right: 8}
                draw_text: {
                    fn get_color(self) -> vec4 {
                        return #374151;
                    }
                    text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                }
            }
            panel_sep1 = <Label> {
                text: "·"
                margin: {right: 8}
                draw_text: {
                    fn get_color(self) -> vec4 {
                        return #9ca3af;
                    }
                    text_style: { font_size: 11.0 }
                }
            }
            panel_size_text = <Label> {
                margin: {right: 8}
                draw_text: {
                    fn get_color(self) -> vec4 {
                        return #6b7280;
                    }
                    text_style: { font_size: 11.0 }
                }
            }
            panel_sep2 = <Label> {
                text: "·"
                margin: {right: 8}
                draw_text: {
                    fn get_color(self) -> vec4 {
                        return #9ca3af;
                    }
                    text_style: { font_size: 11.0 }
                }
            }
            panel_mem_text = <Label> {
                draw_text: {
                    fn get_color(self) -> vec4 {
                        return #6b7280;
                    }
                    text_style: { font_size: 11.0 }
                }
            }
        }

        // Action buttons
        panel_action_row = <View> {
            width: Fill, height: Fit
            flow: Right
            margin: {bottom: 6}
            visible: false
            panel_cancel_btn = <HubActionButton> {
                text: "Cancel"
                visible: false
                draw_bg: { danger: 1.0 }
            }
        }

        // Runtime controls: Load / Unload (shown when model is downloaded)
        panel_runtime_row = <View> {
            width: Fill, height: Fit
            flow: Right
            align: {y: 0.5}
            margin: {bottom: 6}
            visible: false
            panel_load_btn = <HubActionButton> {
                text: "Load"
                visible: false
            }
            panel_remove_btn = <HubActionButton> {
                text: "Remove"
                visible: false
                draw_bg: { danger: 1.0 }
            }
            panel_unload_btn = <HubActionButton> {
                text: "Unload"
                visible: false
            }
            panel_loading_label = <Label> {
                draw_text: {
                    fn get_color(self) -> vec4 {
                        return #6366f1;
                    }
                    text_style: <FONT_MEDIUM>{ font_size: 11.5 }
                }
                text: ""
            }
            panel_chat_btn = <HubActionButton> {
                text: "Open in Chat"
            }
        }

        // Progress bar (visible while downloading)
        panel_progress_section = <View> {
            visible: false
            width: Fill, height: Fit
            flow: Down
            margin: {top: 2, bottom: 4}
            panel_progress_bg = <View> {
                width: Fill, height: 6
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 3.0);
                        sdf.fill(#d9dee7);
                        return sdf.result;
                    }
                }
                panel_progress_fill = <HubProgressFill> {}
            }
            panel_progress_text = <Label> {
                width: Fill, height: Fit
                margin: {top: 6}
                draw_text: {
                    fn get_color(self) -> vec4 {
                        return #6b7280;
                    }
                    text_style: { font_size: 11.0 }
                }
            }
        }

        // Manual install / error message
        panel_status_msg = <Label> {
            width: Fill, height: Fit
            margin: {top: 1}
            draw_text: {
                fn get_color(self) -> vec4 {
                    return #64748b;
                }
                text_style: { font_size: 11.5 }
                wrap: Word
            }
        }
    }

    // ── TTS voice selector item ──
    HubTtsVoiceItem = <View> {
        width: Fill, height: 40
        padding: {left: 12, right: 12, top: 8, bottom: 8}
        margin: {left: 4, right: 4}
        cursor: Hand
        event_order: Down
        flow: Right
        align: {y: 0.5}
        show_bg: true
        draw_bg: {
            instance selected: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, 6.0);
                let normal  = #ffffff;
                let sel_col = #eaecf0;
                sdf.fill(mix(normal, sel_col, self.selected));
                return sdf.result;
            }
        }
        tts_voice_name = <Label> {
            width: Fill
            draw_text: {
                fn get_color(self) -> vec4 { return #1f2937; }
                text_style: <FONT_REGULAR>{ font_size: 11.5 }
                wrap: Ellipsis
            }
        }
        tts_lang_badge = <View> {
            width: Fit, height: Fit
            padding: {left: 6, right: 6, top: 2, bottom: 2}
            margin: {left: 8}
            draw_bg: {
                instance is_chinese: 0.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 3.0);
                    let en_bg = #dbeafe;
                    let zh_bg = #fef3c7;
                    sdf.fill(mix(en_bg, zh_bg, self.is_chinese));
                    return sdf.result;
                }
            }
            tts_lang_label = <Label> {
                draw_text: {
                    instance is_chinese: 0.0
                    fn get_color(self) -> vec4 {
                        let en_col = #1a40af;
                        let zh_col = #92400f;
                        return mix(en_col, zh_col, self.is_chinese);
                    }
                    text_style: <FONT_MEDIUM>{ font_size: 9.0 }
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Main ModelHubApp widget
    // ─────────────────────────────────────────────────────────────────────────

    // ── Voice Studio footer item in model list ──
    HubVoiceStudioItem = <View> {
        width: Fill, height: Fit
        padding: {left: 14, right: 8, top: 12, bottom: 12}
        margin: {left: 4, right: 4}
        cursor: Hand
        event_order: Down
        show_bg: true
        draw_bg: {
            instance selected: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, 6.0);
                let base = #ffffff;
                let gray = #eaecf0;
                sdf.fill(mix(base, gray, self.selected));
                return sdf.result;
            }
        }
        flow: Right
        align: {y: 0.5}
        <Label> {
            width: Fit
            margin: {right: 8}
            text: "🎙"
            draw_text: {
                text_style: { font_size: 14.0 }
            }
        }
        voice_studio_label = <Label> {
            width: Fill
            text: "Voice Studio"
            draw_text: {
                fn get_color(self) -> vec4 {
                    return #1f2937;
                }
                text_style: <FONT_MEDIUM>{ font_size: 11.3 }
            }
        }
    }

    // ── Voice list item inside Voice Studio panel ──
    HubVoiceListItem = <View> {
        width: Fill, height: 40
        padding: {left: 12, right: 12, top: 8, bottom: 8}
        margin: {left: 4, right: 4}
        cursor: Hand
        event_order: Down
        flow: Right
        align: {y: 0.5}
        show_bg: true
        draw_bg: {
            instance selected: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(1.0, 1.0, self.rect_size.x - 2.0, self.rect_size.y - 2.0, 6.0);
                let normal   = #ffffff;
                let sel_col  = #eaecf0;
                sdf.fill(mix(normal, sel_col, self.selected));
                return sdf.result;
            }
        }
        voice_status_dot = <View> {
            width: 8, height: 8
            margin: {right: 8}
            draw_bg: {
                instance ready: 0.0
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    sdf.circle(4.0, 4.0, 4.0);
                    sdf.fill(mix(#d1d5db, #22c55a, self.ready));
                    return sdf.result;
                }
            }
        }
        voice_item_name = <Label> {
            width: Fill
            draw_text: {
                fn get_color(self) -> vec4 { return #1f2937; }
                text_style: <FONT_REGULAR>{ font_size: 11.5 }
                wrap: Ellipsis
            }
        }
    }

    pub ModelHubApp = {{ModelHubApp}} {
        width: Fill, height: Fill
        flow: Right
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                return #f8fafc;
            }
        }

        // ── Left panel ──────────────────────────────────────────────────────
        hub_left_panel = <View> {
            width: 276, height: Fill
            flow: Down
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 { return #f7f9fc; }
            }

            // Header
            <View> {
                width: Fill, height: 54
                padding: {left: 18, right: 18}
                align: {y: 0.5}
                hub_title_label = <Label> {
                    text: "Model Hub"
                    draw_text: {
                        fn get_color(self) -> vec4 {
                            return #1f2937;
                        }
                        text_style: <FONT_SEMIBOLD>{ font_size: 15.5 }
                    }
                }
            }

            // Divider
            hub_header_divider = <View> {
                width: Fill, height: 1
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 { return #f1f5f9; }
                }
            }

            // Search
            <View> {
                width: Fill, height: Fit
                padding: {left: 14, right: 14, top: 10, bottom: 8}
                search_input = <TextInput> {
                    width: Fill, height: 34
                    padding: {left: 12, right: 12, top: 8, bottom: 7}
                    label_align: {x: 0.0, y: 0.5}
                    empty_text: "Search models..."
                    draw_bg: {
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 7.0);
                            sdf.fill(#ffffff);
                            sdf.stroke(#e5e7eb, 1.0);
                            return sdf.result;
                        }
                    }
                    draw_text: {
                        fn get_color(self) -> vec4 { return #374151; }
                        color: #374151
                        color_empty: #9ca3af
                        text_style: { font_size: 12.0 }
                    }
                    draw_cursor: {
                        uniform border_radius: 0.5
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius);
                            sdf.fill(mix(#00000000, #1f2937, (1.0 - self.blink) * self.focus));
                            return sdf.result;
                        }
                    }
                }
            }

            // Model list
            hub_model_list = <PortalList> {
                width: Fill, height: Fill
                flow: Down
                HubModelItem        = <HubModelListItem> {}
                HubCategoryHeader   = <HubCategoryGroupHeader> {}
                HubSubfolderHeader  = <HubSubfolderGroupHeader> {}
                HubVoiceStudioItem  = <HubVoiceStudioItem> {}
            }

        }

        // Vertical divider – 8 px wide for easy dragging, visually 1px center line
        hub_main_divider = <View> {
            width: 8, height: Fill
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    // 1px opaque line in center, transparent on either side
                    let dist = abs(self.pos.x - 0.5) * self.rect_size.x;
                    let col  = #e2e8f0;
                    return vec4(col.r, col.g, col.b, 1.0 - step(0.5, dist));
                }
            }
        }

        // ── Right panel: type-aware Overlay ──────────────────────────────────
        hub_right_panel = <View> {
            width: Fill, height: Fill
            flow: Overlay
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 { return #f7f9fc; }
            }

            // Empty state (default)
            hub_empty_state = <View> {
                width: Fill, height: Fill
                align: {x: 0.5, y: 0.4}
                visible: true
                hub_empty_label = <Label> {
                    text: "Select a model from the list"
                    draw_text: {
                        fn get_color(self) -> vec4 {
                            return #9ca3af;
                        }
                        text_style: { font_size: 14.0 }
                    }
                }
            }

            // ── LLM panel ────────────────────────────────────────────────────
            hub_llm_panel = <ScrollYView> {
                width: Fill, height: Fill
                visible: false
                flow: Down

                hub_panel_header = <HubPanelHeader> {}

                hub_llm_divider = <View> {
                    width: Fill, height: 1
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 { return #f1f5f9; }
                    }
                }

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 14, bottom: 28}

                    <HubInputLabel> { text: "SYSTEM PROMPT" }
                    llm_system = <HubPanelInput> {
                        height: 58
                        empty_text: "You are a helpful assistant..."
                    }

                    <HubInputLabel> { text: "USER MESSAGE" }
                    llm_user = <HubPanelInput> {
                        height: 48
                        empty_text: "Type your message here..."
                    }

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        margin: {top: 8, bottom: 12}
                        llm_generate_btn = <HubActionButton> { text: "Generate" }
                    }

                    <HubInputLabel> { text: "RESPONSE" }
                    llm_response = <HubPanelOutput> {}
                    llm_status = <HubPanelStatus> {}
                }
            }

            // ── VLM panel ────────────────────────────────────────────────────
            hub_vlm_panel = <ScrollYView> {
                width: Fill, height: Fill
                visible: false
                flow: Down

                hub_panel_header = <HubPanelHeader> {}

                hub_vlm_divider = <View> {
                    width: Fill, height: 1
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 { return #f1f5f9; }
                    }
                }

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 14, bottom: 28}

                    <HubInputLabel> { text: "IMAGE FILE" }

                    // Drag-and-drop zone for image files from Finder
                    vlm_drop_zone = <View> {
                        width: Fill, height: 56
                        margin: {bottom: 6}
                        show_bg: true
                        draw_bg: {
                            instance drag_over: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let border = mix(#d1d5db, #6366f1, self.drag_over);
                                let fill = mix(#f9fafb, #eef2ff, self.drag_over);
                                sdf.box(2.0, 2.0, self.rect_size.x - 4.0, self.rect_size.y - 4.0, 8.0);
                                sdf.fill(border);
                                sdf.box(3.5, 3.5, self.rect_size.x - 7.0, self.rect_size.y - 7.0, 6.5);
                                sdf.fill(fill);
                                return sdf.result;
                            }
                        }
                        align: {x: 0.5, y: 0.5}

                        vlm_drop_label = <Label> {
                            text: "Drop image here"
                            draw_text: {
                                color: #9ca3af
                                text_style: { font_size: 12.0 }
                            }
                        }
                    }

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        align: {y: 0.5}
                        margin: {bottom: 4}
                        vlm_image_path = <HubPanelInput> {
                            width: Fill, height: 36
                            margin: {right: 6, bottom: 0}
                        }
                        vlm_browse_btn = <HubActionButton> { text: "Browse..." margin: {right: 0} }
                    }

                    <HubInputLabel> { text: "USER MESSAGE" }
                    vlm_user = <HubPanelInput> {
                        height: 48
                        empty_text: "Describe this image..."
                    }

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        margin: {top: 8, bottom: 12}
                        vlm_generate_btn = <HubActionButton> { text: "Generate" }
                    }

                    <HubInputLabel> { text: "RESPONSE" }
                    vlm_response = <HubPanelOutput> {}
                    vlm_status = <HubPanelStatus> {}
                }
            }

            // ── ASR panel ────────────────────────────────────────────────────
            hub_asr_panel = <ScrollYView> {
                width: Fill, height: Fill
                visible: false
                flow: Down

                hub_panel_header = <HubPanelHeader> {}

                hub_asr_divider = <View> {
                    width: Fill, height: 1
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 { return #f1f5f9; }
                    }
                }

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 14, bottom: 28}

                    <HubInputLabel> { text: "AUDIO FILE" }
                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        align: {y: 0.5}
                        margin: {bottom: 4}
                        asr_audio_path = <HubPanelInput> {
                            width: Fill, height: 36
                            margin: {right: 6, bottom: 0}
                        }
                        asr_browse_btn = <HubActionButton> { text: "Browse..." margin: {right: 0} }
                    }

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        margin: {top: 8, bottom: 12}
                        asr_transcribe_btn = <HubActionButton> { text: "Transcribe" }
                    }

                    <HubInputLabel> { text: "TRANSCRIPT" }
                    asr_transcript = <HubPanelOutput> {}
                    asr_status = <HubPanelStatus> {}
                }
            }

            // ── TTS panel ────────────────────────────────────────────────────
            hub_tts_panel = <ScrollYView> {
                width: Fill, height: Fill
                visible: false
                flow: Down

                hub_panel_header = <HubPanelHeader> {}

                hub_tts_divider = <View> {
                    width: Fill, height: 1
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 { return #f1f5f9; }
                    }
                }

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 14, bottom: 28}

                    <HubInputLabel> { text: "VOICE" }
                    tts_voice_list = <PortalList> {
                        width: Fill, height: 360
                        flow: Down
                        HubTtsVoiceItem = <HubTtsVoiceItem> {}
                    }

                    <HubInputLabel> { text: "TEXT TO SPEAK" }
                    tts_text_input = <HubPanelInput> {
                        height: 80
                        empty_text: "Enter text to synthesize..."
                    }

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        margin: {top: 8, bottom: 12}
                        tts_generate_btn = <HubActionButton> { text: "Generate & Play" }
                    }

                    tts_status = <HubPanelStatus> {}

                    // Save + Finder row (hidden until audio is generated)
                    tts_result_row = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        align: {y: 0.5}
                        margin: {top: 8}
                        visible: false
                        spacing: 8

                        tts_save_btn = <HubActionButton> { text: "Save to Downloads" }
                        tts_finder_btn = <HubActionButton> {
                            text: "Show in Finder"
                            visible: false
                        }
                    }
                }
            }

            // ── Image panel ──────────────────────────────────────────────────
            hub_image_panel = <ScrollYView> {
                width: Fill, height: Fill
                visible: false
                flow: Down

                hub_panel_header = <HubPanelHeader> {}

                hub_image_divider = <View> {
                    width: Fill, height: 1
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 { return #f1f5f9; }
                    }
                }

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 14, bottom: 28}

                    <HubInputLabel> { text: "PROMPT" }
                    img_prompt = <HubPanelInput> {
                        height: 58
                        empty_text: "A beautiful landscape..."
                    }

                    <HubInputLabel> { text: "NEGATIVE PROMPT (OPTIONAL)" }
                    img_neg_prompt = <HubPanelInput> {
                        height: 48
                        empty_text: "blurry, low quality..."
                    }

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        margin: {top: 8, bottom: 12}
                        img_generate_btn = <HubActionButton> { text: "Generate Image" }
                    }

                    img_status = <HubPanelStatus> {}

                    // Preview image (hidden until generated)
                    img_preview = <Image> {
                        width: Fill, height: 400
                        visible: false
                        margin: {top: 16, bottom: 8}
                        fit: Biggest
                    }

                    // File path + Finder button row (hidden until generated)
                    img_result_row = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        align: {y: 0.5}
                        margin: {bottom: 8}
                        visible: false
                        spacing: 8

                        img_output_path = <Label> {
                            width: Fill, height: Fit
                            draw_text: {
                                fn get_color(self) -> vec4 { return #374151; }
                                text_style: { font_size: 11.0 }
                                wrap: Word
                            }
                        }

                        img_open_finder_btn = <HubActionButton> {
                            text: "Show in Finder"
                            width: Fit
                        }
                    }
                }
            }

            // ── Image Edit Panel ────────────────────────────────────────────────
            hub_image_edit_panel = <ScrollYView> {
                width: Fill, height: Fill
                visible: false
                flow: Down

                hub_panel_header = <HubPanelHeader> {}

                <View> {
                    width: Fill, height: 1
                    show_bg: true
                    draw_bg: { fn pixel(self) -> vec4 { return #f1f5f9; } }
                }

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 14, bottom: 28}

                    <HubInputLabel> { text: "REFERENCE IMAGE" }

                    // Drag-and-drop zone for reference image
                    img_edit_drop_zone = <View> {
                        width: Fill, height: 56
                        margin: {bottom: 6}
                        show_bg: true
                        draw_bg: {
                            instance drag_over: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let border = mix(#d1d5db, #ec4899, self.drag_over);
                                let fill = mix(#f9fafb, #fdf2f8, self.drag_over);
                                sdf.box(2.0, 2.0, self.rect_size.x - 4.0, self.rect_size.y - 4.0, 8.0);
                                sdf.fill(border);
                                sdf.box(3.5, 3.5, self.rect_size.x - 7.0, self.rect_size.y - 7.0, 6.5);
                                sdf.fill(fill);
                                return sdf.result;
                            }
                        }
                        align: {x: 0.5, y: 0.5}

                        img_edit_drop_label = <Label> {
                            text: "Drop reference image here"
                            draw_text: {
                                color: #9ca3af
                                text_style: { font_size: 12.0 }
                            }
                        }
                    }

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        align: {y: 0.5}
                        margin: {bottom: 12}
                        img_edit_image_path = <HubPanelInput> {
                            width: Fill, height: 36
                            margin: {right: 6, bottom: 0}
                        }
                        img_edit_browse_btn = <HubActionButton> { text: "Browse..." margin: {right: 0} }
                    }

                    <HubInputLabel> { text: "EDIT INSTRUCTION" }
                    img_edit_prompt = <HubPanelInput> {
                        height: 58
                        empty_text: "Change the background to a sunny beach..."
                    }

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        margin: {top: 8, bottom: 12}
                        img_edit_btn = <HubActionButton> { text: "Edit Image" }
                    }

                    img_edit_status = <HubPanelStatus> {}

                    // Preview of the edited result (hidden until generated)
                    img_edit_preview = <Image> {
                        width: Fill, height: 400
                        visible: false
                        margin: {top: 16, bottom: 8}
                        fit: Biggest
                    }

                    // File path + Finder button row (hidden until generated)
                    img_edit_result_row = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        align: {y: 0.5}
                        margin: {bottom: 8}
                        visible: false
                        spacing: 8

                        img_edit_output_path = <Label> {
                            width: Fill, height: Fit
                            draw_text: {
                                fn get_color(self) -> vec4 { return #374151; }
                                text_style: { font_size: 11.0 }
                                wrap: Word
                            }
                        }

                        img_edit_open_finder_btn = <HubActionButton> {
                            text: "Show in Finder"
                            width: Fit
                        }
                    }
                }
            }

            // ── Video Generation Panel ─────────────────────────────────────────
            hub_video_panel = <ScrollYView> {
                width: Fill, height: Fill
                visible: false
                flow: Down

                hub_panel_header = <HubPanelHeader> {}

                hub_video_divider = <View> {
                    width: Fill, height: 1
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 { return #f1f5f9; }
                    }
                }

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 14, bottom: 28}

                    <HubInputLabel> { text: "PROMPT" }
                    vid_prompt = <HubPanelInput> {
                        height: 58
                        empty_text: "A cat walking on the beach at sunset..."
                    }

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        margin: {top: 8, bottom: 12}
                        vid_generate_btn = <HubActionButton> { text: "Generate Video" }
                    }

                    vid_status = <HubPanelStatus> {}

                    // File path + action buttons (hidden until generated)
                    vid_result_row = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        align: {y: 0.5}
                        margin: {top: 8, bottom: 8}
                        visible: false
                        spacing: 8

                        vid_output_path = <Label> {
                            width: Fill, height: Fit
                            draw_text: {
                                fn get_color(self) -> vec4 { return #374151; }
                                text_style: { font_size: 11.0 }
                                wrap: Word
                            }
                        }

                        vid_play_btn = <HubActionButton> {
                            text: "Play"
                            width: Fit
                        }

                        vid_open_finder_btn = <HubActionButton> {
                            text: "Show in Finder"
                            width: Fit
                        }
                    }
                }
            }

            // ── Voice Studio Panel ──────────────────────────────────────────────
            hub_voice_panel = <View> {
            width: Fill, height: Fill
            visible: false
            flow: Right

            // Left sub-panel: voice list + actions
            <View> {
                width: 240, height: Fill
                flow: Down
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 { return #ffffff; }
                }

                // Header + New button
                <View> {
                    width: Fill, height: 48
                    padding: {left: 16, right: 8}
                    align: {y: 0.5}
                    flow: Right
                    voice_list_title = <Label> {
                        width: Fill
                        text: "Voices"
                        draw_text: {
                            fn get_color(self) -> vec4 {
                                return #1f2937;
                            }
                            text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                        }
                    }
                    voice_new_btn = <HubActionButton> {
                        text: "+ New"
                        padding: {left: 8, right: 8}
                        height: 28
                    }
                }

                voice_left_divider = <View> {
                    width: Fill, height: 1
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 { return #f1f5f9; }
                    }
                }

                // Voice list
                voice_list = <PortalList> {
                    width: Fill, height: Fill
                    flow: Down
                    HubVoiceListItem = <HubVoiceListItem> {}
                }
            }

            // Vertical divider
            voice_panel_divider = <View> {
                width: 1, height: Fill
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 { return #f1f5f9; }
                }
            }

            // Right sub-panel: training form + synthesis
            <ScrollYView> {
                width: Fill, height: Fill
                flow: Down
                padding: {left: 28, right: 28, top: 20, bottom: 32}

                // Training section header
                voice_training_title = <Label> {
                    width: Fill
                    margin: {bottom: 12}
                    text: "VOICE TRAINING"
                    draw_text: {
                        fn get_color(self) -> vec4 {
                            return #6b7280;
                        }
                        text_style: <FONT_SEMIBOLD>{ font_size: 10.5 }
                    }
                }

                <HubInputLabel> { text: "VOICE NAME" }
                voice_name_input = <HubPanelInput> {
                    height: 36
                    empty_text: "My Voice"
                }

                <HubInputLabel> { text: "AUDIO FILE (.wav)" }
                <View> {
                    width: Fill, height: Fit
                    flow: Right
                    align: {y: 0.5}
                    margin: {bottom: 4}
                    voice_audio_path_input = <HubPanelInput> {
                        width: Fill, height: 36
                        margin: {right: 6, bottom: 0}
                    }
                    voice_audio_browse_btn = <HubActionButton> { text: "Browse..." margin: {right: 0} }
                }

                <HubInputLabel> { text: "TRANSCRIPT (OPTIONAL)" }
                voice_transcript_input = <HubPanelInput> {
                    height: 48
                    empty_text: "Text spoken in the audio file..."
                }

                // Quality selector
                <HubInputLabel> { text: "QUALITY" }
                <View> {
                    width: Fill, height: Fit
                    flow: Right
                    margin: {bottom: 12}
                    voice_quality_fast     = <HubActionButton> { text: "Fast",     margin: {right: 6} }
                    voice_quality_standard = <HubActionButton> { text: "Standard", margin: {right: 6} }
                    voice_quality_high     = <HubActionButton> { text: "High" }
                }

                <View> {
                    width: Fill, height: Fit
                    flow: Right
                    margin: {bottom: 8}
                    voice_train_btn        = <HubActionButton> { text: "Train Voice", margin: {right: 8} }
                    voice_cancel_train_btn = <HubActionButton> {
                        text: "Cancel"
                        visible: false
                        draw_bg: { danger: 1.0 }
                    }
                }

                voice_train_status = <HubPanelStatus> {}

                // Divider
                voice_synth_divider = <View> {
                    width: Fill, height: 1
                    margin: {top: 20, bottom: 20}
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 { return #f1f5f9; }
                    }
                }

                // Synthesis section header
                voice_synthesis_title = <Label> {
                    width: Fill
                    margin: {bottom: 12}
                    text: "VOICE SYNTHESIS"
                    draw_text: {
                        fn get_color(self) -> vec4 {
                            return #6b7280;
                        }
                        text_style: <FONT_SEMIBOLD>{ font_size: 10.5 }
                    }
                }

                <HubInputLabel> { text: "TEXT TO SYNTHESIZE" }
                voice_synth_text = <HubPanelInput> {
                    height: 58
                    empty_text: "Enter text to synthesize..."
                }

                <HubInputLabel> { text: "SPEED (0.5 – 2.0)" }
                voice_speed_input = <HubPanelInput> {
                    height: 36
                    empty_text: "1.0"
                }

                <View> {
                    width: Fill, height: Fit
                    flow: Right
                    margin: {top: 10, bottom: 8}
                    voice_generate_btn = <HubActionButton> { text: "Synthesize", margin: {right: 8} }
                    voice_play_btn     = <HubActionButton> { text: "▶  Play" }
                }

                voice_synth_status = <HubPanelStatus> {}
            }
        }
    }
}
}
