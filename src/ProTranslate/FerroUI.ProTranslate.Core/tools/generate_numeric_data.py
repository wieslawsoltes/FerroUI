#!/usr/bin/env python3
"""Regenerate independent numeric-format expectations with the source runtime.

Usage: python3 tools/generate_numeric_data.py /path/to/dotnet test_data/numeric-runtime.tsv
Requires the Microsoft .NET 10 SDK only when regenerating; Rust tests use the
checked-in results and have no .NET runtime or network dependency.
"""
import itertools
import json
import pathlib
import subprocess
import sys
import tempfile

dotnet, output = sys.argv[1:]
values = ["0", "-0", "12.5", "-12.5", "0.125", "-0.0001", "1234.56789",
          "1234567890", "0.00000125", "999.95", "2.675", "1.225", "1.2345678901234567"]
formats = ["G", "R2", "G3", "F2", "N0", "E2", "P1", "0.00", "0.00;(0.00)",
           "0.00;(0.00);zero", "0.00;;zero", "00-00", "(###) ###-####", "#.##",
           "#,##0,,.00", "0.0%%", "0E+00", "#0.0e0", "0.0##e+00", ".00E-0",
           "0E0 E00", "0.0,0", "0.0.0", "0.00,", "'a\\b'0", "0,foo,.00",
           "#-#", "0#0", "0.0#0", "##0.###", "0,000.00", "0'", "0\\",
           "0;neg;zero;ignored", "0.00%\\", "'unclosed"]
cases = list(itertools.product(["double", "decimal", "float"], values, formats))
source = r'''
using System;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Text.Json;
Console.WriteLine(RuntimeInformation.FrameworkDescription);
string? line;
while ((line = Console.ReadLine()) != null) {
    var fields = JsonSerializer.Deserialize<string[]>(line)!;
    IFormattable value = fields[0] switch {
        "double" => double.Parse(fields[1], CultureInfo.InvariantCulture),
        "float" => float.Parse(fields[1], CultureInfo.InvariantCulture),
        "decimal" => decimal.Parse(fields[1], CultureInfo.InvariantCulture),
        _ => throw new ArgumentException("Unknown numeric type")
    };
    Console.WriteLine(JsonSerializer.Serialize(value.ToString(fields[2], CultureInfo.InvariantCulture)));
}
'''
with tempfile.TemporaryDirectory(prefix="protranslate-numeric-oracle-") as directory:
    project = pathlib.Path(directory)
    (project / "Oracle.csproj").write_text(
        '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType>'
        '<TargetFramework>net10.0</TargetFramework><Nullable>enable</Nullable>'
        '</PropertyGroup></Project>\n')
    (project / "Program.cs").write_text(source)
    subprocess.run([dotnet, "build", str(project / "Oracle.csproj"), "--nologo", "--verbosity", "quiet"], check=True)
    result = subprocess.run([dotnet, str(project / "bin/Debug/net10.0/Oracle.dll")],
                            input="".join(json.dumps(case) + "\n" for case in cases),
                            text=True, capture_output=True, check=True)
lines = result.stdout.splitlines()
assert len(lines) == len(cases) + 1
output_lines = [f"# {lines[0]}; invariant culture; type, value, format, =output."]
for case, result in zip(cases, lines[1:]):
    output_lines.append("\t".join([*case, "=" + json.loads(result)]))
pathlib.Path(output).write_text("\n".join(output_lines) + "\n")
print(f"Generated {len(cases)} numeric runtime expectations")
