// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
// The original DockControl selector's exact comparer expression; no replacement UI/engine.
(string Left, string Right)[] pairs = [
    ("a", "A"), ("a", "["), ("z", "["), ("Document 2", "document 10"),
    ("é", "É"), ("ß", "SS"), ("ı", "I"), ("ſ", "S"), ("K", "K"),
    ("\U00010428", "\U00010400"), ("\U00010428", "\uE000"),
    ("\U0001F600", "\uFFFF"), ("\uE000", "\U00010400"),
    ("\0a", "\0B"), ("a\0", "A"), ("a", "aa"), ("", "a"), ("", ""),
    ("Σ", "ς"), ("İ", "i"), ("é", "e\u0301")
];
Console.WriteLine("left_utf8_hex\tright_utf8_hex\tcomparison_sign");
foreach (var (left,right) in pairs)
    Console.WriteLine($"{Convert.ToHexString(System.Text.Encoding.UTF8.GetBytes(left))}\t{Convert.ToHexString(System.Text.Encoding.UTF8.GetBytes(right))}\t{Math.Sign(StringComparer.OrdinalIgnoreCase.Compare(left,right))}");
