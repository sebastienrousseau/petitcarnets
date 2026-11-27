package main

import "strings"

// corpus is every string the fixture records, in a fixed order.
func corpus() []string {
	var c []string
	for _, part := range []func() []string{
		samples, styled, oddEscapes, sweeps, joined, mixed,
	} {
		c = append(c, part()...)
	}
	return c
}

// samples is hand picked text, by script and feature.
func samples() []string {
	return []string{
		"",
		"a", "Hello, World!", " ", "~", "tab\there", "line\nbreak", "cr\r\nlf",
		// Latin, precomposed and decomposed.
		"café", "cafe\u0301", "naïve", "nai\u0308ve", "Ångström", "A\u030angstro\u0308m",
		"e\u0301\u0302\u0323", "o\u0308\u0304", "ﬁ", "ǅ", "ß", "œuvre",
		"\u0301", "\u0301\u0302", " \u0301", "a\u20dd", "x\u0489",
		// CJK and fullwidth forms.
		"中文", "日本語のテキスト", "漢字かなカナ", "한국어", "ｱｲｳ", "ＡＢＣ１２３", "！？",
		"〜", "\u3000", "中a文b", "𠀀𪚥", "\u3000x", "￥€£",
		// Hangul syllables and conjoining jamo.
		"한글", "\u1100\u1161", "\u1100\u1161\u11a8", "\u1100\u1100\u1161\u11a8\u11a8",
		"가\u11a8", "각\u11a8", "\u1161", "\u11a8", "\u1100", "\u115f\u1160",
		"\ua960\ud7b0\ud7cb", "\u1100가", "가\u1161",
		// Emoji.
		"😀", "😀😃😄", "👍", "👍🏻", "👍🏽", "👍🏿", "🏽", "👨\u200d👩\u200d👧\u200d👦", "👩\u200d💻", "🧑🏽\u200d🚀",
		"🏳\ufe0f\u200d🌈", "🏴\u200d☠\ufe0f", "🇫🇷", "🇫🇷🇩🇪", "🇫", "🇫🇷🇩", "🇺🇸🇬🇧🇯🇵", "1\ufe0f\u20e3", "#\ufe0f\u20e3", "*\ufe0f\u20e3",
		"1\u20e3", "❤", "❤\ufe0f", "❤\ufe0e", "☺", "☺\ufe0f", "☺\ufe0e", "✌\ufe0f", "✌🏾", "⌚", "⌚\ufe0e",
		"🏴\U000e0067\U000e0062\U000e0073\U000e0063\U000e0074\U000e007f",
		"🏴\U000e0067\U000e0062\U000e0065\U000e006e\U000e0067\U000e007f",
		"\U000e0067", "👁\ufe0f\u200d🗨\ufe0f", "🤷\u200d♂\ufe0f", "©", "©\ufe0f", "®\ufe0f", "™\ufe0f", "‼\ufe0f", "a\ufe0f", "\ufe0f",
		"\u200d", "😀\u200d", "\u200d😀", "😀\u200d\u200d😀", "😀\u0301\u200d😀",
		// Combining marks.
		"a\u0300\u0301\u0302\u0303\u0304", "\u0336s\u0336t\u0336", "Z\u0351\u036b\u0343\u036a",
		"i\u0307", "א\u05b8", "क\u093c",
		// Devanagari and Hindi, conjuncts included.
		"ह\u093fन\u094dद\u0940", "नमस\u094dत\u0947", "क\u094dष", "क\u094d\u200dष", "क\u094d\u200cष", "त\u094dर", "ज\u094dञ", "स\u094dत\u0947", "क\u094dक\u094dक",
		"क\u094d\u094dक", "क\u093c", "\u094d", "क\u094d", "अ", "ॐ", "द\u094dध\u094dर", "ङ\u094dक", "र\u094d\u200d",
		"क\u094dष\u093f", "श\u094dर\u0940", "प\u094dर\u093e", "क\u094dष\u094dम\u094dय",
		// Bengali, Tamil, Thai, Arabic, Hebrew.
		"ব\u09be\u0982ল\u09be", "ক\u09cdষ", "স\u09cdত\u09cdর", "ক\u09cd\u200dষ", "தம\u0bbfழ\u0bcd", "க\u0bcdஷ", "ஸ\u0bcdர\u0bc0", "ภาษาไทย", "ก\u0e33", "ก\u0e33",
		"น\u0e49\u0e33", "العربية", "مرحبا", "لا", "\u0600١", "\u0600a", "\u06dd١٢", "ש\u05b8\u05c1לו\u05b9ם", "ע\u05b4ב\u05b0ר\u05b4ית",
		"ก\u0e34\u0e4a", "เก", "ക\u0d4dഷ", "ક\u0acdષ",
		// Zero width characters.
		"\u200b", "a\u200bb", "\u200c", "a\u200cb", "\u2060", "a\u2060b", "\ufeff", "\ufeffa",
		"\u00ad", "soft\u00adhyphen", "\u200e\u200f", "\u2061\u2062", "\u180e", "\u034f",
		"\u115f", "\u1160", "\u3164", "\uffa0",
		// Controls: C0, DEL and C1 (UTF-8 encoded).
		"\x00", "\x01", "\a", "\b", "\x7f", "a\x00b", "\u0080", "\u0085", "\u009b",
		"\u009b31m", "\u009d0;t\u009c", "\u0090q\u009c", "a\u009cb", "\u00a0", "\u00a0\u0301",
		"\r", "\n", "\r\n", "\n\r", "\r\u0301", "\x01\u0301",
		// Private use, noncharacters, unassigned, specials.
		"\ue000", "\U000f0000", "\U0010fffd", "\ufdd0", "\uffff", "\U0001fffe", "\u0378",
		"\U000e0001", "\U000e0100", "�", "￼",
	}
}

// styled is text with the escape sequences programs commonly print.
func styled() []string {
	return []string{
		"\x1b[31mred\x1b[0m", "\x1b[1;4;31mbold\x1b[m", "\x1b[38;5;196mx\x1b[39m",
		"\x1b[38;2;255;128;0mtrue\x1b[0m", "\x1b[38:2::255:128:0mcolon\x1b[0m",
		"\x1b[48;5;21m 中 \x1b[49m", "\x1b[4:3munder\x1b[24m", "\x1b[58;2;1;2;3mx",
		"\x1b]8;;https://example.com\x07link\x1b]8;;\x07",
		"\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\",
		"\x1b]8;id=1;http://a.b/c?d=é\x1b\\é\x1b]8;;\x1b\\",
		"\x1b]0;title\x07text", "\x1b]2;中文\x1b\\x", "\x1b]52;c;aGVsbG8=\x07",
		"\x1b[2J\x1b[H", "\x1b[10;20Hx", "\x1b[A\x1b[B\x1b[C\x1b[D", "\x1b[?25l\x1b[?25h",
		"\x1b[?1049h", "\x1b[>4;2m", "\x1b[=5u", "\x1b[<0;1;1M", "\x1b[ q", "\x1b[2 q",
		"\x1b[!p", "\x1b[0\"q", "\x1b7\x1b8", "\x1bc", "\x1b=", "\x1b>", "\x1b(B", "\x1b)0",
		"\x1b#8", "\x1b%G", "\x1bM", "\x1bD", "\x1bE", "\x1bN", "\x1bO", "\x1b\\",
		"\x1bP1$r0m\x1b\\", "\x1bP+q544e\x1b\\", "\x1bPq#0;2;0;0;0#0~~@@vv@@~~\x1b\\",
		"\x1b_Gf=24,s=1,v=1;AAAA\x1b\\", "\x1b^privacy\x1b\\", "\x1bXsos\x1b\\",
		"a\x1b[31mé\x1b[0m", "a\x1b[31m\u0301", "e\x1b[0m\u0301", "\x1b[1m👨\u200d👩\u200d👧\x1b[0m",
		"\x1b[7m🇫🇷\x1b[27m", "x\x1b]8;;u\x07\u0301", "\x1b[31m\x1b[1m\x1b[4mx\x1b[0m",
		"\x1bPtmux;\x1b\x1b[31m\x1b\\x", "\x1b]1337;File=:AAAA\x07", "\x1b]11;?\x07",
		"\x1b[6n", "\x1b[c", "\x1b[>c", "\x1b[0;0r", "\x1b[3~", "\x1b[200~paste\x1b[201~",
	}
}

// oddEscapes are malformed, cut off and cancelled sequences.
func oddEscapes() []string {
	return []string{
		"\x1b", "a\x1b", "\x1b\x1b", "\x1b\x1b[m", "\x1b[", "\x1b[31", "\x1b[31;", "\x1b[?",
		"\x1b[1\x0a2m", "\x1b[1\x18m", "\x1b[1\x1am", "\x1b[1\x07m", "\x1b[1\x7fm", "\x1b[\x1b[m",
		"\x1b[1?h", "\x1b[1 2m", "\x1b[ 1m", "\x1b[é", "\x1b[1é", "\x1b[1;中m", "\x1b[1\u0085m",
		"\x1b]", "\x1b]0;t", "\x1b]0;t\x18x", "\x1b]0;t\x1ax", "\x1b]0;t\x1bx", "\x1b]0;t\x1b",
		"\x1b]0;\x01\x02\x07", "\x1b]0;é\x07", "\x1b]0;Ĝ", "\x1b]0;Ĝx", "\x1b]0;ܜx",
		"\x1b]0;a\u009cb", "\x1b]8;;\x1b\\", "\x1bP", "\x1bPq", "\x1bPqdata",
		"\x1bP1;2|x\x07y\x1b\\", "\x1bP\x18x", "\x1bPq\x18x", "\x1bPqé\x1b\\", "\x1bP1é",
		"\x1b_", "\x1b_x", "\x1b_x\x07y\x1b\\", "\x1b_é\x1b\\", "\x1b_Ĝz", "\x1b_a\x18b",
		"\x1b^x\x1a", "\x1bXé", "\x1b(", "\x1b( ", "\x1b(\x01B", "\x1b(é", "\x1b([1m", "\x1b(]0;x\x07",
		"\x1b(P1;2qx\x1b\\", "\x1b \x7f", "\x1b\x7f", "\x1b\x01[m", "\x1bé", "\x1b\u0301",
		"\x1b[38;5;\x1b[0m", "\x1b[m\x1b", "\x1b]\x1b]", "\x1b[\u0085", "\x1b]\u0085\x07",
		"\x1b_\u0090x\x1b\\", "\x1b[0\u009b1m", "a\x1b[0m\u0301\u0302", "\x1b[0m\u0301",
		"\x1b]0;x\x1b[31m", "\x1bP\x1b\\", "\x1bP\x1bx\x1b\\", "\x1b[<", "\x1b[<?1m",
		"\x1b[?<1m", "\x1b[1:2:3m", "\x1b[;;m", "\x1b[:m", "\x1b[@", "\x1b[~",
	}
}

// sweep is every code point from lo to hi, on its own and, when prefix
// is set, after it.
func sweep(lo, hi, step rune, prefix string) []string {
	var c []string
	for r := lo; r <= hi; r += step {
		if r >= 0xd800 && r <= 0xdfff {
			continue
		}
		c = append(c, string(r))
		if prefix != "" {
			c = append(c, prefix+string(r))
		}
	}
	return c
}

// sweeps is every code point in ranges whose widths or boundaries vary.
func sweeps() []string {
	var c []string
	for _, s := range []struct {
		lo, hi, step rune
		prefix       string
	}{
		{0x00, 0xff, 1, ""},
		{0x300, 0x36f, 1, "a"},
		{0x483, 0x489, 1, "a"},
		{0x590, 0x5ff, 1, "א"},
		{0x600, 0x6ff, 1, "ب"},
		{0x900, 0x97f, 1, "क"},
		{0x980, 0x9ff, 1, "ক"},
		{0xb80, 0xbff, 1, "க"},
		{0xe00, 0xe7f, 1, "ก"},
		{0x1100, 0x11ff, 1, "\u1100"},
		{0x1ab0, 0x1aff, 1, "a"},
		{0x200b, 0x206f, 1, "a"},
		{0x20d0, 0x20ff, 1, "a"},
		{0x2190, 0x21ff, 1, ""},
		{0x2600, 0x27bf, 2, ""},
		{0x2e80, 0x2eff, 4, ""},
		{0x3000, 0x303f, 1, ""},
		{0x3099, 0x309a, 1, "か"},
		{0xa960, 0xa97f, 1, "\u1100"},
		{0xac00, 0xac1f, 1, "\u1100"},
		{0xd7b0, 0xd7ff, 1, "가"},
		{0xfe00, 0xfe0f, 1, "❤"},
		{0xfe10, 0xfe6f, 1, ""},
		{0xff00, 0xffef, 1, ""},
		{0xfff0, 0xffff, 1, ""},
		{0x1f000, 0x1f0ff, 3, ""},
		{0x1f1e6, 0x1f1ff, 1, "\U0001f1eb"},
		{0x1f300, 0x1f5ff, 2, ""},
		{0x1f3fb, 0x1f3ff, 1, "\U0001f44d"},
		{0x1f600, 0x1f64f, 1, ""},
		{0x1f680, 0x1f6ff, 1, ""},
		{0x1f900, 0x1f9ff, 1, ""},
		{0x1fa70, 0x1faff, 1, ""},
		{0x20000, 0x2000f, 1, ""},
		{0x1d165, 0x1d17a, 1, "a"},
		{0xe0000, 0xe007f, 1, "\U0001f3f4"},
		{0xe0100, 0xe010f, 1, "a"},
	} {
		c = append(c, sweep(s.lo, s.hi, s.step, s.prefix)...)
	}
	return c
}

// probes stand for each grapheme break property and width class.
var probes = []string{
	"a", "\r", "\n", "\x01", "\u0301", "\u200d", "\U0001f1eb", "\u0600", "\u0903",
	"\u1100", "\u1161", "\u11a8", "가", "각", "\U0001f600", "क", "\u094d",
	"\u093c", "\u0e33", "\ufe0f", "\u200b", "中", "\x1b[0m",
}

// joined is every probe followed by every probe, so each pair of
// properties meets.
func joined() []string {
	var c []string
	for _, a := range probes {
		for _, b := range probes {
			c = append(c, a+b)
		}
	}
	return c
}

// atoms are the pieces mixed strings are made of.
var atoms = []string{
	"a", "Z", "0", " ", ";", "[", "]", "\\", "P", "_", "^", "X", "m", "31", "?", "(", "~",
	"é", "e\u0301", "\u0301", "\u0308", "中", "ｱ", "Ａ", "한", "\u1100", "\u1161", "\u11a8",
	"😀", "👍", "🏽", "\u200d", "👨", "🇫", "🇷", "1", "\ufe0f", "\ufe0e", "\u20e3", "❤", "🏴",
	"\U000e0067", "\U000e007f", "\u200b", "\u200c", "\u2060", "\ufeff", "\u00ad", "क", "ष",
	"\u094d", "\u093f", "\u09bf", "\u09cd", "ক", "த", "\u0bcd", "ก", "\u0e31", "\u0e33", "ع",
	"\u064b", "ש", "\u05b8", "\u0600", "\u0903", "\x00", "\x07", "\b", "\t", "\n", "\r",
	"\x18", "\x1a", "\x1b", "\x7f", "\u0085", "\u009b", "\u009c", "\u009d", "\x1b[", "\x1b]",
	"\x1b\\", "\x1bP", "\x1b_", "\x1b^", "\x1bX", "\x1b[0m", "\x1b[31m", "\x1b]8;;u\x07",
	"Ĝ", "ܜ", "✓", "\U0001f71c", "�", "\ue000", "\u3000",
}

// mixed is deterministic pseudo random strings of atoms.
func mixed() []string {
	var c []string
	state := uint64(0x9e3779b97f4a7c15)
	next := func(n int) int {
		state ^= state << 13
		state ^= state >> 7
		state ^= state << 17
		return int(state % uint64(n))
	}
	for range 1600 {
		var b strings.Builder
		for range 1 + next(10) {
			b.WriteString(atoms[next(len(atoms))])
		}
		c = append(c, b.String())
	}
	return c
}
