# jskel

Extract JSON structure by stripping values.

## Why

Sometimes the shape matters and the data does not.

- Share an API payload's structure without leaking real data.
- Generate a quick mock for frontend/backend alignment.
- Diff two payload shapes by eye.
- Document a contract in a minimal, readable way.

## Install

```bash
cargo install --path .
```

Or run directly from a release build:

```bash
cargo build --release
./target/release/jskel '{"x":1}'
```

## Usage

```bash
jskel '{"name":"Brendan","age":40,"active":true}'
jskel -c '{"x":1,"y":[1,2]}'           # copy result to clipboard
cat payload.json | jskel                # read from stdin
jskel payload.json                      # read from file
jskel -                                 # force stdin
```

## Example

Input:

```json
{
  "id": 42,
  "name": "Brendan",
  "active": true,
  "tags": ["admin", "ops"],
  "meta": null
}
```

Output:

```json
{
  "id": 0,
  "name": "",
  "active": false,
  "tags": [
    "",
    ""
  ],
  "meta": null
}
```

## Rules

| Input    | Output         |
| -------- | -------------- |
| string   | `""`           |
| number   | `0`            |
| boolean  | `false`        |
| null     | `null`         |
| array    | recurse        |
| object   | recurse        |

Object key order is preserved.

## Flags

```
-c, --copy             Copy result to the system clipboard
-m, --compact          Output compact JSON (no whitespace)
    --indent <N>       Indent with N spaces (default: 2)
-s, --sort-keys        Sort object keys alphabetically

    --nulls            Replace every scalar with null
    --types            Replace scalars with their type name as a string
    --preserve-bool    Keep booleans as-is; strip other scalars

    --pick <KEYS>      Comma-separated keys to keep at the top level
    --omit <KEYS>      Comma-separated keys to drop at the top level

    --no-color         Disable ANSI color in terminal output
-h, --help             Show help
-V, --version          Show version
```

`--pick` and `--omit` apply to the top-level object. For an array of objects,
they apply to each item. They do not recurse into nested objects.

## Strategies

Default:

```bash
$ jskel '{"a":1,"b":"x","c":true}'
{
  "a": 0,
  "b": "",
  "c": false
}
```

`--types` (useful for documenting a contract):

```bash
$ jskel --types '{"a":1,"b":"x","c":true,"d":null}'
{
  "a": "number",
  "b": "string",
  "c": "boolean",
  "d": "null"
}
```

`--nulls`:

```bash
$ jskel --nulls '{"a":1,"b":"x"}'
{
  "a": null,
  "b": null
}
```

`--preserve-bool`:

```bash
$ jskel --preserve-bool '{"flag":true,"x":42}'
{
  "flag": true,
  "x": 0
}
```

## Clipboard

`-c` / `--copy` writes the result to the system clipboard.

- macOS: `pbcopy`
- Linux: `wl-copy` (Wayland), `xclip`, or `xsel`
- Windows: `clip`

If the underlying tool isn't installed, `jskel` reports it and exits non-zero.
The clipboard always receives uncolored JSON, even when stdout is colored.

## Philosophy

- Simple. One job, done deterministically.
- Predictable. Same input, same output. Object key order preserved.
- Composable. Reads stdin, writes stdout, exits non-zero on error.
- No third-party dependencies. Just the Rust standard library.

## License

MIT
