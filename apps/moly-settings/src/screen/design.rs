//! Settings Screen UI Design

use makepad_widgets::*;

use super::SettingsApp;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use moly_widgets::theme::*;
    use makepad_component::widgets::switch::*;

    // Provider icons - registered for dynamic loading
    ICON_OPENAI = dep("crate://self/resources/providers/openai.png")
    ICON_ANTHROPIC = dep("crate://self/resources/providers/anthropic.png")
    ICON_GEMINI = dep("crate://self/resources/providers/gemini.png")
    ICON_OLLAMA = dep("crate://self/resources/providers/ollama.png")
    ICON_DEEPSEEK = dep("crate://self/resources/providers/deepseek.png")
    ICON_NVIDIA = dep("crate://self/resources/providers/nvidia.png")
    ICON_GROQ = dep("crate://self/resources/providers/groq.png")
    ICON_KIMI = dep("crate://self/resources/providers/kimi.png")
    ICON_ZHIPU = dep("crate://self/resources/providers/zhipu.png")

    // Settings label style
    SettingsLabel = <Label> {
        draw_text: {
            fn get_color(self) -> vec4 {
                return #374151;
            }
            text_style: <FONT_SEMIBOLD>{ font_size: 11.0 }
        }
    }

    // Settings hint/helper text
    SettingsHint = <Label> {
        draw_text: {
            fn get_color(self) -> vec4 {
                return #9ca3af;
            }
            text_style: <FONT_REGULAR>{ font_size: 10.0 }
        }
    }

    // Text input for settings
    SettingsTextInput = <TextInput> {
        width: Fill, height: 44
        padding: {left: 12, right: 12, top: 10, bottom: 10}
        cursor: Text

        draw_bg: {
            instance radius: 6.0
            instance border_width: 1.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let sz = self.rect_size - 2.0;
                sdf.box(1.0, 1.0, sz.x, sz.y, max(1.0, self.radius - self.border_width));

                sdf.fill(#ffffff);
                sdf.stroke(#d1d5db, self.border_width);
                return sdf.result;
            }
        }

        draw_text: {
            fn get_color(self) -> vec4 {
                return #1f2937;
            }
            text_style: <FONT_REGULAR>{ font_size: 12.0 }
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

    // Toggle switch using MpSwitch from makepad-component
    EnableToggle = <MpSwitch> {}

    // Status indicator dot
    StatusDot = <View> {
        width: 8, height: 8
        show_bg: true
        draw_bg: {
            // status: 0=not_connected (gray), 1=connecting (yellow), 2=connected (green), 3=error (red)
            instance status: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let center = self.rect_size / 2.0;
                let radius = min(center.x, center.y);
                sdf.circle(center.x, center.y, radius);

                // Color based on status
                let gray = #9ca3af;
                let yellow = #f59e0b;
                let green = #22c55e;
                let red = #ef4444;

                // Select color based on status value
                let color = mix(
                    mix(gray, yellow, clamp(self.status, 0.0, 1.0)),
                    mix(green, red, clamp(self.status - 2.0, 0.0, 1.0)),
                    step(1.5, self.status)
                );

                sdf.fill(color);
                return sdf.result;
            }
        }
    }

    // Provider list item
    ProviderItem = <View> {
        width: Fill, height: Fit
        padding: {left: 16, right: 16, top: 12, bottom: 12}
        cursor: Hand
        show_bg: true

        draw_bg: {
            instance hover: 0.0
            instance selected: 0.0

            fn pixel(self) -> vec4 {
                let base = #ffffff;
                let hover_color = #f1f5f9;
                let selected_color = #dbeafe;
                return mix(mix(base, hover_color, self.hover), selected_color, self.selected);
            }
        }

        flow: Right
        align: {y: 0.5}
        spacing: 12

        provider_icon = <Image> {
            width: 32, height: 32
            fit: Smallest
        }

        // Status indicator
        status_dot = <StatusDot> {}

        provider_name = <Label> {
            width: Fill
            draw_text: {
                fn get_color(self) -> vec4 {
                    return #1f2937;
                }
                text_style: <FONT_REGULAR>{ font_size: 11.3 }
            }
        }

        // Enabled toggle on the right
        provider_enabled = <EnableToggle> {}
    }

    // Save button
    SaveButton = <Button> {
        width: Fit, height: 40
        padding: {left: 20, right: 20, top: 10, bottom: 10}

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let sz = self.rect_size - 2.0;
                // Blue button colors: #3b82f6 -> #2563eb -> #1d4ed8
                let base_color = vec4(0.231, 0.510, 0.965, 1.0);
                let hover_color = vec4(0.145, 0.388, 0.922, 1.0);
                let pressed_color = vec4(0.114, 0.306, 0.847, 1.0);
                let color = mix(
                    mix(base_color, hover_color, self.hover),
                    pressed_color,
                    self.pressed
                );
                sdf.box(1.0, 1.0, sz.x, sz.y, self.radius);
                sdf.fill(color);
                return sdf.result;
            }
        }

        draw_text: {
            color: #ffffff
            text_style: <FONT_SEMIBOLD>{ font_size: 12.0 }
        }

        text: "Save"
    }

    // Test Connection button (secondary style)
    TestButton = <Button> {
        width: Fit, height: 40
        padding: {left: 20, right: 20, top: 10, bottom: 10}

        draw_bg: {
            instance hover: 0.0
            instance pressed: 0.0
            instance radius: 6.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let sz = self.rect_size - 2.0;
                // Secondary button: gray outline style
                let bg_color = mix(#ffffff, #f3f4f6, self.hover);
                sdf.box(1.0, 1.0, sz.x, sz.y, self.radius);
                sdf.fill(bg_color);
                sdf.stroke(#d1d5db, 1.0);
                return sdf.result;
            }
        }

        draw_text: {
            fn get_color(self) -> vec4 {
                return #374151;
            }
            text_style: <FONT_SEMIBOLD>{ font_size: 12.0 }
        }

        text: "Test Connection"
    }

    pub SettingsApp = {{SettingsApp}} {
        width: Fill, height: Fill
        flow: Right
        show_bg: true
        draw_bg: {
            fn pixel(self) -> vec4 {
                return #f5f7fa;
            }
        }

        // Provider icons for dynamic loading (order: openai, anthropic, gemini, ollama, deepseek, nvidia, groq, kimi)
        provider_icons: [
            (ICON_OPENAI),
            (ICON_ANTHROPIC),
            (ICON_GEMINI),
            (ICON_OLLAMA),
            (ICON_DEEPSEEK),
            (ICON_NVIDIA),
            (ICON_GROQ),
            (ICON_KIMI),
            (ICON_ZHIPU),
        ]

        // Left panel - provider list
        providers_panel = <View> {
            width: 280, height: Fill
            flow: Down
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    return #ffffff;
                }
            }

            // Header with Add button
            <View> {
                width: Fill, height: Fit
                flow: Right
                padding: {left: 16, right: 16, top: 16, bottom: 12}
                align: {y: 0.5}

                header_label = <Label> {
                    text: "Providers"
                    draw_text: {
                        fn get_color(self) -> vec4 {
                            return #1f2937;
                        }
                        text_style: <FONT_SEMIBOLD>{ font_size: 20.0 }
                    }
                }

                <View> { width: Fill } // Spacer

                add_provider_button = <Button> {
                    width: 28, height: 28
                    padding: 0
                    draw_bg: {
                        instance hover: 0.0
                        instance pressed: 0.0
                        instance radius: 4.0

                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let sz = self.rect_size - 2.0;
                            let color = mix(vec4(0.0), #e5e7eb, self.hover);
                            sdf.box(1.0, 1.0, sz.x, sz.y, self.radius);
                            sdf.fill(color);
                            return sdf.result;
                        }
                    }
                    draw_text: {
                        fn get_color(self) -> vec4 {
                            return #374151;
                        }
                        text_style: <FONT_SEMIBOLD>{ font_size: 16.0 }
                    }
                    text: "+"
                }
            }

            // Provider list (dynamic)
            providers_list = <PortalList> {
                width: Fill, height: Fill
                drag_scrolling: false

                ProviderListItem = <ProviderItem> {}
            }
        }

        // Divider
        <View> {
            width: 1, height: Fill
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    return #e5e7eb;
                }
            }
        }

        // Right panel - provider details
        provider_view = <View> {
            width: Fill, height: Fill
            flow: Down
            padding: 24
            spacing: 20

            // Header with title and enabled checkbox on same row
            provider_header = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 4

                // Title row with checkbox on the right
                title_row = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    align: {y: 0.5}
                    spacing: 12

                    provider_title_icon = <Image> {
                        width: 32, height: 32
                        fit: Smallest
                        source: (ICON_OPENAI)
                    }

                    provider_title = <Label> {
                        text: "OpenAI"
                        draw_text: {
                            fn get_color(self) -> vec4 {
                                return #1f2937;
                            }
                            text_style: <FONT_SEMIBOLD>{ font_size: 20.0 }
                        }
                    }
                }

                provider_type_label = <Label> {
                    text: "OpenAI Compatible API"
                    draw_text: {
                        fn get_color(self) -> vec4 {
                            return #6b7280;
                        }
                        text_style: <FONT_REGULAR>{ font_size: 12.0 }
                    }
                }
            }

            // API Host section
            host_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 6

                <SettingsLabel> { text: "API Host" }
                api_host_input = <SettingsTextInput> {
                    text: "https://api.openai.com/v1"
                }
                <SettingsHint> { text: "The base URL for API requests" }
            }

            // API Key section
            key_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 6

                <SettingsLabel> { text: "API Key" }
                api_key_input = <SettingsTextInput> {
                    is_password: true
                    empty_text: "sk-..."
                }
                <SettingsHint> { text: "Your API key (stored locally)" }
            }

            // A2UI section (only visible for OpenAI-compatible providers)
            a2ui_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 6
                visible: true

                a2ui_header = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    align: {y: 0.5}
                    spacing: 12

                    <SettingsLabel> { text: "A2UI (AI-to-UI)" }

                    <View> { width: Fill } // Spacer

                    a2ui_toggle = <EnableToggle> {}
                }

                a2ui_hint = <SettingsHint> {
                    text: "Enable AI-generated UI rendering in the Canvas panel"
                }
            }

            // Actions
            actions = <View> {
                width: Fill, height: Fit
                flow: Right
                spacing: 12
                margin: {top: 12}

                save_button = <SaveButton> {}
                test_button = <TestButton> {}

                <View> { width: Fill } // Spacer

                delete_provider_button = <Button> {
                    width: Fit, height: 40
                    padding: {left: 20, right: 20, top: 10, bottom: 10}
                    visible: false

                    draw_bg: {
                        instance hover: 0.0
                        instance pressed: 0.0
                        instance radius: 6.0

                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let sz = self.rect_size - 2.0;
                            // Red button colors: #ef4444 -> #dc2626 -> #b91c1c
                            let base_color = vec4(0.937, 0.267, 0.267, 1.0);
                            let hover_color = vec4(0.863, 0.149, 0.149, 1.0);
                            let pressed_color = vec4(0.725, 0.110, 0.110, 1.0);
                            let color = mix(
                                mix(base_color, hover_color, self.hover),
                                pressed_color,
                                self.pressed
                            );
                            sdf.box(1.0, 1.0, sz.x, sz.y, self.radius);
                            sdf.fill(color);
                            return sdf.result;
                        }
                    }

                    draw_text: {
                        color: #ffffff
                        text_style: <FONT_SEMIBOLD>{ font_size: 12.0 }
                    }

                    text: "Delete"
                }
            }

            // Status message
            status_message = <Label> {
                text: ""
                draw_text: {
                    fn get_color(self) -> vec4 {
                        return #059669;
                    }
                    text_style: <FONT_REGULAR>{ font_size: 11.0 }
                }
            }

            // Models section (shown after successful connection test)
            models_section = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 8
                margin: {top: 16}
                visible: false

                // Header row with label and Select All toggle
                models_header_row = <View> {
                    width: Fill, height: Fit
                    flow: Right
                    align: {y: 0.5}
                    spacing: 12

                    models_header = <Label> {
                        text: "Available Models"
                        draw_text: {
                            fn get_color(self) -> vec4 {
                                return #374151;
                            }
                            text_style: <FONT_SEMIBOLD>{ font_size: 13.0 }
                        }
                    }

                    <View> { width: Fill } // Spacer

                    select_all_label = <Label> {
                        text: "Select All"
                        draw_text: {
                            fn get_color(self) -> vec4 {
                                return #6b7280;
                            }
                            text_style: <FONT_REGULAR>{ font_size: 11.0 }
                        }
                    }

                    select_all_toggle = <EnableToggle> {}
                }

                models_scroll = <View> {
                    width: Fill, height: 200
                    flow: Down
                    show_bg: true
                    draw_bg: {
                        instance radius: 6.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let sz = self.rect_size - 2.0;
                            sdf.box(1.0, 1.0, sz.x, sz.y, self.radius);
                            sdf.fill(#f9fafb);
                            sdf.stroke(#e5e7eb, 1.0);
                            return sdf.result;
                        }
                    }

                    models_list = <PortalList> {
                        width: Fill, height: Fill
                        drag_scrolling: false

                        ModelItem = <View> {
                            width: Fill, height: Fit
                            padding: {left: 12, right: 12, top: 8, bottom: 8}
                            flow: Right
                            align: {y: 0.5}
                            spacing: 12

                            model_enabled = <EnableToggle> {}

                            model_name = <Label> {
                                width: Fill
                                draw_text: {
                                    fn get_color(self) -> vec4 {
                                        return #374151;
                                    }
                                    text_style: <FONT_REGULAR>{ font_size: 11.0 }
                                }
                            }
                        }
                    }
                }
            }

            // Spacer
            <View> { width: Fill, height: Fill }

            // ── App Info / Update Section ────────────────────────────────
            <View> {
                width: Fill, height: Fit
                flow: Right
                align: {y: 0.5}
                padding: {left: 24, right: 24, bottom: 20}
                spacing: 12

                version_label = <Label> {
                    width: Fill
                    draw_text: {
                        color: #9ca3af
                        text_style: <FONT_REGULAR>{ font_size: 11.0 }
                    }
                }

                update_status_label = <Label> {
                    width: Fit
                    draw_text: {
                        color: #16a39c
                        text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                    }
                }

                check_update_button = <TestButton> {
                    text: "Check for Updates"
                }
            }
        }

        // Add Provider Modal (overlay)
        add_provider_modal = <View> {
            width: Fill, height: Fill
            flow: Overlay
            visible: false
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    return vec4(0.0, 0.0, 0.0, 0.5); // Semi-transparent backdrop
                }
            }

            // Center the modal content
            <View> {
                width: Fill, height: Fill
                align: {x: 0.5, y: 0.5}

                modal_content = <View> {
                    width: 400, height: Fit
                    flow: Down
                    padding: 24
                    spacing: 16
                    show_bg: true
                    draw_bg: {
                        instance radius: 8.0
                        fn pixel(self) -> vec4 {
                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                            let sz = self.rect_size - 2.0;
                            sdf.box(1.0, 1.0, sz.x, sz.y, self.radius);
                            // Slightly gray background so white inputs stand out
                            sdf.fill(#f3f4f6);
                            sdf.stroke(#d1d5db, 1.0);
                            return sdf.result;
                        }
                    }

                    // Modal header
                    modal_header = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        align: {y: 0.5}

                        modal_title = <Label> {
                            text: "Add Provider"
                            draw_text: {
                                fn get_color(self) -> vec4 {
                                    return #1f2937;
                                }
                                text_style: <FONT_SEMIBOLD>{ font_size: 18.0 }
                            }
                        }

                        <View> { width: Fill } // Spacer

                        close_modal_button = <Button> {
                            width: 24, height: 24
                            padding: 0
                            draw_bg: {
                                instance hover: 0.0
                                instance pressed: 0.0
                                instance radius: 4.0

                                fn pixel(self) -> vec4 {
                                    let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                    let sz = self.rect_size - 2.0;
                                    let color = mix(vec4(0.0), #e5e7eb, self.hover);
                                    sdf.box(1.0, 1.0, sz.x, sz.y, self.radius);
                                    sdf.fill(color);
                                    return sdf.result;
                                }
                            }
                            draw_text: {
                                fn get_color(self) -> vec4 {
                                    return #6b7280;
                                }
                                text_style: <FONT_REGULAR>{ font_size: 14.0 }
                            }
                            text: "×"
                        }
                    }

                    // Provider name input
                    name_section = <View> {
                        width: Fill, height: Fit
                        flow: Down
                        spacing: 6

                        <SettingsLabel> { text: "Provider Name" }
                        new_provider_name = <SettingsTextInput> {
                            empty_text: "My Provider"
                        }
                    }

                    // API URL input
                    url_section = <View> {
                        width: Fill, height: Fit
                        flow: Down
                        spacing: 6

                        <SettingsLabel> { text: "API URL" }
                        new_provider_url = <SettingsTextInput> {
                            text: "https://api.example.com/v1"
                            empty_text: "https://api.example.com/v1"
                        }
                        <SettingsHint> { text: "OpenAI-compatible API endpoint" }
                    }

                    // API Key input
                    key_section = <View> {
                        width: Fill, height: Fit
                        flow: Down
                        spacing: 6

                        <SettingsLabel> { text: "API Key (optional)" }
                        new_provider_key = <SettingsTextInput> {
                            is_password: true
                            empty_text: "sk-..."
                        }
                    }

                    // Modal actions
                    modal_actions = <View> {
                        width: Fill, height: Fit
                        flow: Right
                        spacing: 12
                        margin: {top: 8}
                        align: {x: 1.0}

                        cancel_modal_button = <TestButton> {
                            text: "Cancel"
                        }
                        save_new_provider_button = <SaveButton> {
                            text: "Add Provider"
                        }
                    }
                }
            }
        }
    }
}
