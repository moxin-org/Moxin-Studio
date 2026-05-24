use makepad_widgets::*;
use super::ModelHubApp;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use moly_widgets::theme::*;

    ICON_SEARCH = dep("crate://self/resources/icons/search.svg")

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
        margin: {left: 8, right: 8, bottom: 4}
        flow: Down
        cursor: Hand
        event_order: Down
        show_bg: true
        draw_bg: {
            instance selected: 0.0
            instance hover: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                // Demo: rounded-xl (12px); hover:bg-surface-container-high #dfeaef;
                // selected: bg-primary-container/10 + border border-primary
                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                let hover_col = #dfeaef;
                let selected_bg = #d8edf0;
                let base = #00000000;
                sdf.fill_keep(mix(mix(base, hover_col, self.hover), selected_bg, self.selected));
                // Full-opacity primary border when selected (faint otherwise)
                sdf.stroke(vec4(0.0, 0.412, 0.447, self.selected), 1.0);
                return sdf.result;
            }
        }
        item_row = <View> {
            width: Fill, height: 46
            flow: Right
            align: {y: 0.5}
            padding: {left: 16, right: 12}
            model_status = <HubStatusDot> { width: 0, height: 0, margin: {right: 0} }
            model_name = <Label> {
                width: Fill
                flow: Right
                align: {y: 0.5}
                margin: {right: 8}
                draw_text: {
                    instance selected: 0.0
                    fn get_color(self) -> vec4 {
                        // Demo: text-on-surface-variant #3d494a → text-primary #006972 + font-bold when selected
                        return mix(#3d494a, #006972, self.selected);
                    }
                    text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                    wrap: Ellipsis
                }
            }
            downloaded_badge = <View> {
                width: Fit, height: 20
                padding: {left: 8, right: 8, top: 0, bottom: 0}
                margin: {left: 0}
                align: {x: 0.5, y: 0.5}
                visible: false
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        // Demo: bg-green-100 rounded-full
                        let r = self.rect_size.y * 0.25;
                        sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, r);
                        sdf.fill(#dcfce7);
                        return sdf.result;
                    }
                }
                <Label> {
                    text: "READY"
                    draw_text: {
                        // Demo: text-green-700 #15803d, font-bold uppercase
                        fn get_color(self) -> vec4 { return #15803d; }
                        text_style: <FONT_SEMIBOLD>{ font_size: 9.0 }
                    }
                }
            }
        }
        inline_progress = <HubInlineProgress> { visible: false }
    }

    // ── Subfolder group header (e.g. "Qwen3", "GLM") ──

    HubSubfolderGroupHeader = <View> {
        width: Fill, height: Fit
        padding: {left: 24, right: 14, top: 18, bottom: 8}
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                return #ffffff;
            }
        }
        // Demo: text-[10px] font-bold text-on-surface-variant/60 uppercase tracking-widest
        subfolder_header_label = <Label> {
            draw_text: {
                fn get_color(self) -> vec4 {
                    return #3d494a99;
                }
                text_style: <FONT_SEMIBOLD>{ font_size: 10.0 }
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
                return #ffffff;
            }
        }
        category_header_label = <Label> {
            draw_text: {
                fn get_color(self) -> vec4 {
                    return #3d494a99;
                }
                text_style: <FONT_SEMIBOLD>{ font_size: 10.0 }
            }
        }
    }

    // ── Action button ──

    // Demo: rounded-full pill; primary=teal (#22a6b3, on-primary-container #00353a),
    // purple=secondary #5240dd white text, danger=outlined error #ba1a1a transparent bg
    HubActionButton = <Button> {
        width: Fit, height: 44
        padding: {left: 24, right: 24}
        margin: {right: 12}
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
            instance danger: 0.0
            instance purple: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                // Makepad sdf.box doubles r internally; h*0.25 ⇒ true pill
                let r = self.rect_size.y * 0.25;
                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, r);
                // Demo: primary-container teal
                let teal = mix(#22a6b3, #2bb6c4, self.hover);
                // Demo: secondary #5240dd
                let purple = mix(#5240dd, #6253e8, self.hover);
                let primary = mix(teal, purple, self.purple);
                // Danger (Remove): transparent fill with 5% error tint on hover
                let danger_fill = mix(#00000000, #ba1a1a0d, self.hover);
                let color = mix(primary, danger_fill, self.danger);
                sdf.fill_keep(mix(color, color * 0.95, self.pressed));
                // Border for danger only
                sdf.stroke(vec4(0.729, 0.102, 0.102, self.danger), 1.2);
                return sdf.result;
            }
        }
        draw_text: {
            color: #ffffff
            color_hover: #ffffff
            color_down: #ffffff
            color_focus: #ffffff
            color_disabled: #ffffff
            text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
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
        margin: {bottom: 6, top: 10}
        draw_text: {
            fn get_color(self) -> vec4 {
                return #15242b;
            }
            text_style: <FONT_SEMIBOLD>{ font_size: 10.5 }
        }
    }

    HubPanelInput = <TextInput> {
        width: Fill, height: 36
        padding: {left: 24, right: 24, top: 22, bottom: 18}
        label_align: {x: 0.0, y: 0.5}
        margin: {bottom: 14}
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 12.0);
                sdf.fill(#f8fdff);
                sdf.stroke(#b8cbd2, 1.2);
                return sdf.result;
            }
        }
        draw_text: {
            fn get_color(self) -> vec4 { return #374151; }
            color: #374151
            color_empty: #9ca3af
            text_style: { font_size: 13.0 }
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

    // ── Generate button with sparkle icon + text (demo: bg-primary-container + on-primary-container, rounded-xl) ──
    HubGenerateButton = <View> {
        width: Fit, height: 44
        padding: {left: 20, right: 24, top: 0, bottom: 0}
        flow: Right
        align: {x: 0.5, y: 0.5}
        spacing: 8
        cursor: Hand
        show_bg: true

        animator: {
            hover = {
                default: off
                off = { from: {all: Forward {duration: 0.1}} apply: { draw_bg: {hover: 0.0} } }
                on  = { from: {all: Forward {duration: 0.1}} apply: { draw_bg: {hover: 1.0} } }
            }
        }

        draw_bg: {
            instance hover: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                sdf.fill(mix(#22a6b3, #2bb6c4, self.hover));
                return sdf.result;
            }
        }

        generate_icon = <View> {
            width: 18, height: 18
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                    let main_cx = 8.0;
                    let main_cy = 10.0;
                    // Main 4-point sparkle drawn as two overlapping diamonds
                    // Vertical narrow diamond
                    sdf.move_to(main_cx, main_cy - 7.0);
                    sdf.line_to(main_cx + 1.6, main_cy);
                    sdf.line_to(main_cx, main_cy + 7.0);
                    sdf.line_to(main_cx - 1.6, main_cy);
                    sdf.close_path();
                    sdf.fill(#ffffff);
                    // Horizontal narrow diamond
                    sdf.move_to(main_cx - 7.0, main_cy);
                    sdf.line_to(main_cx, main_cy - 1.6);
                    sdf.line_to(main_cx + 7.0, main_cy);
                    sdf.line_to(main_cx, main_cy + 1.6);
                    sdf.close_path();
                    sdf.fill(#ffffff);
                    // Accent sparkle (upper-right, smaller)
                    let ax = 14.0;
                    let ay = 4.5;
                    sdf.move_to(ax, ay - 3.0);
                    sdf.line_to(ax + 0.8, ay);
                    sdf.line_to(ax, ay + 3.0);
                    sdf.line_to(ax - 0.8, ay);
                    sdf.close_path();
                    sdf.fill(#ffffff);
                    sdf.move_to(ax - 3.0, ay);
                    sdf.line_to(ax, ay - 0.8);
                    sdf.line_to(ax + 3.0, ay);
                    sdf.line_to(ax, ay + 0.8);
                    sdf.close_path();
                    sdf.fill(#ffffff);
                    return sdf.result;
                }
            }
        }
        generate_label = <Label> {
            text: "Generate"
            draw_text: {
                color: #ffffff
                text_style: <FONT_BOLD>{ font_size: 13.5 }
            }
        }
    }

    // ── Card header icon (used inside SYSTEM PROMPT / USER MESSAGE cards) ──
    // kind: 0 = shield (admin_panel_settings), 1 = person, 2 = sparkle
    HubCardHeaderIcon = <View> {
        width: 22, height: 22
        show_bg: true
        draw_bg: {
            instance kind: 0.0
            instance fg_col: vec4(0.0, 0.412, 0.447, 1.0)
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let cx = 11.0;
                let cy = 11.0;
                // Shield: top arc + side bevels meeting at bottom point
                if self.kind < 0.5 {
                    sdf.move_to(cx - 6.0, cy - 5.0);
                    sdf.line_to(cx, cy - 8.0);
                    sdf.line_to(cx + 6.0, cy - 5.0);
                    sdf.line_to(cx + 6.0, cy + 1.0);
                    sdf.line_to(cx, cy + 8.5);
                    sdf.line_to(cx - 6.0, cy + 1.0);
                    sdf.close_path();
                    sdf.stroke(self.fg_col, 1.6);
                    // Inner tick
                    sdf.move_to(cx - 2.5, cy);
                    sdf.line_to(cx - 0.5, cy + 2.0);
                    sdf.line_to(cx + 3.0, cy - 2.0);
                    sdf.stroke(self.fg_col, 1.6);
                }
                // Person: head circle + shoulders arc
                if self.kind > 0.5 && self.kind < 1.5 {
                    sdf.circle(cx, cy - 3.0, 3.2);
                    sdf.stroke(self.fg_col, 1.6);
                    // Shoulders (rounded bottom)
                    sdf.box(cx - 6.0, cy + 1.5, 12.0, 8.0, 4.0);
                    sdf.stroke(self.fg_col, 1.6);
                }
                // Sparkle (auto_awesome): 4-pointed star
                if self.kind > 1.5 {
                    // Diamond/star: vertical+horizontal+two diagonal lines from center
                    sdf.move_to(cx, cy - 7.0);
                    sdf.line_to(cx + 2.0, cy - 2.0);
                    sdf.line_to(cx + 7.0, cy);
                    sdf.line_to(cx + 2.0, cy + 2.0);
                    sdf.line_to(cx, cy + 7.0);
                    sdf.line_to(cx - 2.0, cy + 2.0);
                    sdf.line_to(cx - 7.0, cy);
                    sdf.line_to(cx - 2.0, cy - 2.0);
                    sdf.close_path();
                    sdf.fill(self.fg_col);
                }
                return sdf.result;
            }
        }
    }

    // ── Labeled card (SYSTEM PROMPT / USER MESSAGE wrapper) ──
    // Demo: bg-surface border outline-variant rounded-2xl, header (icon + uppercase label)
    // + divider + body. `overflow-hidden custom-shadow` approximated with single rounded outline.
    HubLabeledCard = <View> {
        width: Fill, height: Fit
        flow: Down
        margin: {bottom: 14}
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 8.0);
                sdf.fill_keep(#ffffff);
                sdf.stroke(#bcc9ca, 1.0);
                return sdf.result;
            }
        }

        card_header = <View> {
            width: Fill, height: Fit
            flow: Right
            align: {y: 0.5}
            padding: {left: 22, right: 22, top: 14, bottom: 14}
            spacing: 10
            header_icon = <HubCardHeaderIcon> {}
            header_label = <Label> {
                draw_text: {
                    color: #131d21
                    text_style: <FONT_BOLD>{ font_size: 11.0 }
                }
            }
        }

        card_divider = <View> {
            width: Fill, height: 1
            show_bg: true
            draw_bg: { color: #bcc9ca4d }
        }

        card_body = <View> {
            width: Fill, height: Fit
            padding: {left: 22, right: 22, top: 18, bottom: 18}
            flow: Down
        }
    }

    // ── Transparent input used inside a labeled card body ──
    HubCardInput = <TextInput> {
        width: Fill, height: 64
        padding: {left: 0, right: 0, top: 0, bottom: 0}
        label_align: {x: 0.0, y: 0.0}
        draw_bg: {
            fn pixel(self) -> vec4 { return #00000000; }
        }
        draw_text: {
            fn get_color(self) -> vec4 { return #131d21; }
            color: #131d21
            color_empty: #3d494a66
            text_style: { font_size: 13.5 }
        }
        draw_cursor: {
            uniform border_radius: 0.5
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.border_radius);
                sdf.fill(mix(#00000000, #131d21, (1.0 - self.blink) * self.focus));
                return sdf.result;
            }
        }
    }

    // Solid bordered card matching the System Prompt / User Message cards.
    HubPanelOutput = <View> {
        width: Fill, height: Fit
        padding: {left: 28, right: 28, top: 28, bottom: 28}
        margin: {top: 0, bottom: 12}
        flow: Overlay
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 8.0);
                sdf.fill_keep(#ffffff);
                sdf.stroke(#bcc9ca, 1.0);
                return sdf.result;
            }
        }

        // Placeholder shown when no output yet
        output_placeholder = <View> {
            width: Fill, height: Fit
            flow: Down
            align: {x: 0.5}
            spacing: 10
            visible: true

            <View> {
                width: 44, height: 44
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 {
                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                        let cx = 22.0;
                        let cy = 22.0;
                        // 4-point sparkle drawn as two crossed diamonds (renders cleanly)
                        sdf.move_to(cx, cy - 16.0);
                        sdf.line_to(cx + 3.2, cy);
                        sdf.line_to(cx, cy + 16.0);
                        sdf.line_to(cx - 3.2, cy);
                        sdf.close_path();
                        sdf.fill(#3d494a66);
                        sdf.move_to(cx - 16.0, cy);
                        sdf.line_to(cx, cy - 3.2);
                        sdf.line_to(cx + 16.0, cy);
                        sdf.line_to(cx, cy + 3.2);
                        sdf.close_path();
                        sdf.fill(#3d494a66);
                        return sdf.result;
                    }
                }
            }
            <Label> {
                text: "Output will appear here after generation..."
                draw_text: {
                    color: #3d494a80
                    text_style: { font_size: 13.0 }
                }
            }
        }

        // Actual output text, hidden until output arrives
        output_label = <Label> {
            width: Fill
            visible: false
            draw_text: {
                fn get_color(self) -> vec4 {
                    return #131d21;
                }
                text_style: { font_size: 13.5 }
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

    // Demo: bg-surface border border-outline-variant p-4 rounded-xl flex items-center gap-4
    HubStatCard = <View> {
        width: Fill, height: 120
        padding: {left: 14, right: 14, top: 10, bottom: 10}
        margin: {right: 12}
        flow: Right
        align: {y: 0.5}
        spacing: 12
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 8.0);
                sdf.fill_keep(#ffffff);
                sdf.stroke(#bcc9ca, 1.0);
                return sdf.result;
            }
        }
    }

    // 40×40 rounded-full icon container. `kind` selects shape; bg/fg colors fixed per instance.
    HubStatCardIcon = <View> {
        width: 40, height: 40
        show_bg: true
        draw_bg: {
            // 0=check, 1=disk, 2=cpu, 3=gauge
            instance kind: 0.0
            instance status: 0.0   // kept for compatibility with apply_over status updates (no-op)
            instance bg_col: vec4(0.86, 0.99, 0.91, 1.0)   // bg-green-100
            instance fg_col: vec4(0.086, 0.639, 0.290, 1.0) // text-green-600
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let cx = 20.0;
                let cy = 20.0;
                // Background circle
                sdf.circle(cx, cy, 19.5);
                sdf.fill(self.bg_col);

                // Symbol: checkmark for kind ~ 0
                if self.kind < 0.5 {
                    sdf.move_to(cx - 6.0, cy + 0.5);
                    sdf.line_to(cx - 1.5, cy + 5.0);
                    sdf.line_to(cx + 6.5, cy - 4.0);
                    sdf.stroke(self.fg_col, 2.2);
                }
                // Disk: stacked ellipses (database cylinder)
                if self.kind > 0.5 && self.kind < 1.5 {
                    // Top oval
                    sdf.box(cx - 7.0, cy - 7.5, 14.0, 4.0, 2.0);
                    sdf.fill(self.fg_col);
                    // Side body
                    sdf.box(cx - 7.0, cy - 5.5, 14.0, 9.0, 0.0);
                    sdf.fill(self.fg_col);
                    // Inner cutout band
                    sdf.box(cx - 5.0, cy - 1.0, 10.0, 1.4, 0.4);
                    sdf.fill(self.bg_col);
                    // Bottom rounded
                    sdf.box(cx - 7.0, cy + 3.0, 14.0, 4.0, 2.0);
                    sdf.fill(self.fg_col);
                }
                // CPU chip: square with corner pins
                if self.kind > 1.5 && self.kind < 2.5 {
                    // Outer chip
                    sdf.box(cx - 6.5, cy - 6.5, 13.0, 13.0, 2.0);
                    sdf.fill(self.fg_col);
                    // Inner cutout
                    sdf.box(cx - 4.0, cy - 4.0, 8.0, 8.0, 1.0);
                    sdf.fill(self.bg_col);
                    // Pins (top/bottom/left/right small bars)
                    sdf.box(cx - 4.5, cy - 9.5, 1.5, 3.0, 0.4);  sdf.fill(self.fg_col);
                    sdf.box(cx - 0.75, cy - 9.5, 1.5, 3.0, 0.4); sdf.fill(self.fg_col);
                    sdf.box(cx + 3.0, cy - 9.5, 1.5, 3.0, 0.4);  sdf.fill(self.fg_col);
                    sdf.box(cx - 4.5, cy + 6.5, 1.5, 3.0, 0.4);  sdf.fill(self.fg_col);
                    sdf.box(cx - 0.75, cy + 6.5, 1.5, 3.0, 0.4); sdf.fill(self.fg_col);
                    sdf.box(cx + 3.0, cy + 6.5, 1.5, 3.0, 0.4);  sdf.fill(self.fg_col);
                }
                // Gauge: arc with needle
                if self.kind > 2.5 {
                    // Outer ring stroke as arc by overdrawing semicircle and clipping
                    sdf.circle(cx, cy + 1.0, 7.5);
                    sdf.stroke(self.fg_col, 1.6);
                    // Needle pointing upper-right
                    sdf.move_to(cx, cy + 1.0);
                    sdf.line_to(cx + 4.5, cy - 4.0);
                    sdf.stroke(self.fg_col, 1.6);
                    // Hub
                    sdf.circle(cx, cy + 1.0, 1.4);
                    sdf.fill(self.fg_col);
                }

                return sdf.result;
            }
        }
    }

    HubStatCardLabel = <Label> {
        // Demo: font-label-sm text-on-surface-variant/60 uppercase text-[10px]
        draw_text: {
            color: #3d494a99
            text_style: <FONT_SEMIBOLD>{ font_size: 9.5 }
        }
    }

    HubStatCardValue = <Label> {
        width: Fill, height: Fit
        margin: {top: -4}
        // Demo: font-bold text-on-surface
        draw_text: {
            color: #131d21
            text_style: <FONT_BOLD>{ font_size: 12.5 }
            wrap: Word
        }
    }

    // Shared model detail header included in each type panel
    HubPanelHeader = <View> {
        width: Fill, height: Fit
        flow: Down
        padding: {left: 16, right: 30, top: 16, bottom: 6}

        <View> {
            width: Fill, height: Fit
            flow: Right
            align: {y: 0.0}
            margin: {bottom: 14}

            <View> {
                width: Fill, height: Fit
                flow: Down
                // Demo: text-display-xl font-black text-on-surface
                panel_model_name = <Label> {
                    draw_text: {
                        fn get_color(self) -> vec4 { return #131d21; }
                        text_style: <FONT_BOLD>{ font_size: 24.0 }
                    }
                }

                // Demo: text-on-surface-variant/80 font-body-md max-w-lg
                panel_model_desc = <Label> {
                    width: Fill, height: Fit
                    margin: {top: 2}
                    draw_text: {
                        fn get_color(self) -> vec4 { return #3d494acc; }
                        text_style: { font_size: 13.0 }
                        wrap: Word
                    }
                }
            }

            <View> {
                width: Fit, height: Fit
                flow: Right
                align: {y: 0.5}
                // `spacing` only applies between *visible* children, so the right edge of
                // whichever sub-row ends up showing sits flush against this Fit container's
                // right boundary — unlike per-child `margin: right` which leaves a phantom
                // gap when the bearing sibling is hidden.
                spacing: 12
                margin: {right: -27}
                panel_action_row = <View> {
                    width: Fit, height: Fit
                    flow: Right
                    visible: false
                    panel_cancel_btn = <HubActionButton> {
                        text: "Cancel"
                        visible: false
                        margin: {right: 0}
                        draw_bg: { danger: 1.0 }
                        draw_text: {
                            color: #b91c1c
                            color_hover: #b91c1c
                            color_down: #b91c1c
                            color_focus: #b91c1c
                            color_disabled: #b91c1c
                        }
                    }
                }
                panel_runtime_row = <View> {
                    width: Fit, height: Fit
                    flow: Right
                    align: {y: 0.5}
                    visible: false
                    spacing: 12
                    margin: {right: 27}

                    // Round X button shown only while a remove confirmation is pending —
                    // placed first so when visible it appears to the LEFT of Remove,
                    // roughly where the Load button would normally sit.
                    panel_remove_cancel_btn = <View> {
                        width: 32, height: 32
                        cursor: Hand
                        visible: false
                        margin: {right: 0}
                        align: {x: 0.5, y: 0.5}
                        show_bg: true
                        animator: {
                            hover = {
                                default: off
                                off = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 0.0}} }
                                on  = { from: {all: Forward{duration: 0.1}}, apply: {draw_bg: {hover: 1.0}} }
                            }
                        }
                        draw_bg: {
                            instance hover: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                let cx = self.rect_size.x * 0.5;
                                let cy = self.rect_size.y * 0.5;
                                let r = min(cx, cy) - 0.5;
                                sdf.circle(cx, cy, r);
                                let base = #ffffff;
                                let hover_col = #f5e9e9;
                                sdf.fill_keep(mix(base, hover_col, self.hover));
                                sdf.stroke(#b91c1c, 1.2);
                                // X mark
                                let arm = 5.0;
                                sdf.move_to(cx - arm, cy - arm);
                                sdf.line_to(cx + arm, cy + arm);
                                sdf.stroke(#b91c1c, 1.4);
                                sdf.move_to(cx + arm, cy - arm);
                                sdf.line_to(cx - arm, cy + arm);
                                sdf.stroke(#b91c1c, 1.4);
                                return sdf.result;
                            }
                        }
                    }
                    panel_load_btn = <HubActionButton> {
                        text: "Load"
                        visible: false
                        margin: {right: 0}
                        draw_bg: { purple: 1.0 }
                    }
                    panel_remove_btn = <HubActionButton> {
                        text: "Remove"
                        visible: false
                        margin: {right: 0}
                        draw_bg: { danger: 1.0 }
                        draw_text: {
                            color: #b91c1c
                            color_hover: #b91c1c
                            color_down: #b91c1c
                            color_focus: #b91c1c
                            color_disabled: #b91c1c
                        }
                    }
                    panel_unload_btn = <HubActionButton> {
                        text: "Unload"
                        visible: false
                        margin: {right: 0}
                    }
                    panel_chat_btn = <HubActionButton> {
                        text: "Open in Chat"
                        margin: {right: 0}
                    }
                }
                // Loading label is wrapped in its own Fit View so its 27px right margin
                // (which compensates for the outer wrapper's -27 margin) only counts in
                // the flow when this row is actually visible. When `visible: false`, the
                // whole holder collapses to zero size and contributes nothing — leaving
                // the Load/Remove button layout unaffected.
                panel_loading_row = <View> {
                    width: Fit, height: Fit
                    align: {y: 0.5}
                    visible: false
                    margin: {right: 27}
                    panel_loading_label = <Label> {
                        draw_text: {
                            fn get_color(self) -> vec4 { return #5b45d8; }
                            text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                        }
                        text: ""
                    }
                }
                // Same wrapping rationale for the Download button — keep its margin scoped
                // to when it's actually shown.
                panel_download_row = <View> {
                    width: Fit, height: Fit
                    align: {y: 0.5}
                    visible: false
                    margin: {right: 27}
                    panel_download_btn = <HubActionButton> {
                        text: "Download"
                        margin: {right: 0}
                        draw_bg: { purple: 1.0 }
                    }
                }
            }
        }

        <View> {
            width: Fill, height: Fit
            flow: Right
            margin: {bottom: 6}

            // STATUS — green check
            <HubStatCard> {
                panel_status_dot = <HubStatCardIcon> {
                    draw_bg: {
                        kind: 0.0
                        bg_col: vec4(0.863, 0.988, 0.906, 1.0)
                        fg_col: vec4(0.086, 0.639, 0.290, 1.0)
                    }
                }
                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    <HubStatCardLabel> { text: "STATUS" }
                    panel_status_text = <HubStatCardValue> {}
                }
            }

            // DISK SIZE — primary teal disk
            <HubStatCard> {
                <HubStatCardIcon> {
                    draw_bg: {
                        kind: 1.0
                        bg_col: vec4(0.886, 0.957, 0.969, 1.0)
                        fg_col: vec4(0.0, 0.412, 0.447, 1.0)
                    }
                }
                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    <HubStatCardLabel> { text: "DISK SIZE" }
                    panel_size_text = <HubStatCardValue> {}
                }
            }

            // PEAK RAM — secondary purple chip
            <HubStatCard> {
                <HubStatCardIcon> {
                    draw_bg: {
                        kind: 2.0
                        bg_col: vec4(0.929, 0.918, 1.0, 1.0)
                        fg_col: vec4(0.322, 0.251, 0.867, 1.0)
                    }
                }
                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    <HubStatCardLabel> { text: "PEAK RAM" }
                    panel_mem_text = <HubStatCardValue> {}
                }
            }

            // QUANTIZATION — neutral gauge
            <HubStatCard> {
                margin: {right: 0}
                <HubStatCardIcon> {
                    draw_bg: {
                        kind: 3.0
                        bg_col: vec4(0.851, 0.894, 0.914, 1.0)
                        fg_col: vec4(0.239, 0.286, 0.290, 1.0)
                    }
                }
                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    <HubStatCardLabel> { text: "QUANTIZATION" }
                    <HubStatCardValue> { text: "4-bit MLX" }
                }
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
                return #ffffff;
            }
        }

        // ── Left panel ──────────────────────────────────────────────────────
        hub_left_panel = <View> {
            width: 320, height: Fill
            flow: Down
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 { return #ffffff; }
            }

            // Header
            <View> {
                width: Fill, height: 56
                padding: {left: 16, right: 16}
                align: {y: 0.5}
                hub_title_label = <Label> {
                    text: "Model Hub"
                    draw_text: {
                        fn get_color(self) -> vec4 {
                            return #15242b;
                        }
                        text_style: <FONT_SEMIBOLD>{ font_size: 16.5 }
                    }
                }
            }

            // Divider
            hub_header_divider = <View> {
                width: Fill, height: 0
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 { return #f1f5f9; }
                }
            }

            // Search
            <View> {
                width: Fill, height: Fit
                padding: {left: 16, right: 16, top: 0, bottom: 10}
                search_container = <View> {
                    width: Fill, height: 40
                    padding: {left: 14, right: 14}
                    flow: Right
                    align: {y: 0.5}
                    show_bg: true
                    draw_bg: {
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            // Search bg: same neutral gray as sidebar/workspace for cohesion
                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 6.0);
                            sdf.fill(#f3f5f9);
                            return sdf.result;
                        }
                    }
                    <Icon> {
                        icon_walk: {width: 18, height: 18, margin: {right: 8}}
                        draw_icon: {
                            svg_file: (ICON_SEARCH)
                            color: #5f7078
                        }
                    }
                    search_input = <TextInput> {
                        width: Fill, height: 34
                        padding: {left: 0, right: 0, top: 7, bottom: 7}
                        label_align: {x: 0.0, y: 0.5}
                        empty_text: "Search models..."
                        draw_bg: {
                            fn pixel(self) -> vec4 {
                                return #00000000;
                            }
                        }
                        draw_text: {
                            fn get_color(self) -> vec4 { return #374151; }
                            color: #374151
                            color_empty: #9ca3af
                            text_style: { font_size: 12.5 }
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

        // Vertical divider
        hub_main_divider = <View> {
            width: 1, height: Fill
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    return #bcc9ca;
                }
            }
        }

        // ── Right panel: type-aware Overlay ──────────────────────────────────
        hub_right_panel = <View> {
            width: Fill, height: Fill
            flow: Overlay
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 { return #f3f5f9; }
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

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 16, right: 16, top: 0, bottom: 28}

                    // ── SYSTEM PROMPT card ──
                    llm_system_card = <HubLabeledCard> {
                        card_header = {
                            header_icon = { draw_bg: {
                                kind: 0.0
                                fg_col: vec4(0.0, 0.412, 0.447, 1.0)  // primary
                            } }
                            header_label = { text: "SYSTEM PROMPT" }
                        }
                        card_body = {
                            llm_system = <HubCardInput> {
                                height: 80
                                empty_text: "You are a helpful assistant..."
                            }
                        }
                    }

                    // ── USER MESSAGE card with inset Generate button ──
                    llm_user_card = <HubLabeledCard> {
                        card_header = {
                            header_icon = { draw_bg: {
                                kind: 1.0
                                fg_col: vec4(0.322, 0.251, 0.867, 1.0)  // secondary
                            } }
                            header_label = { text: "USER MESSAGE" }
                        }
                        card_body = {
                            llm_user = <HubCardInput> {
                                height: 120
                                empty_text: "Type your message here..."
                            }

                            // Generate button row inside the card body, aligned right
                            <View> {
                                width: Fill, height: Fit
                                flow: Right
                                align: {y: 0.5}
                                margin: {top: 10}
                                <View> { width: Fill }
                                llm_generate_btn = <HubGenerateButton> {}
                            }
                        }
                    }

                    // ── RESPONSE section (label above + dashed-border placeholder box) ──
                    <Label> {
                        margin: {top: 8, bottom: 8, left: 0}
                        text: "RESPONSE"
                        draw_text: {
                            color: #3d494a
                            text_style: <FONT_BOLD>{ font_size: 11.0 }
                        }
                    }
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

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 0, bottom: 28}

                    <HubInputLabel> { text: "IMAGE FILE" margin: {top: 0, bottom: 6} }

                    // Drag-and-drop zone for image files from Finder
                    vlm_drop_zone = <View> {
                        width: Fill, height: 132
                        margin: {bottom: 14}
                        padding: {left: 12, right: 12, top: 12, bottom: 12}
                        flow: Overlay
                        cursor: Hand
                        event_order: Down
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

                        vlm_image_preview = <Image> {
                            width: Fill, height: Fill
                            visible: false
                            fit: Smallest
                        }

                        vlm_drop_label = <Label> {
                            text: "Click or drag file to this area"
                            draw_text: {
                                color: #9ca3af
                                text_style: { font_size: 12.0 }
                            }
                        }

                        <View> {
                            width: Fill, height: Fill
                            align: {x: 1.0, y: 1.0}
                            vlm_preview_fullscreen_btn = <View> {
                                width: 52, height: 30
                                visible: false
                                cursor: Hand
                                align: {x: 0.5, y: 0.5}
                                show_bg: true
                                draw_bg: {
                                    instance hover: 0.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                        sdf.fill_keep(mix(vec4(0.067, 0.094, 0.153, 0.80), vec4(0.067, 0.094, 0.153, 0.93), self.hover));
                                        sdf.stroke(vec4(1.0, 1.0, 1.0, 0.20), 1.0);
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
                                    text: "Full"
                                    draw_text: {
                                        color: #ffffff
                                        text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                    }
                                }
                            }
                        }
                    }

                    // ── USER MESSAGE card with inset Generate button ──
                    vlm_user_card = <HubLabeledCard> {
                        card_header = {
                            header_icon = { draw_bg: {
                                kind: 1.0
                                fg_col: vec4(0.322, 0.251, 0.867, 1.0)  // secondary
                            } }
                            header_label = { text: "USER MESSAGE" }
                        }
                        card_body = {
                            vlm_user = <HubCardInput> {
                                height: 120
                                empty_text: "Describe this image..."
                            }

                            // Generate button row inside the card body, aligned right
                            <View> {
                                width: Fill, height: Fit
                                flow: Right
                                align: {y: 0.5}
                                margin: {top: 10}
                                <View> { width: Fill }
                                vlm_generate_btn = <HubGenerateButton> {}
                            }
                        }
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

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 0, bottom: 28}

                    <HubInputLabel> { text: "AUDIO FILE" margin: {top: 0, bottom: 6} }

                    // Drag-and-drop zone for audio files from Finder
                    asr_drop_zone = <View> {
                        width: Fill, height: 96
                        margin: {bottom: 14}
                        padding: {left: 12, right: 12, top: 12, bottom: 12}
                        flow: Overlay
                        cursor: Hand
                        event_order: Down
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

                        asr_drop_label = <Label> {
                            text: "Click or drag file to this area"
                            draw_text: {
                                color: #9ca3af
                                text_style: { font_size: 12.0 }
                            }
                        }

                        asr_audio_preview = <View> {
                            width: Fill, height: Fit
                            visible: false
                            flow: Right
                            align: {y: 0.5}
                            spacing: 10

                            asr_audio_name = <Label> {
                                width: Fill, height: Fit
                                draw_text: {
                                    color: #374151
                                    text_style: <FONT_MEDIUM>{ font_size: 12.0 }
                                }
                            }

                            asr_play_btn = <View> {
                                width: 68, height: 30
                                cursor: Hand
                                align: {x: 0.5, y: 0.5}
                                show_bg: true
                                draw_bg: {
                                    instance hover: 0.0
                                    fn pixel(self) -> vec4 {
                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                        sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                        sdf.fill_keep(mix(#ffffff, #f3f4f6, self.hover));
                                        sdf.stroke(#d1d5db, 1.0);
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
                                asr_play_label = <Label> {
                                    text: "Play"
                                    draw_text: {
                                        color: #374151
                                        text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                                    }
                                }
                            }
                        }
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

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 0, bottom: 28}

                    <HubInputLabel> { text: "VOICE" margin: {top: 0, bottom: 6} }
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

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 0, bottom: 28}

                    <HubInputLabel> { text: "PROMPT" margin: {top: 0, bottom: 6} }
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
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 0, bottom: 28}

                    <HubInputLabel> { text: "REFERENCE IMAGE" margin: {top: 0, bottom: 6} }

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

                <View> {
                    width: Fill, height: Fit
                    flow: Down
                    padding: {left: 30, right: 30, top: 0, bottom: 28}

                    <HubInputLabel> { text: "PROMPT" margin: {top: 0, bottom: 6} }
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
                        draw_text: { color: #b91c1c }
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

            vlm_fullscreen_overlay = <View> {
                width: Fill, height: Fill
                visible: false
                flow: Overlay
                padding: {left: 28, right: 28, top: 28, bottom: 28}
                show_bg: true
                draw_bg: {
                    fn pixel(self) -> vec4 {
                        return #000000cc;
                    }
                }
                vlm_fullscreen_image = <Image> {
                    width: Fill, height: Fill
                    fit: Smallest
                }
                <View> {
                    width: Fill, height: Fill
                    align: {x: 1.0, y: 0.0}
                    vlm_fullscreen_close_btn = <HubActionButton> {
                        text: "Close"
                        margin: {right: 0}
                    }
                }
            }
        }
    }
}
