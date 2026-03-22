# Touch Mode

wlr-which-key supports a touch mode that renders menu entries as large, tappable buttons instead of keyboard hints. Activate it with the `--touch` flag:

```sh
wlr-which-key --touch
wlr-which-key --touch my-config
```

Touch mode uses the same config file as normal mode. All theming options (colors, font, padding, border, etc.) apply to both modes. Pointer clicks and touchscreen taps are supported in both modes, but `--touch` changes the visual rendering to large buttons showing only the description text.

## Layout Decision Flowchart

When a page is rendered, the layout engine follows this order of operations:

```
For each entry in the menu:
 |
 |-- Is it a `row:` entry?
 |     |
 |     |-- Is touch mode ON, or `use_touch_layout: true`?
 |     |     YES --> Create an explicit section with the row's column groups
 |     |     NO  --> Flatten the row's entries into the pending item list
 |     |             (row structure is ignored, entries treated as regular items)
 |
 |-- Regular entry (cmd/submenu/submenu_file)?
 |     --> Add to pending item list
 |
 After all entries processed (or before a row section is created):
 |
 |-- Pending items exist?
       |
       |-- Is `rows_per_column` set?
       |     YES --> Distribute items into columns using rows_per_column
       |
       |-- Is touch mode ON and `rows_per_column` NOT set?
       |     YES --> Auto-grid: compute optimal columns for target aspect ratio
       |             (controlled by `touch_grid_ratio`, default 16:9)
       |
       |-- Otherwise?
             --> Single column (all items stacked vertically)
```

Summary of precedence (highest to lowest):

1. **`row:` entries** -- full manual control over sections and column groups
2. **`rows_per_column`** -- explicit column distribution for non-row items
3. **Auto-grid** -- automatic 16:9 grid (touch mode only, when nothing above is set)
4. **Single column** -- default fallback

## Basic Usage

With a standard config, touch mode renders each entry as a button. With auto-grid (the default), items are arranged to approximate a 16:9 aspect ratio:

```yaml
font: JetBrainsMono Nerd Font 14
background: "#282828d0"
color: "#fbf1c7"
border: "#8ec07c"

menu:
  - key: "s"
    desc: Sleep
    cmd: systemctl suspend
  - key: "r"
    desc: Reboot
    cmd: reboot
  - key: "o"
    desc: Off
    cmd: poweroff
```

With 3 items, auto-grid produces a single row of 3 buttons (closest to 16:9).

## Auto-Grid

In touch mode, when no `rows_per_column` or `row:` entries are used, items are automatically arranged in a grid targeting a 16:9 aspect ratio. The algorithm considers button dimensions and gaps to find the column count whose pixel aspect ratio is closest to the target.

Configure the target ratio with `touch_grid_ratio` (default `1.778`, i.e. 16/9):

```yaml
# Wider layout
touch_grid_ratio: 2.0

# Squarer layout
touch_grid_ratio: 1.0
```

To disable auto-grid, set `rows_per_column` explicitly. For example, `rows_per_column: 100` forces a single column.

Auto-grid is computed per-page, so each submenu gets its own optimal layout based on its item count and button dimensions.

## Multi-Column Layout

Use `rows_per_column` to create a grid with explicit control. Items flow top-to-bottom, then into the next column:

```yaml
rows_per_column: 3

menu:
  - key: "1"
    desc: Option A
    cmd: cmd-a
  - key: "2"
    desc: Option B
    cmd: cmd-b
  - key: "3"
    desc: Option C
    cmd: cmd-c
  - key: "4"
    desc: Option D
    cmd: cmd-d
  - key: "5"
    desc: Option E
    cmd: cmd-e
  - key: "6"
    desc: Option F
    cmd: cmd-f
```

With `rows_per_column: 3`, this produces a 3x2 grid:

```
┌──────────┬──────────┐
│ Option A │ Option D │
│ Option B │ Option E │
│ Option C │ Option F │
└──────────┴──────────┘
```

Setting `rows_per_column` disables auto-grid for that page.

## Variable Column Heights

`rows_per_column` can be a list to specify different heights per column:

```yaml
rows_per_column: [2, 3, 2]

menu:
  # Column 1 (2 items)
  - key: "j"
    desc: Vol -
    cmd: vol-down
  - key: "k"
    desc: Vol +
    cmd: vol-up
  # Column 2 (3 items)
  - key: "b"
    desc: Prev
    cmd: playerctl previous
  - key: "p"
    desc: Play/Pause
    cmd: playerctl play-pause
  - key: "n"
    desc: Next
    cmd: playerctl next
  # Column 3 (2 items)
  - key: "m"
    desc: Mute
    cmd: amixer set Master toggle
  - key: "e"
    desc: EQ
    cmd: easyeffects
```

```
┌──────────┬────────────┬──────┐
│ Vol -    │ Prev       │ Mute │
│ Vol +    │ Play/Pause │ EQ   │
│          │ Next       │      │
└──────────┴────────────┴──────┘
```

## Grid Layout with Rows

For complex layouts where you need both horizontal and vertical grouping, use `row:` entries. A `row` contains a list of column groups, and each column group is a list of entries:

```yaml
menu:
  - row:
    - - key: "j"
        desc: Vol -
        cmd: vol-down
      - key: "k"
        desc: Vol +
        cmd: vol-up
    - - key: "m"
        desc: Mute
        cmd: amixer set Master toggle
      - key: "e"
        desc: EQ
        cmd: easyeffects

  - row:
    - - key: "b"
        desc: Prev
        cmd: playerctl previous
    - - key: "p"
        desc: Play/Pause
        cmd: playerctl play-pause
    - - key: "n"
        desc: Next
        cmd: playerctl next
```

This produces:

```
┌──────────┬──────────┐
│ Vol -    │ Mute     │
│ Vol +    │ EQ       │
├──────┬───┴──┬───────┤
│ Prev │ Play │ Next  │
└──────┴──────┴───────┘
```

Each `row:` defines a horizontal section. Within a row, each `- -` (nested list) is a vertical column group. The row's height is determined by its tallest column group. Columns within a row share equal width.

### Row Behavior in Normal vs Touch Mode

By default, `row:` entries are only respected in touch mode. In normal mode (no `--touch`), row structure is flattened and entries are treated as a regular list.

To make normal mode also respect `row:` layout, set:

```yaml
use_touch_layout: true
```

### Empty Column Spacers

Use empty lists `[]` as column groups to create gaps. This is useful for cross or diamond layouts:

```yaml
menu:
  - key: "l"
    desc: Layout
    submenu:
      - row:
        - []
        - - key: j
            desc: top
            cmd: set-top
        - []
      - row:
        - - key: h
            desc: left
            cmd: set-left
        - - key: k
            desc: center
            cmd: set-center
        - - key: l
            desc: right
            cmd: set-right
      - row:
        - []
        - - key: ";"
            desc: bottom
            cmd: set-bottom
        - []
```

This produces a cross pattern:

```
         ┌───────┐
         │  top  │
┌──────┬─┴─────┬─┴──────┐
│ left │center │ right  │
└──────┴─┬─────┴─┬──────┘
         │bottom │
         └───────┘
```

All three rows have 3 columns, so the buttons align. Empty columns take up space but render nothing.

### Mixing Rows with Regular Entries

`row:` entries can be mixed with regular entries. Regular entries are collected and laid out separately (using `rows_per_column`, auto-grid, or single column):

```yaml
menu:
  # Grid section
  - row:
    - - key: "j"
        desc: Vol -
        cmd: vol-down
      - key: "k"
        desc: Vol +
        cmd: vol-up
    - - key: "m"
        desc: Mute
        cmd: amixer set Master toggle

  # Regular entry (auto-laid-out separately)
  - key: "q"
    desc: Quit
    cmd: quit
```

### Submenus Inside Rows

Submenus and `submenu_file` work inside row groups:

```yaml
menu:
  - row:
    - - key: "v"
        desc: Volume
        submenu:
          - key: "j"
            desc: Down
            cmd: vol-down
          - key: "k"
            desc: Up
            cmd: vol-up
    - - key: "m"
        desc: Media
        submenu_file: media
    - - key: "p"
        desc: Power
        submenu_file: power
```

## Navigation

In touch mode, a navigation button is always shown at the bottom of the menu:

- On a submenu page: shows **← Back** (returns to parent, same as Backspace in normal mode)
- On the root page: shows **✕ Close** (exits the menu, same as Escape in normal mode)

Keyboard shortcuts (Escape, Backspace, Ctrl+[, Ctrl+g) continue to work alongside touch navigation.

## Theming

Touch mode uses the same theming options as normal mode, plus button-specific options.

### Window Theming

These options apply to the overall menu window in both modes:

| Option            | Effect                                        |
|-------------------|-----------------------------------------------|
| `background`      | Menu window background                        |
| `color`           | Separator/default text color                  |
| `border`          | Window border color                           |
| `font`            | Text font                                     |
| `border_width`    | Window border thickness                       |
| `corner_r`        | Window corner radius                          |
| `padding`         | Space between window edge and content         |

### Button Theming

These options control the appearance of buttons in touch mode:

| Option                | Default              | Effect                                    |
|-----------------------|----------------------|-------------------------------------------|
| `button_color`        | value of `border`    | Button fill color                         |
| `button_text_color`   | value of `desc_color`| Button label text color                   |
| `button_border_color` | none (no outline)    | Stroke color around button edge           |
| `button_border_width` | `0`                  | Stroke width around button edge (0 = off) |
| `button_corner_r`     | `8`                  | Button corner radius                      |
| `button_padding`      | `24`                 | Horizontal padding inside button          |
| `button_padding_v`    | `16`                 | Vertical padding inside button            |
| `button_width`        | auto (from text)     | Explicit button width                     |
| `button_height`       | auto (from font)     | Explicit button height                    |
| `button_row_gap`      | `8`                  | Vertical gap between button rows          |
| `button_column_gap`   | value of `button_row_gap` | Horizontal gap between button columns |
| `button_overflow`     | `fit`                | How to handle text wider than `button_width` (see below) |

### Button Overflow

When `button_width` is set, `button_overflow` controls what happens if the text is wider than the button:

| Value       | Behavior |
|-------------|----------|
| `fit`       | `button_width` acts as a minimum -- buttons expand to fit text (default) |
| `ellipsize` | `button_width` is strict -- text is truncated with `...` |

```yaml
button_width: 200
button_overflow: ellipsize  # Long text becomes "Some long tex..."
```

Without `button_width`, `button_overflow` has no effect (buttons always auto-size to fit).

### Button Gaps vs Normal Mode Gaps

Touch mode uses `button_row_gap` and `button_column_gap` instead of `row_padding` and `column_padding`. This lets you set different spacing for touch and normal modes in the same config:

```yaml
# Normal mode gaps
row_padding: 0
column_padding: 20

# Touch mode gaps
button_row_gap: 8
button_column_gap: 12
```

Example with custom button styling:

```yaml
font: JetBrainsMono Nerd Font 14
background: "#1d2021e0"
color: "#ebdbb2"
border: "#504945"

# Button appearance
button_color: "#3c3836"
button_text_color: "#ebdbb2"
button_border_color: "#665c54"
button_border_width: 1
button_corner_r: 12
button_padding: 32
button_padding_v: 20
button_height: 60

# Spacing
button_row_gap: 6
button_column_gap: 6
padding: 12

menu:
  - key: "s"
    desc: Sleep
    cmd: systemctl suspend
  - key: "r"
    desc: Reboot
    cmd: reboot
  - key: "o"
    desc: Off
    cmd: poweroff
```

### Per-Submenu Overrides

Button theming, layout, and auto-grid options can be overridden per submenu via `submenu_file`:

```yaml
# ~/.config/wlr-which-key/media.yaml
button_color: "#458588"
button_text_color: "#ebdbb2"
button_corner_r: 20
button_width: 300
touch_grid_ratio: 1.0
menu:
  - key: "b"
    desc: Prev
    cmd: playerctl previous
  - key: "p"
    desc: Play/Pause
    cmd: playerctl play-pause
  - key: "n"
    desc: Next
    cmd: playerctl next
```

Theme overrides cascade from parent to child submenus. If a parent page sets `button_width: 300`, child submenus inherit it unless they override it themselves.

Or as a plain list (inherits all theming from parent):

```yaml
# ~/.config/wlr-which-key/power.yaml
- key: "s"
  desc: Sleep
  cmd: systemctl suspend
- key: "r"
  desc: Reboot
  cmd: reboot
```

## Pointer/Click Support

Pointer clicks and touch taps work in **both** normal and touch mode. Clicking or tapping a menu item activates it just like pressing its key. In normal mode the visual rendering stays the same (key hints with separator and description), but items are clickable.
