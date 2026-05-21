use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    use crate::theme::*;

    // ========================================================================
    // SHARED TEXT STYLES
    // Reusable label templates with consistent typography
    // ========================================================================

    // Section title: semibold, 16px, primary color
    pub SectionTitle = <Label> {
        draw_text: {
            color: (TEXT_PRIMARY)
            text_style: <FONT_SEMIBOLD>{ font_size: 16.0 }
        }
    }

    // Body text: regular, 11px, secondary gray
    pub BodyText = <Label> {
        draw_text: {
            color: (GRAY_700)
            text_style: <FONT_REGULAR>{ font_size: 11.0 }
        }
    }

    // Hint/muted text: regular, 10px, muted gray
    pub HintText = <Label> {
        draw_text: {
            color: (TEXT_MUTED)
            text_style: <FONT_REGULAR>{ font_size: 10.0 }
        }
    }

    // ========================================================================
    // CHAT LIST ITEM
    // Sidebar chat history item with hover and selected states
    // ========================================================================

    pub ChatListItem = <View> {
        width: Fill, height: 24
        padding: {left: 0, right: 8}
        margin: {right: 6, bottom: 2}
        align: {y: 0.5}
        cursor: Hand
        show_bg: true
        draw_bg: {
            instance selected: 0.0
            instance hover: 0.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let base = #00000000;
                let hover = #00000000;
                let selected = #d8e7eb;
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 7.0);
                sdf.fill(mix(mix(base, hover, self.hover), selected, self.selected));
                return sdf.result;
            }
        }
        animator: {
            hover = {
                default: off
                off = { from: {all: Forward{duration: 0.0}}, apply: {draw_bg: {hover: 0.0}} }
                on  = { from: {all: Forward{duration: 0.0}}, apply: {draw_bg: {hover: 1.0}} }
            }
        }
        title = <Label> {
            width: Fill, height: 16
            flow: Right
            draw_text: {
                color: vec4(0.32, 0.40, 0.43, 1.0)
                text_style: <FONT_MEDIUM>{ font_size: 11.0 }
                wrap: Ellipsis
            }
        }
    }
}
