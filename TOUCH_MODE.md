# Touch Mode

wlr-which-key supports a touch mode that renders menu entries as large, tappable buttons instead of keyboard hints. Activate it with the `--touch` flag:

```sh
wlr-which-key --touch
wlr-which-key --touch my-config
```

Touch mode uses the same config file as normal mode. All theming options (colors, font, padding, border, etc.) apply to both modes. Pointer clicks and touchscreen taps are supported in both modes, but `--touch` changes the visual rendering to large buttons showing only the description text.

## Basic Usage

With a standard config, touch mode renders each entry as a full-width button labeled with its `desc` text:

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

This produces three vertically stacked buttons: **Sleep**, **Reboot**, **Off**.

## Multi-Column Layout

Use `rows_per_column` to create a grid. Items flow top-to-bottom, then into the next column:

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

Rows stack vertically with `row_padding` between them. `column_padding` controls the horizontal gap between columns within a row.

### Mixing Rows with Regular Entries

`row:` entries can be mixed with regular entries. Regular entries without `row:` behave as they do today (auto-layout with `rows_per_column`, or single-column if not set):

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

  # Regular entry
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
| `row_padding`     | Vertical gap between rows/buttons             |
| `column_padding`  | Horizontal gap between columns                |

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
| `button_height`       | auto (from font)     | Explicit button height override           |
| `button_gap`          | `8`                  | Minimum gap between buttons (overridden by `row_padding` if larger) |

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
row_padding: 6
column_padding: 6
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

### Per-Submenu Button Overrides

Button theming options can be overridden in `submenu_file` files alongside other theme overrides:

```yaml
# ~/.config/wlr-which-key/media.yaml
button_color: "#458588"
button_text_color: "#ebdbb2"
button_corner_r: 20
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

### External Submenu Theming

Submenu files loaded via `submenu_file` can override theme values for their page:

```yaml
# ~/.config/wlr-which-key/media.yaml
desc_color: "#d3869b"
border: "#458588"
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
