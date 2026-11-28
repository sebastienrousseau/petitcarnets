//! Rendering a block of text in a style, as Lip Gloss's `Style.Render`
//! does: colours on each line, wrapping at the width, padding, the
//! height, lines brought to one width, the border, then the margins.

use super::{Style, width, wrap};

impl Style {
    /// `text` as a block in this style: Lip Gloss's `Style.Render`.
    /// Each line is painted on its own; then come the padding, the
    /// height, the lines padded to the widest (or to the width), the
    /// border and the margins. Text wider than a set width is wrapped
    /// first. A plain style returns the text unchanged.
    ///
    /// ```
    /// use petitcarnets::style::Style;
    ///
    /// assert_eq!(Style::new().padding_sides(0, 0, 1, 2).render("a\nbc"), "  a \n  bc\n    ");
    /// assert_eq!(Style::new().width(4).render("one two"), "one \ntwo ");
    /// ```
    #[must_use]
    pub fn render(&self, text: &str) -> String {
        if *self == Self::new() {
            return text.to_string();
        }
        let text = text.replace("\r\n", "\n").replace('\t', "    ");
        let [b_top, b_right, b_bottom, b_left] = self.border_widths();
        let inner_width = usize::from(self.frame.width).saturating_sub(b_left + b_right);
        let inner_height = usize::from(self.frame.height).saturating_sub(b_top + b_bottom);
        let [p_top, p_right, p_bottom, p_left] = self.frame.padding.map(usize::from);
        let text = if inner_width > 0 {
            wrap(&text, inner_width.saturating_sub(p_left + p_right))
        } else {
            text
        };
        let mut lines: Vec<String> = text.split('\n').map(|l| self.paint(l)).collect();
        let space = self.whitespace();
        let (left, right) = (
            space.paint(&" ".repeat(p_left)),
            space.paint(&" ".repeat(p_right)),
        );
        if p_left + p_right > 0 {
            lines = lines.iter().map(|l| format!("{left}{l}{right}")).collect();
        }
        let blank = || String::new();
        let mut padded: Vec<String> = std::iter::repeat_with(blank).take(p_top).collect();
        padded.append(&mut lines);
        let mut lines = padded;
        lines.extend(std::iter::repeat_with(blank).take(p_bottom));
        if lines.len() < inner_height {
            lines.resize_with(inner_height, blank);
        }
        let block = align(&lines, inner_width, &space);
        self.apply_margins(&self.apply_border(&block))
    }

    /// The style spaces get: the background, and the colours too when
    /// reversed, as Lip Gloss's whitespace.
    fn whitespace(&self) -> Self {
        let reversed = self.attributes & super::REVERSE != 0;
        Self {
            fg: if reversed { self.fg } else { None },
            bg: self.bg,
            attributes: self.attributes & super::REVERSE,
            ..Self::new()
        }
    }

    /// `block` with the margins around it.
    fn apply_margins(&self, block: &str) -> String {
        let [top, right, bottom, left] = self.frame.margin.map(usize::from);
        if top + right + bottom + left == 0 {
            return block.to_string();
        }
        let lines: Vec<String> = block
            .split('\n')
            .map(|l| format!("{}{l}{}", " ".repeat(left), " ".repeat(right)))
            .collect();
        let wide = lines.iter().map(|l| width(l)).max().unwrap_or(0);
        let blank = " ".repeat(wide);
        let mut out = vec![blank.clone(); top];
        out.extend(lines);
        out.extend(std::iter::repeat_n(blank, bottom));
        out.join("\n")
    }
}

/// `lines` joined, each padded with `space` to the widest line or to
/// `at_least`, when there is more than one line or a width.
fn align(lines: &[String], at_least: usize, space: &Style) -> String {
    if lines.len() < 2 && at_least == 0 {
        return lines.join("\n");
    }
    let widest = lines
        .iter()
        .map(|l| width(l))
        .max()
        .unwrap_or(0)
        .max(at_least);
    let padded: Vec<String> = lines
        .iter()
        .map(|l| {
            let short = widest - width(l);
            if short == 0 {
                l.clone()
            } else {
                format!("{l}{}", space.paint(&" ".repeat(short)))
            }
        })
        .collect();
    padded.join("\n")
}
