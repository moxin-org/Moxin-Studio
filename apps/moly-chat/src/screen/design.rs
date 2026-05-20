//! Chat Screen UI Design
//!
//! Contains the live_design! DSL block defining the UI layout and styling.

use makepad_widgets::*;

use super::{ChatApp, ChatHistoryItem, ChatHistoryPanel};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use moly_widgets::theme::*;
    use moly_kit::widgets::chat::Chat;
    use moly_kit::widgets::prompt_input::PromptInput;

    // Provider icons - registered so they can be loaded at runtime
    ICON_OPENAI = dep("crate://self/resources/providers/openai.png")
    ICON_ANTHROPIC = dep("crate://self/resources/providers/anthropic.png")
    ICON_GEMINI = dep("crate://self/resources/providers/gemini.png")
    ICON_OLLAMA = dep("crate://self/resources/providers/ollama.png")
    ICON_DEEPSEEK = dep("crate://self/resources/providers/deepseek.png")
    ICON_OPENROUTER = dep("crate://self/resources/providers/openrouter.png")
    ICON_SILICONFLOW = dep("crate://self/resources/providers/siliconflow.png")
    ICON_NVIDIA = dep("crate://self/resources/providers/nvidia.png")
    ICON_GROQ = dep("crate://self/resources/providers/groq.png")
    ICON_ZHIPU = dep("crate://self/resources/providers/zhipu.png")

    // Delete icon for chat history
    ICON_TRASH = dep("crate://self/resources/icons/trash.svg")

    // Individual chat history item - Widget with proper event handling
    pub ChatHistoryItem = {{ChatHistoryItem}} {
        width: Fill, height: Fit
        padding: {left: 12, right: 8, top: 8, bottom: 8}
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance selected: 0.0
            instance hover: 0.0
            instance down: 0.0
            fn pixel(self) -> vec4 {
                let base = #ffffff;
                let selected_color = #eaecf0;
                let hover_color = #eaecf0;
                let color = mix(base, selected_color, self.selected);
                return mix(color, hover_color, self.hover * (1.0 - self.selected));
            }
        }

        // Animator enables finger event handling
        animator: {
            hover = {
                default: off
                off = {
                    from: {all: Forward {duration: 0.15}}
                    apply: {
                        draw_bg: {hover: 0.0}
                    }
                }
                on = {
                    from: {all: Forward {duration: 0.15}}
                    apply: {
                        draw_bg: {hover: 1.0}
                    }
                }
            }
            down = {
                default: off
                off = {
                    from: {all: Forward {duration: 0.2}}
                    apply: {
                        draw_bg: {down: 0.0}
                    }
                }
                on = {
                    from: {all: Forward {duration: 0.1}}
                    apply: {
                        draw_bg: {down: 1.0}
                    }
                }
            }
        }

        flow: Right
        spacing: 4
        align: {y: 0.5}

        // Left side: title and date
        content = <View> {
            width: Fill, height: Fit
            flow: Down
            spacing: 2

            title_label = <Label> {
                width: Fill
                draw_text: {
                    color: #1f2937
                    text_style: { font_size: 12.0 }
                    wrap: Ellipsis
                }
                text: "New Session"
            }

            category_row = <View> {
                width: Fit, height: Fit
                flow: Right
                spacing: 4
                category_tag = <RoundedView> {
                    width: Fit, height: Fit
                    padding: {left: 4, right: 4, top: 1, bottom: 1}
                    show_bg: true
                    draw_bg: { border_radius: 3.0, color: #6366f1 }
                    visible: false
                    tag_label = <Label> {
                        draw_text: { color: #ffffff, text_style: <FONT_MEDIUM>{ font_size: 7.5 } }
                        text: ""
                    }
                }
                date_label = <Label> {
                    width: Fit
                    draw_text: {
                        color: #6b7280
                        text_style: { font_size: 10.0 }
                    }
                    text: ""
                }
            }
        }

        // Right side: delete button (visible on hover)
        delete_button = <View> {
            width: 24, height: 24
            align: {x: 0.5, y: 0.5}
            cursor: Hand
            show_bg: true
            draw_bg: {
                instance hover: 0.0
                fn pixel(self) -> vec4 {
                    let hover_color = #fee2e2;
                    return mix(vec4(0.0, 0.0, 0.0, 0.0), hover_color, self.hover);
                }
            }

            animator: {
                hover = {
                    default: off
                    off = {
                        from: {all: Forward {duration: 0.1}}
                        apply: { draw_bg: {hover: 0.0} }
                    }
                    on = {
                        from: {all: Forward {duration: 0.1}}
                        apply: { draw_bg: {hover: 1.0} }
                    }
                }
            }

            delete_icon = <Icon> {
                draw_icon: {
                    svg_file: (ICON_TRASH)
                    color: #9ca3af
                }
                icon_walk: { width: 18, height: 18 }
            }
        }
    }

    // Template alias for PortalList
    ChatHistoryItemTemplate = <ChatHistoryItem> {}

    // Chat history panel as a separate widget
    pub ChatHistoryPanel = {{ChatHistoryPanel}} {
        width: 220, height: Fill
        flow: Down
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                return #f8fafc;
            }
        }

        // New chat button
        new_chat_container = <View> {
            width: Fill, height: Fit
            padding: 12

            new_chat_button = <Button> {
                width: Fill, height: Fit
                padding: {left: 12, right: 12, top: 10, bottom: 10}
                text: "+ New Session"
                draw_text: {
                    text_style: { font_size: 12.0 }
                    color: #ffffff
                }
                draw_bg: {
                    instance hover: 0.0
                    instance pressed: 0.0
                    fn pixel(self) -> vec4 {
                        let base = #3b82f6;
                        let hover_color = #2055ff;
                        let pressed_color = #1045cc;
                        let color = mix(base, hover_color, self.hover);
                        return mix(color, pressed_color, self.pressed);
                    }
                }
            }
        }

        // History header
        history_header = <View> {
            width: Fill, height: Fit
            padding: {left: 12, right: 12, top: 8, bottom: 8}

            history_title = <Label> {
                text: "History"
                draw_text: {
                    color: #6b7280
                    text_style: { font_size: 11.0 }
                }
            }
        }

        // Chat history list
        history_list = <PortalList> {
            width: Fill, height: Fill
            flow: Down

            ChatHistoryItem = <ChatHistoryItem> {}
        }
    }

    pub ChatApp = {{ChatApp}} {
        width: Fill, height: Fill
        flow: Down
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                return #f5f7fa;
            }
        }

        // Provider icons for model selector and chat messages
        // Order: openai, anthropic, gemini, ollama, deepseek, openrouter, siliconflow, nvidia, groq, zhipu
        provider_icons: [
            (ICON_OPENAI),
            (ICON_ANTHROPIC),
            (ICON_GEMINI),
            (ICON_OLLAMA),
            (ICON_DEEPSEEK),
            (ICON_OPENROUTER),
            (ICON_SILICONFLOW),
            (ICON_NVIDIA),
            (ICON_GROQ),
            (ICON_ZHIPU),
        ]

        // Header with provider status
        header = <View> {
            width: Fill, height: Fit
            flow: Down
            padding: 16
            spacing: 4

            title_label = <Label> {
                text: "Session"
                draw_text: {
                    color: #1f2937
                    text_style: <FONT_SEMIBOLD>{ font_size: 20.0 }
                }
            }

        }

        // Mode-specific controls bar (VLM image, TTS voice, Image settings, ASR upload)
        mode_controls = <View> {
            width: Fill, height: Fit
            flow: Down
            padding: {left: 16, right: 16, bottom: 8}
            visible: false

            // ── VLM image upload ──────────────────────────────────────
            vlm_controls = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 8
                visible: false

                <Label> {
                    width: Fit, height: Fit
                    text: "Image"
                    draw_text: { color: #374151, text_style: <FONT_SEMIBOLD>{ font_size: 12.0 } }
                }

                vlm_drop_zone = <View> {
                    width: Fill, height: 64
                    show_bg: true
                    draw_bg: {
                        instance hover: 0.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 8.0);
                            sdf.fill(mix(#f9fafb, #eef2ff, self.hover));
                            sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 8.0);
                            sdf.stroke(mix(#d1d5db, #818cf8, self.hover), 1.0);
                            return sdf.result;
                        }
                    }
                    align: {x: 0.5, y: 0.5}
                    vlm_drop_label = <Label> {
                        text: "Drop image here"
                        draw_text: { color: #9ca3af, text_style: <FONT_REGULAR>{ font_size: 12.0 } }
                    }
                }

                vlm_file_row = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 8

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 8
                        align: {y: 0.5}

                        vlm_browse_btn = <View> {
                            width: Fit, height: 28, cursor: Hand
                            align: {x: 0.5, y: 0.5}
                            padding: {left: 12, right: 12}
                            show_bg: true
                            draw_bg: {
                                instance hover: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                    sdf.fill(mix(#6366f1, #4f46e5, self.hover));
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
                            <Label> { text: "Browse...", draw_text: { color: #ffffff, text_style: <FONT_MEDIUM>{ font_size: 12.0 } } }
                        }
                        vlm_file_label = <Label> {
                            width: Fill, height: Fit
                            text: ""
                            draw_text: { color: #6b7280, text_style: <FONT_REGULAR>{ font_size: 11.0 } }
                        }
                        vlm_clear_btn = <View> {
                            width: Fit, height: 24, cursor: Hand
                            align: {x: 0.5, y: 0.5}
                            padding: {left: 8, right: 8}
                            visible: false
                            show_bg: true
                            draw_bg: {
                                instance hover: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 4.0);
                                    sdf.fill(mix(#fee2e2, #fecaca, self.hover));
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
                            <Label> { text: "Clear", draw_text: { color: #dc2626, text_style: <FONT_MEDIUM>{ font_size: 10.0 } } }
                        }
                    }

                    vlm_preview = <Image> {
                        width: 200, height: 120
                        visible: false
                        fit: Smallest
                    }
                }
            }

            // ── TTS voice selector ─────────────────────────────────────
            tts_controls = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 6
                visible: false

                <Label> {
                    width: Fit, height: Fit
                    text: "Voice"
                    draw_text: { color: #374151, text_style: <FONT_SEMIBOLD>{ font_size: 12.0 } }
                }
                tts_voice_row = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    spacing: 6
                    margin: {bottom: 4}

                    tts_voice_0 = <View> {
                        width: Fit, height: 28, padding: {left: 10, right: 10}, cursor: Hand
                        align: {x: 0.5, y: 0.5}, show_bg: true
                        draw_bg: { instance selected: 1.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#f3f4f6, #fef3c7, self.selected));
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.stroke(mix(#d1d5db, #f59e0b, self.selected), 1.0);
                                return sdf.result;
                            }
                        }
                        <Label> { text: "Vivian", draw_text: { color: #374151, text_style: <FONT_MEDIUM>{ font_size: 11.0 } } }
                    }
                    tts_voice_1 = <View> {
                        width: Fit, height: 28, padding: {left: 10, right: 10}, cursor: Hand
                        align: {x: 0.5, y: 0.5}, show_bg: true
                        draw_bg: { instance selected: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#f3f4f6, #fef3c7, self.selected));
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.stroke(mix(#d1d5db, #f59e0b, self.selected), 1.0);
                                return sdf.result;
                            }
                        }
                        <Label> { text: "Serena", draw_text: { color: #374151, text_style: <FONT_MEDIUM>{ font_size: 11.0 } } }
                    }
                    tts_voice_2 = <View> {
                        width: Fit, height: 28, padding: {left: 10, right: 10}, cursor: Hand
                        align: {x: 0.5, y: 0.5}, show_bg: true
                        draw_bg: { instance selected: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#f3f4f6, #fef3c7, self.selected));
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.stroke(mix(#d1d5db, #f59e0b, self.selected), 1.0);
                                return sdf.result;
                            }
                        }
                        <Label> { text: "Ryan", draw_text: { color: #374151, text_style: <FONT_MEDIUM>{ font_size: 11.0 } } }
                    }
                    tts_voice_3 = <View> {
                        width: Fit, height: 28, padding: {left: 10, right: 10}, cursor: Hand
                        align: {x: 0.5, y: 0.5}, show_bg: true
                        draw_bg: { instance selected: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#f3f4f6, #fef3c7, self.selected));
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.stroke(mix(#d1d5db, #f59e0b, self.selected), 1.0);
                                return sdf.result;
                            }
                        }
                        <Label> { text: "Aiden", draw_text: { color: #374151, text_style: <FONT_MEDIUM>{ font_size: 11.0 } } }
                    }
                    tts_voice_4 = <View> {
                        width: Fit, height: 28, padding: {left: 10, right: 10}, cursor: Hand
                        align: {x: 0.5, y: 0.5}, show_bg: true
                        draw_bg: { instance selected: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#f3f4f6, #fef3c7, self.selected));
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.stroke(mix(#d1d5db, #f59e0b, self.selected), 1.0);
                                return sdf.result;
                            }
                        }
                        <Label> { text: "English Male", draw_text: { color: #374151, text_style: <FONT_MEDIUM>{ font_size: 11.0 } } }
                    }
                }
                tts_voice_row_zh = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    spacing: 6

                    tts_voice_5 = <View> {
                        width: Fit, height: 28, padding: {left: 10, right: 10}, cursor: Hand
                        align: {x: 0.5, y: 0.5}, show_bg: true
                        draw_bg: { instance selected: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#f3f4f6, #fef3c7, self.selected));
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.stroke(mix(#d1d5db, #f59e0b, self.selected), 1.0);
                                return sdf.result;
                            }
                        }
                        <Label> { text: "Uncle Fu", draw_text: { color: #374151, text_style: <FONT_MEDIUM>{ font_size: 11.0 } } }
                    }
                    tts_voice_6 = <View> {
                        width: Fit, height: 28, padding: {left: 10, right: 10}, cursor: Hand
                        align: {x: 0.5, y: 0.5}, show_bg: true
                        draw_bg: { instance selected: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#f3f4f6, #fef3c7, self.selected));
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.stroke(mix(#d1d5db, #f59e0b, self.selected), 1.0);
                                return sdf.result;
                            }
                        }
                        <Label> { text: "Chinese Female", draw_text: { color: #374151, text_style: <FONT_MEDIUM>{ font_size: 11.0 } } }
                    }
                    tts_voice_7 = <View> {
                        width: Fit, height: 28, padding: {left: 10, right: 10}, cursor: Hand
                        align: {x: 0.5, y: 0.5}, show_bg: true
                        draw_bg: { instance selected: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#f3f4f6, #fef3c7, self.selected));
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.stroke(mix(#d1d5db, #f59e0b, self.selected), 1.0);
                                return sdf.result;
                            }
                        }
                        <Label> { text: "Chinese Male", draw_text: { color: #374151, text_style: <FONT_MEDIUM>{ font_size: 11.0 } } }
                    }
                    tts_voice_8 = <View> {
                        width: Fit, height: 28, padding: {left: 10, right: 10}, cursor: Hand
                        align: {x: 0.5, y: 0.5}, show_bg: true
                        draw_bg: { instance selected: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#f3f4f6, #fef3c7, self.selected));
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.stroke(mix(#d1d5db, #f59e0b, self.selected), 1.0);
                                return sdf.result;
                            }
                        }
                        <Label> { text: "Dialect", draw_text: { color: #374151, text_style: <FONT_MEDIUM>{ font_size: 11.0 } } }
                    }
                }

                // Play/Save controls for latest audio
                tts_audio_controls = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    spacing: 8
                    margin: {top: 4}
                    visible: false

                    tts_play_btn = <View> {
                        width: Fit, height: 26, padding: {left: 10, right: 10}
                        cursor: Hand, align: {x: 0.5, y: 0.5}
                        show_bg: true
                        draw_bg: {
                            instance hover: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#3b82f6, #2563db, self.hover));
                                return sdf.result;
                            }
                        }
                        animator: { hover = {
                            default: off
                            off = { from: {all: Forward{duration: 0.15}}, apply: {draw_bg: {hover: 0.0}} }
                            on  = { from: {all: Forward{duration: 0.15}}, apply: {draw_bg: {hover: 1.0}} }
                        }}
                        tts_play_label = <Label> {
                            text: "▶ Play"
                            draw_text: { color: #ffffff, text_style: <FONT_MEDIUM>{ font_size: 10.5 } }
                        }
                    }
                    tts_save_btn = <View> {
                        width: Fit, height: 26, padding: {left: 10, right: 10}
                        cursor: Hand, align: {x: 0.5, y: 0.5}
                        show_bg: true
                        draw_bg: {
                            instance hover: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#10b981, #059669, self.hover));
                                return sdf.result;
                            }
                        }
                        animator: { hover = {
                            default: off
                            off = { from: {all: Forward{duration: 0.15}}, apply: {draw_bg: {hover: 0.0}} }
                            on  = { from: {all: Forward{duration: 0.15}}, apply: {draw_bg: {hover: 1.0}} }
                        }}
                        <Label> {
                            text: "⬇ Save"
                            draw_text: { color: #ffffff, text_style: <FONT_MEDIUM>{ font_size: 10.5 } }
                        }
                    }
                    tts_audio_info = <Label> {
                        width: Fit, height: Fit
                        text: ""
                        draw_text: { color: #6b7280, text_style: <FONT_REGULAR>{ font_size: 10.0 } }
                    }
                }
            }

            // ── Image gen controls ─────────────────────────────────────
            image_controls = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 8
                visible: false

                image_ref_section = <View> {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 8
                    visible: false

                    <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 12
                        align: {y: 0.5}

                        <Label> {
                            width: Fit, height: Fit
                            text: "Ref Image:"
                            draw_text: { color: #374151, text_style: <FONT_SEMIBOLD>{ font_size: 12.0 } }
                        }
                        image_ref_browse_btn = <View> {
                            width: Fit, height: 28, cursor: Hand
                            align: {x: 0.5, y: 0.5}
                            padding: {left: 12, right: 12}
                            show_bg: true
                            draw_bg: {
                                instance hover: 0.0
                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                    sdf.fill(mix(#f3f4f6, #e5e7eb, self.hover));
                                    sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                    sdf.stroke(#d1d5db, 1.0);
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
                            <Label> { text: "Browse...", draw_text: { color: #374151, text_style: <FONT_REGULAR>{ font_size: 12.0 } } }
                        }
                        image_ref_file_label = <Label> {
                            width: Fit, height: Fit
                            text: "No image selected"
                            draw_text: { color: #9ca3af, text_style: <FONT_REGULAR>{ font_size: 11.0 } }
                        }
                    }

                    image_ref_preview = <Image> {
                        width: 200, height: 120
                        visible: false
                        fit: Smallest
                    }
                }

                image_neg_row = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    spacing: 8
                    align: {y: 0.5}

                    <Label> {
                        width: Fit, height: Fit
                        text: "Negative:"
                        draw_text: { color: #374151, text_style: <FONT_SEMIBOLD>{ font_size: 12.0 } }
                    }
                    image_neg_prompt_input = <TextInput> {
                        width: Fill, height: 28
                        empty_text: "Negative prompt (optional)"
                        draw_bg: {
                            color: #ffffff
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(self.color);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.stroke(#d1d5db, 1.0);
                                return sdf.result;
                            }
                        }
                        draw_text: { color: #1f2937, text_style: <FONT_REGULAR>{ font_size: 12.0 }, wrap: Ellipsis }
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

            // ── ASR audio upload ───────────────────────────────────────
            asr_controls = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 8
                visible: false

                <Label> {
                    width: Fit, height: Fit
                    text: "Audio File"
                    draw_text: { color: #374151, text_style: <FONT_SEMIBOLD>{ font_size: 12.0 } }
                }

                asr_drop_zone = <View> {
                    width: Fill, height: 64
                    show_bg: true
                    draw_bg: {
                        instance hover: 0.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 8.0);
                            sdf.fill(mix(#f9fafb, #ecfdf5, self.hover));
                            sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 8.0);
                            sdf.stroke(mix(#d1d5db, #10b981, self.hover), 1.0);
                            return sdf.result;
                        }
                    }
                    align: {x: 0.5, y: 0.5}
                    asr_drop_label = <Label> {
                        text: "Drop audio file here"
                        draw_text: { color: #9ca3af, text_style: <FONT_REGULAR>{ font_size: 12.0 } }
                    }
                }

                asr_file_row = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    spacing: 8
                    align: {y: 0.5}

                    asr_browse_btn = <View> {
                        width: Fit, height: 28, cursor: Hand
                        align: {x: 0.5, y: 0.5}
                        padding: {left: 12, right: 12}
                        show_bg: true
                        draw_bg: {
                            instance hover: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#10b981, #059669, self.hover));
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
                        <Label> { text: "Browse...", draw_text: { color: #ffffff, text_style: <FONT_MEDIUM>{ font_size: 12.0 } } }
                    }
                    asr_play_btn = <View> {
                        width: Fit, height: 28, cursor: Hand
                        align: {x: 0.5, y: 0.5}
                        padding: {left: 10, right: 10}
                        visible: false
                        show_bg: true
                        draw_bg: {
                            instance hover: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.fill(mix(#f3f4f6, #e5e7eb, self.hover));
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 6.0);
                                sdf.stroke(#d1d5db, 1.0);
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
                        asr_play_label = <Label> { text: "▶ Play", draw_text: { color: #374151, text_style: <FONT_MEDIUM>{ font_size: 11.0 } } }
                    }
                    asr_file_label = <Label> {
                        width: Fill, height: Fit
                        text: ""
                        draw_text: { color: #6b7280, text_style: <FONT_REGULAR>{ font_size: 11.0 } }
                    }
                    asr_clear_btn = <View> {
                        width: Fit, height: 24, cursor: Hand
                        align: {x: 0.5, y: 0.5}
                        padding: {left: 8, right: 8}
                        visible: false
                        show_bg: true
                        draw_bg: {
                            instance hover: 0.0
                            fn pixel(self) -> vec4 {
                                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, 4.0);
                                sdf.fill(mix(#fee2e2, #fecaca, self.hover));
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
                        <Label> { text: "Clear", draw_text: { color: #dc2626, text_style: <FONT_MEDIUM>{ font_size: 10.0 } } }
                    }
                }
            }
        }

        // Main content area - full width chat (history moved to shell sidebar)
        main_content = <View> {
            width: Fill, height: Fill
            flow: Overlay

            // Chat widget from moly-kit (always present)
            chat = <Chat> {
                width: Fill, height: Fill
                prompt = {
                    margin: {left: 20, right: 20, bottom: 20}
                    persistent = {
                        draw_bg: {
                            border_radius: 10.0
                        }
                    }
                }
            }

            // Empty chat welcome overlay (shows greeting when no messages)
            welcome_overlay = <View> {
                width: Fill, height: Fill
                flow: Down
                align: {x: 0.5, y: 0.35}
                spacing: 32
                padding: {left: 48, right: 48}
                visible: true

                // Greeting text
                greeting_label = <Label> {
                    width: Fit, height: Fit
                    text: "What can I help you with?"
                    draw_text: {
                        color: #1f2937
                        text_style: <FONT_SEMIBOLD>{ font_size: 28.0 }
                    }
                }

                // Prompt input
                welcome_prompt = <PromptInput> {
                    width: Fill, height: Fit
                }
            }

        }

    }
}
