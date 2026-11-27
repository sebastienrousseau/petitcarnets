package main

import (
	"fmt"
	"strings"
)

// corpus is every input the fixture records, in a fixed order.
func corpus() []chunks {
	var c []chunks
	for _, part := range []func() []chunks{
		plainBytes, altKeys, utf8Text, cursorKeys, tildeKeys, urxvtKeys,
		kittyKeys, modifyOtherKeys, mice, replies, unknown, pastes, mixed,
	} {
		c = append(c, part()...)
	}
	return c
}

func each(format string, values ...int) []chunks {
	var c []chunks
	for _, v := range values {
		c = append(c, one(fmt.Sprintf(format, v)))
	}
	return c
}

func span(from, to int) []int {
	var v []int
	for i := from; i <= to; i++ {
		v = append(v, i)
	}
	return v
}

// plainBytes is every single byte that is a key or a C1 control.
func plainBytes() []chunks {
	var c []chunks
	for b := 0; b <= 0x9f; b++ {
		c = append(c, chunks{{byte(b)}})
	}
	for _, b := range []byte{0xa0, 0xc3, 0xe2, 0xf0, 0xfe, 0xff} {
		c = append(c, chunks{{b}})
	}
	return c
}

// altKeys is ESC before every ASCII byte.
func altKeys() []chunks {
	var c []chunks
	for b := 0; b <= 0x7f; b++ {
		c = append(c, chunks{{0x1b, byte(b)}})
	}
	return append(c, one("\x1bé"), one("\x1b\x1b\x1b"), one("\x1b\x1b\x1b[A"),
		one("\x1b\x9bA"), one("\x1b\x1bOa"), one("\x1b\x1b[2$"))
}

func utf8Text() []chunks {
	var c []chunks
	for _, s := range []string{
		"é", "€", "日本", "🦀", "é", "é́", "ñ̃x", "👨‍👩‍👧",
		"🇫🇷", "🇫🇷🇩", "👍🏽", "❤️", "한", "각", "क्षि", "ไทย",
		"؀١", "مرحبا", "�", "\xc3", "\xe2\x82", "a\xffb", "\xc0\xaf",
		"\xed\xa0\x80", "\xf4\x90\x80\x80", "é\r\n", "Ωmega", "ǅ", "ß", "­",
		"​", "\U0001F600\U0001F600", "©️", "á",
	} {
		c = append(c, one(s))
	}
	return c
}

var cursorFinals = "ABCDEFHPQRS"

func cursorKeys() []chunks {
	var c []chunks
	for _, f := range cursorFinals {
		c = append(c, one("\x1b["+string(f)), one("\x1bO"+string(f)), one("\x9b"+string(f)))
		for m := 1; m <= 16; m++ {
			c = append(c, one(fmt.Sprintf("\x1b[1;%d%c", m, f)))
		}
	}
	for _, f := range "APMXjkoqy" {
		for m := 2; m <= 16; m += 3 {
			c = append(c, one(fmt.Sprintf("\x1bO%d%c", m, f)))
		}
	}
	for _, s := range "MXjklmnopqrstuvwxyabcdZ" {
		c = append(c, one("\x1bO"+string(s)))
	}
	return append(c, one("\x1b[Z"), one("\x1b[2A"), one("\x1b[;5A"), one("\x1b[1;0A"),
		one("\x1b[1;5:3A"), one("\x1b[1;1:2B"), one("\x1b[1;2;3C"), one("\x1b[1:2;3D"),
		one("\x1bO99999999999999999999A"), one("\x1bO0A"), one("\x1bOz"), one("\x1bO\x01"))
}

var tilde = []int{1, 2, 3, 4, 5, 6, 7, 8, 11, 12, 13, 14, 15, 17, 18, 19, 20, 21,
	23, 24, 25, 26, 28, 29, 31, 32, 33, 34}

func tildeKeys() []chunks {
	c := each("\x1b[%d~", tilde...)
	c = append(c, each("\x1b[%d;5~", tilde...)...)
	for _, n := range []int{1, 2, 3, 5, 6, 15, 24, 34} {
		for m := 1; m <= 16; m++ {
			c = append(c, one(fmt.Sprintf("\x1b[%d;%d~", n, m)))
		}
	}
	c = append(c, each("\x1b[%d~", 9, 10, 16, 22, 27, 30, 35, 99, 200, 201)...)
	return append(c, one("\x1b[3;0~"), one("\x1b[3;5:3~"), one("\x1b[3;1:2~"),
		one("\x1b[3;5;7~"), one("\x1b[~"), one("\x1b[2147483648~"), one("\x1b[4294967299~"))
}

func urxvtKeys() []chunks {
	var c []chunks
	for _, n := range tilde {
		c = append(c, each("\x1b[%d$", n)...)
		c = append(c, each("\x1b[%d^", n)...)
		c = append(c, each("\x1b[%d@", n)...)
	}
	return append(c, one("\x1b[2$a"), one("\x1b[3^x"), one("\x1b[5;3^"), one("\x1b[2;5@"),
		one("\x1b[$"), one("\x1b[99$"))
}

func kittyKeys() []chunks {
	var c []chunks
	c = append(c, each("\x1b[%du", append(span(0, 40), 64, 65, 90, 91, 96, 97, 122, 126, 127,
		128, 159, 160, 233, 1089, 0x4e00, 0x1f600, 0xd800, 0xe09c, 0x10ffff, 1114112,
		2147483647)...)...)
	c = append(c, each("\x1b[%du", span(57344, 57454)...)...)
	for _, m := range append(span(1, 16), 33, 65, 66, 129, 130, 131, 193, 255, 256) {
		c = append(c, each("\x1b[%d;"+fmt.Sprint(m)+"u", 97, 49, 13, 32, 233, 57399, 57410, 57441)...)
	}
	for _, s := range []string{
		"97;1:1", "97;1:2", "97;1:3", "97;5:3", "97;2:2", "57441;2:3", "97:65;2", "97:65:113;2",
		"1089::99;5", "1089::99;1", "97::;1", "97:66;2:3", "97;1;97", "97;2;65", "97;;97:98",
		"0;;104:105", "97;5;97", "97;1;0", "97;1;1114112", ";5", "97:65", "57399;129",
		"57399;1;48", "57400;3", "57413;2", "57416;1", "57415;65", "57414;1", "233;3",
		"97;1:3:9;97", "97;0", "97;", "97:65:113:5;2", "1;1:3", "65;1", "65;2",
	} {
		c = append(c, one("\x1b["+s+"u"))
	}
	return c
}

func modifyOtherKeys() []chunks {
	var c []chunks
	for m := 1; m <= 16; m++ {
		c = append(c, each("\x1b[27;"+fmt.Sprint(m)+";%d~", 8, 9, 13, 27, 127, 97, 65, 32, 49, 233)...)
	}
	return append(c, one("\x1b[27;5~"), one("\x1b[27;0;97~"), one("\x1b[27;;97~"),
		one("\x1b[27;5;10~"), one("\x1b[27;5;1114113~"), one("\x1b[27;1;55296~"),
		one("\x1b[27;2;1114112~"))
}

func mice() []chunks {
	var c []chunks
	buttons := []int{0, 1, 2, 3, 4, 8, 16, 28, 32, 33, 34, 35, 64, 65, 66, 67, 96,
		128, 129, 130, 131, 160, 192}
	for _, b := range buttons {
		for _, f := range "Mm" {
			c = append(c, one(fmt.Sprintf("\x1b[<%d;10;5%c", b, f)))
		}
	}
	for _, xy := range [][2]int{{1, 1}, {80, 24}, {300, 200}, {2000, 1000}, {65536, 1}} {
		c = append(c, one(fmt.Sprintf("\x1b[<0;%d;%dM", xy[0], xy[1])))
	}
	for _, b := range []int{0, 1, 2, 3, 4, 8, 16, 32, 35, 64, 65, 66, 67, 128, 131} {
		c = append(c, chunks{{0x1b, '[', 'M', byte(32 + b), 33 + 9, 33 + 4}})
	}
	return append(c, chunks{{0x1b, '[', 'M', 32, 255, 255}}, one("\x1b[M"), one("\x1b[M!!"),
		one("\x1b[<0;10M"), one("\x1b[<0;10;5;1M"), one("\x1b[<;;M"), one("\x1b[<0;1;1M\x1b[<0;1;1m"),
		one("\x1b[I"), one("\x1b[O"), one("\x1b[1I"), one("\x1b[<64;3;3M\x1b[<65;3;3M"))
}

func replies() []chunks {
	var c []chunks
	for _, s := range []string{
		"\x1b[?2004;1$y", "\x1b[?1004;2$y", "\x1b[?1;0$y", "\x1b[4;1$y", "\x1b[?$y",
		"\x1b[?2026;4$y", "\x1b[?2026$y", "\x1b[20;3$y", "\x1b[?2027;300$y", "\x1b[?62;22c",
		"\x1b[?1;2c", "\x1b[>1;10;0c", "\x1bP!|7E565445\x1b\\", "\x1bP!|7E56544\x1b\\",
		"\x1b[12;40R", "\x1b[1;1R", "\x1b[1;16R", "\x1b[1;17R", "\x1b[1;20R", "\x1b[?5;10R",
		"\x1b[?5R", "\x1b[R", "\x1b[5R", "\x1b[0;0R", "\x1b[?1u", "\x1b[?31u", "\x1b[?u",
		"\x1b[?0u", "\x1bP>|xterm(388)\x1b\\", "\x1bP>|kitty(0.36.0)\x1b\\",
		"\x90>|WezTerm 20240203\x9c", "\x1bP>|\x1b\\", "\x1b]10;rgb:ffff/ffff/ffff\x07",
		"\x1b]11;rgb:0000/0000/0000\x1b\\", "\x1b]11;rgb:1c1c/1c1c/1c1c\x07",
		"\x1b]12;#ff8800\x07", "\x1b]10;#abc\x1b\\", "\x1b]10;rgba:ffff/0000/0000/ffff\x07",
		"\x1b]11;rgb:f/0/8\x07", "\x1b]11;rgb:fff/808/7f7f7\x07", "\x1b]10;nonsense\x07",
		"\x1b]10;rgb:zz/00/00\x07", "\x1b]10;rgb:1/2\x07", "\x1b]10;#12345\x07",
		"\x1b]10;#12345g\x07", "\x1b]10\x07", "\x1b]10;\x07", "\x9d11;rgb:8080/8080/8080\x9c",
		"\x1b]11;rgb:7f7f/7f7f/7f7f\x07", "\x1b]11;rgb:8080/8080/8080\x07",
		"\x1b]11;rgb:ffffffffff/0/0\x07", "\x1b]12;rgb:00/ff/00\x1b\\", "\x1b]11;#FFFFFF\x07",
	} {
		c = append(c, one(s))
	}
	return c
}

func unknown() []chunks {
	var c []chunks
	for _, s := range []string{
		"\x1b[99X", "\x1b[>0q", "\x1b]99;foo\x07", "\x1b]52;c;aGVsbG8=\x07", "\x1b]52;c;!!\x07",
		"\x1bPqfoo\x1b\\", "\x1bP1+r544e=787465726d\x1b\\", "\x1bP0+r\x1b\\",
		"\x1b_Gi=1;OK\x1b\\", "\x1b_foo\x1b\\", "\x1b^foo\x1b\\", "\x1bXfoo\x1b\\",
		"\x1b]0;x\x18", "\x1b]0;x\x1a", "\x1b_x\x18", "\x1b[8;24;80t", "\x1b[4;600;800t",
		"\x1b[6;16;8t", "\x1b[48;24;80;600;800t", "\x1b[11t", "\x1b[8;24t", "\x1b[?997;1n",
		"\x1b[?997;2n", "\x1b[?997;3n", "\x1b[>4;2m", "\x1b[>4m", "\x1b[>5;1m", "\x1b]7501;?\x07",
		"\x1b]7501;x\x07", "\x1b]0;title\x1bx", "\x1b]\x1bx", "\x1bX\x1bx", "\x1b^\x1b\\",
		"\x1bPq\x1bx", "\x1b[u", "\x1b[@", "\x1b[^", "\x1b[1;2;3;4;5;6;7;8;9;10;11;12;13;14;15;16;17;18;19;20;21;22;23;24;25;26;27;28;29;30;31;32;33;34A",
		"\x1b[\x1b[A", "\x1b[1\x01", "\x1b[0;0;97;1;0;1x", "\x1b[?1;2;3$p", "\x1bOt", "\x1b[1?2u",
		"\x9b?1u", "\x9e" + "foo\x9c", "\x98foo\x9c", "\x9fGa=1\x9c", "\x98\x1bx",
	} {
		c = append(c, one(s))
	}
	return c
}

func pastes() []chunks {
	return []chunks{
		one("\x1b[200~hello\x1b[201~"),
		one("\x1b[200~a\r\nb\tc\x1b[201~"),
		one("\x1b[200~é🦀 日本\x1b[201~"),
		one("\x1b[200~x\x1b[Ay\x1b[201~"),
		one("\x1b[200~\x1b[201~"),
		one("\x1b[200~a\xffb�c\x1b[201~"),
		one("\x1b[200~\x1bx\x1b\x1b[201~"),
		one("\x1b[200~\x01\x7f \x1b[201~"),
		one("\x1b[200~abc"),
		one("\x1b[200~ab\x1b[1;5Ac\x1b[201~d"),
		one("\x1b[200~A\x1b]11;rgb:0/0/0\x07B\x1b[201~"),
		split("\x1b[200~hel", "lo wor", "ld\x1b[201~"),
		split("\x1b[200~ab", "\x1b", "[201~"),
		split("\x1b[200~ab\x1b", "c\x1b[201~"),
		split("\x1b[200~a", "\x1b[A", "b\x1b[201~"),
		split("\x1b[200~", "\x1b[201~", "x"),
		split("\x1b[2", "00~x\x1b[20", "1~"),
		one("x\x1b[200~p\x1b[201~y"),
		one("\x1b[200~\x1b[200~p\x1b[201~"),
		one("\x1b[201~"),
	}
}

func mixed() []chunks {
	return []chunks{
		one("a\x1b[Ab"), one("\x1b[A\x1b[B"), one("\x1b\x1b[A"), split("\x1b", "a"),
		split("\x1b[", "A"), split("\x1b[1;", "5A"), split("\x1b", "[A"), split("é"[:1], "é"[1:]),
		split("\x1b]11;rgb:0/0/0", "\x07"), split("\x1bP>|xt", "erm\x1b\\"), one("\x1b[1;5R"),
		one("\x1b[1;5Rx"), one("x\x1b[1;5R"), one("\x1b[23$"), one("\x1b[1;2P"),
		one("\x1b[I\x1b[O"), one("\x1b[<0;5;5M\x1b[<0;5;5m"), one("q\x1b"), split("q\x1b", ""),
		one("\x1b[3~\x1b[3;2~\x1b[3^"), one("\x1bOAx"), one("\x1b[200~"), one("\x1b[3$x"),
		one(strings.Repeat("\x1b", 7)), one("\x1b[M" + "\x20\x21"), split("\x1b[M ", "!!"),
		one("\x1b[5M!!!"), one("\x9bM!!!x"), one("\x1b[?1;2c\x1b[?62c"), one("\x1b[0u"),
		one("ab\x1b"), one("\x1b[27;5;97~x"), one("\x1b[97;5u\x1b[97;5:3u"),
	}
}
